use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use super::{HackrfTxSink, TxSink, test_support::*};
use crate::Error;

fn expect_sink(
    result: Result<HackrfTxSink, Error>, name: &str,
) -> HackrfTxSink {
    match result {
        Ok(sink) => sink,
        Err(error) => panic!("{name} construction failed: {error}"),
    }
}

fn expect_error(result: Result<(), Error>, name: &str) -> Error {
    match result {
        Ok(()) => panic!("{name} unexpectedly succeeded"),
        Err(error) => error,
    }
}

#[test]
fn preparation_failures_remain_off() {
    let failure_points = [
        FailurePoint::Open,
        FailurePoint::SetFreq,
        FailurePoint::SetSampleRate,
        FailurePoint::SetAmp,
        FailurePoint::SetGain,
        FailurePoint::PrepareWriter,
    ];

    for failure in failure_points {
        let scenario = Scenario {
            failure: Some(failure),
            ..Scenario::default()
        };
        let (result, events, _) = build_sink(valid_config(), 4, scenario);
        assert!(result.is_err(), "{failure:?} unexpectedly succeeded");
        let events = events.snapshot();
        assert!(
            !events.contains(&Event::Activate),
            "{failure:?}: {events:?}"
        );
        assert!(!events.contains(&Event::Stop), "{failure:?}: {events:?}");
    }
}

#[test]
fn writer_spawn_failure_remains_off() {
    let scenario = Scenario {
        spawn: SpawnBehavior::Fail,
        ..Scenario::default()
    };
    let (result, events, _) = build_sink(valid_config(), 4, scenario);
    let Err(error) = result else {
        panic!("writer spawn unexpectedly succeeded");
    };
    assert!(error.to_string().contains("synthetic writer spawn failure"));
    let events = events.snapshot();
    assert!(events.contains(&Event::PrepareWriter));
    assert!(events.contains(&Event::SpawnWriter));
    assert!(!events.contains(&Event::Activate));
    assert!(!events.contains(&Event::Stop));
}

#[test]
fn prefill_completes_before_activation_and_first_write() {
    let (result, events, _) =
        build_sink(valid_config(), 4, Scenario::default());
    let mut sink = expect_sink(result, "prefill");
    events.clear();

    assert!(sink.write_block_i16(&iq_block(1)).is_ok());
    assert_eq!(events.snapshot(), vec![Event::Buffer(1)]);
    assert!(sink.write_block_i16(&iq_block(2)).is_ok());
    assert!(sink.finish().is_ok());
    assert!(sink.finish().is_ok());
    drop(sink);

    assert_eq!(events.snapshot(), vec![
        Event::Buffer(1),
        Event::Buffer(2),
        Event::Activate,
        Event::Write(1),
        Event::Write(2),
        Event::Flush,
        Event::Stop,
    ]);
}

#[test]
fn zero_prefill_activates_on_first_ready_block() {
    let mut config = valid_config();
    config.prefill_blocks = 0;
    let (result, events, _) = build_sink(config, 4, Scenario::default());
    let mut sink = expect_sink(result, "zero prefill");
    events.clear();

    assert!(sink.write_block_i16(&iq_block(1)).is_ok());
    assert!(sink.finish().is_ok());
    assert_eq!(events.snapshot(), vec![
        Event::Buffer(1),
        Event::Activate,
        Event::Write(1),
        Event::Flush,
        Event::Stop,
    ]);
}

#[test]
fn partial_prefill_is_drained_during_finish() {
    let (result, events, _) =
        build_sink(valid_config(), 4, Scenario::default());
    let mut sink = expect_sink(result, "partial prefill");
    events.clear();

    assert!(sink.write_block_i16(&iq_block(1)).is_ok());
    assert!(sink.finish().is_ok());
    assert_eq!(events.snapshot(), vec![
        Event::Buffer(1),
        Event::Activate,
        Event::Write(1),
        Event::Flush,
        Event::Stop,
    ]);
}

#[test]
fn empty_stream_never_activates_or_counts_startup_underrun() {
    let counter = Arc::new(AtomicU64::new(0));
    let mut config = valid_config();
    config.underrun_counter = Some(counter.clone());
    let (result, events, _) = build_sink(config, 4, Scenario::default());
    let mut sink = expect_sink(result, "empty stream");
    events.clear();

    assert!(sink.finish().is_ok());
    drop(sink);
    assert_eq!(counter.load(Ordering::Relaxed), 0);
    assert_eq!(events.snapshot(), vec![Event::Flush]);
}

#[test]
fn activation_failure_is_primary_and_rollback_warning_has_context() {
    let scenario = Scenario {
        failure: Some(FailurePoint::Activate),
        stop_failures: 1,
        ..Scenario::default()
    };
    let ((write_error, finish_result, events), output) =
        capture_tracing(|| {
            let mut config = valid_config();
            config.prefill_blocks = 0;
            let (result, events, _) = build_sink(config, 4, scenario);
            let mut sink = expect_sink(result, "activation failure");
            events.clear();
            let error = expect_error(
                sink.write_block_i16(&iq_block(1)),
                "activation failure",
            );
            let finish_result = sink.finish();
            drop(sink);
            (error, finish_result, events)
        });

    assert!(
        write_error
            .to_string()
            .contains("synthetic activate failure")
    );
    assert!(finish_result.is_ok());
    let events = events.snapshot();
    assert_eq!(
        events
            .iter()
            .filter(|event| **event == Event::Activate)
            .count(),
        1
    );
    assert_eq!(
        events.iter().filter(|event| **event == Event::Stop).count(),
        2
    );
    assert!(!events.iter().any(|event| matches!(event, Event::Write(_))));
    assert!(
        output.contains("failed to return hackrf to Off"),
        "{output}"
    );
    assert!(
        output.contains("synthetic rollback stop failure"),
        "{output}"
    );
    assert!(output.contains("rf_freq_hz=1575420000"), "{output}");
}

#[test]
fn partial_prefill_activation_failure_is_never_retried() {
    let scenario = Scenario {
        failure: Some(FailurePoint::Activate),
        ..Scenario::default()
    };
    let (result, events, _) = build_sink(valid_config(), 4, scenario);
    let mut sink = expect_sink(result, "partial activation failure");
    events.clear();

    assert!(sink.write_block_i16(&iq_block(1)).is_ok());
    let error = expect_error(sink.finish(), "partial activation failure");
    assert!(error.to_string().contains("synthetic activate failure"));
    assert!(sink.finish().is_ok());
    drop(sink);

    let events = events.snapshot();
    assert_eq!(
        events
            .iter()
            .filter(|event| **event == Event::Activate)
            .count(),
        1
    );
    assert_eq!(events, vec![Event::Buffer(1), Event::Activate, Event::Stop]);
}

#[test]
fn prepared_write_failure_discards_prefill_and_never_activates() {
    let (result, events, _) =
        build_sink(valid_config(), 4, Scenario::default());
    let mut sink = expect_sink(result, "prepared write failure");
    events.clear();

    assert!(sink.write_block_i16(&iq_block(1)).is_ok());
    let error = expect_error(
        sink.write_block_i16(&[1, 2, 3]),
        "prepared write failure",
    );
    assert!(error.to_string().contains("IQ block length invalid"));
    assert!(sink.finish().is_ok());
    drop(sink);

    assert_eq!(events.snapshot(), vec![Event::Buffer(1)]);
}

#[test]
fn initial_dispatch_failure_is_terminal_and_discards_prefill() {
    let scenario = Scenario {
        spawn: SpawnBehavior::DropReceiver,
        ..Scenario::default()
    };
    let (result, events, _) = build_sink(valid_config(), 4, scenario);
    let mut sink = expect_sink(result, "initial dispatch failure");
    events.clear();

    assert!(sink.write_block_i16(&iq_block(1)).is_ok());
    let error = expect_error(
        sink.write_block_i16(&iq_block(2)),
        "initial dispatch failure",
    );
    assert!(error.to_string().contains("sending on a closed channel"));
    assert!(sink.finish().is_ok());
    drop(sink);

    assert_eq!(events.snapshot(), vec![
        Event::Buffer(1),
        Event::Buffer(2),
        Event::Activate,
        Event::Stop,
    ]);
}

#[test]
fn later_send_failure_is_terminal_and_never_reactivates() {
    let mut config = valid_config();
    config.prefill_blocks = 0;
    let scenario = Scenario {
        spawn: SpawnBehavior::DropAfterOne,
        ..Scenario::default()
    };
    let (result, events, closed) = build_sink(config, 4, scenario);
    let mut sink = expect_sink(result, "later send failure");
    let Some(closed) = closed else {
        panic!("missing receiver-close notification");
    };
    events.clear();

    assert!(sink.write_block_i16(&iq_block(1)).is_ok());
    assert!(closed.recv_timeout(Duration::from_secs(5)).is_ok());
    let error =
        expect_error(sink.write_block_i16(&iq_block(2)), "later send failure");
    assert!(error.to_string().contains("sending on a closed channel"));
    assert!(sink.finish().is_ok());
    drop(sink);

    let events = events.snapshot();
    assert_eq!(
        events
            .iter()
            .filter(|event| **event == Event::Activate)
            .count(),
        1
    );
    assert_eq!(events, vec![
        Event::Buffer(1),
        Event::Activate,
        Event::Write(1),
        Event::Stop
    ]);
}

#[test]
fn writer_write_failure_stops_tx_and_remains_primary() {
    let mut config = valid_config();
    config.prefill_blocks = 0;
    let scenario = Scenario {
        writer: WriterBehavior::FailWrite,
        stop_failures: 1,
        ..Scenario::default()
    };
    let (result, events, _) = build_sink(config, 4, scenario);
    let mut sink = expect_sink(result, "writer failure");
    events.clear();

    assert!(sink.write_block_i16(&iq_block(1)).is_ok());
    let error = expect_error(sink.finish(), "writer failure");
    assert!(error.to_string().contains("synthetic writer write failure"));
    drop(sink);
    assert_eq!(events.snapshot(), vec![
        Event::Buffer(1),
        Event::Activate,
        Event::Write(1),
        Event::Stop,
        Event::Stop,
    ]);
}

#[test]
fn writer_flush_failure_stops_tx_and_remains_primary() {
    let mut config = valid_config();
    config.prefill_blocks = 0;
    let scenario = Scenario {
        writer: WriterBehavior::FailFlush,
        ..Scenario::default()
    };
    let (result, events, _) = build_sink(config, 4, scenario);
    let mut sink = expect_sink(result, "flush failure");
    events.clear();

    assert!(sink.write_block_i16(&iq_block(1)).is_ok());
    let error = expect_error(sink.finish(), "flush failure");
    assert!(error.to_string().contains("synthetic writer flush failure"));
    assert_eq!(events.snapshot(), vec![
        Event::Buffer(1),
        Event::Activate,
        Event::Write(1),
        Event::Flush,
        Event::Stop,
    ]);
}

#[test]
fn standalone_stop_failure_is_returned() {
    let mut config = valid_config();
    config.prefill_blocks = 0;
    let scenario = Scenario {
        stop_failures: 1,
        ..Scenario::default()
    };
    let (result, events, _) = build_sink(config, 4, scenario);
    let mut sink = expect_sink(result, "stop failure");
    events.clear();

    assert!(sink.write_block_i16(&iq_block(1)).is_ok());
    let error = expect_error(sink.finish(), "stop failure");
    assert!(
        error
            .to_string()
            .contains("synthetic rollback stop failure")
    );
    drop(sink);

    let events = events.snapshot();
    assert_eq!(
        events.iter().filter(|event| **event == Event::Stop).count(),
        2
    );
    assert_eq!(events[..4], [
        Event::Buffer(1),
        Event::Activate,
        Event::Write(1),
        Event::Flush
    ]);
}

#[cfg(panic = "unwind")]
#[test]
fn armed_guard_rolls_back_on_same_thread_unwind() {
    let events = EventLog::default();
    let outcome = catch_unwind(AssertUnwindSafe({
        let events = events.clone();
        move || {
            let mut guard = fake_activation_guard(
                &valid_config(),
                Scenario::default(),
                events,
            );
            assert!(guard.activate().is_ok());
            std::panic::panic_any(String::from("armed guard panic"));
        }
    }));

    let payload = match outcome {
        Ok(()) => panic!("guard panic did not unwind"),
        Err(payload) => payload,
    };
    assert_eq!(
        payload.downcast_ref::<String>().map(String::as_str),
        Some("armed guard panic")
    );
    assert_eq!(events.snapshot(), vec![Event::Activate, Event::Stop]);
}

#[cfg(panic = "unwind")]
#[test]
fn activation_panic_in_sink_is_never_retried() {
    let mut config = valid_config();
    config.prefill_blocks = 0;
    let scenario = Scenario {
        failure: Some(FailurePoint::PanicActivate),
        ..Scenario::default()
    };
    let (result, events, _) = build_sink(config, 4, scenario);
    let mut sink = expect_sink(result, "activation panic");
    events.clear();

    let outcome = catch_unwind(AssertUnwindSafe(move || {
        let result = sink.write_block_i16(&iq_block(1));
        panic!("activation did not panic: {result:?}");
    }));

    let payload = match outcome {
        Ok(()) => panic!("activation panic did not unwind"),
        Err(payload) => payload,
    };
    let message = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied());
    assert_eq!(message, Some("synthetic activation panic"));
    assert_eq!(events.snapshot(), vec![
        Event::Buffer(1),
        Event::Activate,
        Event::Stop
    ]);
}

#[test]
fn writer_thread_panic_finalization_stops_tx() {
    let mut config = valid_config();
    config.prefill_blocks = 0;
    let scenario = Scenario {
        writer: WriterBehavior::PanicWrite,
        ..Scenario::default()
    };
    let (result, events, _) = build_sink(config, 4, scenario);
    let mut sink = expect_sink(result, "writer panic");
    events.clear();

    assert!(sink.write_block_i16(&iq_block(1)).is_ok());
    let error = expect_error(sink.finish(), "writer panic");
    assert!(error.to_string().contains("writer thread panicked"));
    assert_eq!(events.snapshot(), vec![
        Event::Buffer(1),
        Event::Activate,
        Event::Write(1),
        Event::Stop,
    ]);
}
