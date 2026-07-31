use std::{
    error::Error as StdError,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};

use super::{
    TxSink,
    root_error_test_support::*,
    shutdown::{
        WorkerObservation, WriterFinalResult, WriterResultMailbox,
        WriterTerminal,
    },
    shutdown_test_support::ReleaseGuard,
    test_support::{Event, iq_block},
};
use crate::{Error, error::resolve_run_and_finish, tx::TxTee};

fn take_error(result: Result<(), Error>, operation: &str) -> Error {
    match result {
        Ok(()) => panic!("{operation} unexpectedly succeeded"),
        Err(error) => error,
    }
}

fn wait_for(signal: &mpsc::Receiver<()>, name: &str) {
    assert!(
        signal.recv_timeout(Duration::from_secs(1)).is_ok(),
        "missing {name} signal"
    );
}

fn stop_count(events: &[Event]) -> usize {
    events.iter().filter(|event| **event == Event::Stop).count()
}

struct FailingFinishSink {
    backend: &'static str,
    message: &'static str,
}

impl TxSink for FailingFinishSink {
    fn backend(&self) -> &'static str {
        self.backend
    }

    fn write_block_i16(
        &mut self, _interleaved_iq_i16: &[i16],
    ) -> Result<(), Error> {
        Ok(())
    }

    fn finish(&mut self) -> Result<(), Error> {
        Err(Error::tx_backend_msg(self.backend, self.message))
    }
}

#[test]
fn writer_result_mailbox_preserves_terminal_ownership() {
    let (completed, completed_publisher) = WriterResultMailbox::new();
    completed_publisher.publish(WriterTerminal::Completed);
    assert!(completed.take_failure().is_none());
    assert!(matches!(
        completed.take_for_finalization(),
        WriterFinalResult::Terminal(WriterTerminal::Completed)
    ));

    let (cancelled, cancelled_publisher) = WriterResultMailbox::new();
    cancelled_publisher.publish(WriterTerminal::Cancelled);
    assert!(cancelled.take_failure().is_none());
    assert!(matches!(
        cancelled.take_for_finalization(),
        WriterFinalResult::Terminal(WriterTerminal::Cancelled)
    ));

    let (failed, failed_publisher) = WriterResultMailbox::new();
    failed_publisher.publish(WriterTerminal::Failed(writer_error()));
    let Some(error) = failed.take_failure() else {
        panic!("producer did not take available writer failure");
    };
    assert_writer_error(&error);
    assert!(matches!(
        failed.take_for_finalization(),
        WriterFinalResult::FailureConsumed
    ));

    let (missing, missing_publisher) = WriterResultMailbox::new();
    drop(missing_publisher);
    assert!(matches!(
        missing.take_for_finalization(),
        WriterFinalResult::Missing
    ));
}

#[test]
fn writer_result_mailbox_recovers_from_poison() {
    let (mailbox, publisher) = WriterResultMailbox::new();
    assert!(mailbox.poison_for_test());
    publisher.publish(WriterTerminal::Failed(writer_error()));
    let Some(error) = mailbox.take_failure() else {
        panic!("poisoned mailbox lost writer failure");
    };
    assert_writer_error(&error);
    assert!(matches!(
        mailbox.take_for_finalization(),
        WriterFinalResult::FailureConsumed
    ));
}

#[test]
fn asynchronous_writer_root_replaces_disconnected_channel_error() {
    let (mut sink, events, signals) =
        build_active_failure_sink(|| {}, RootSinkOptions::default());
    wait_for(&signals.disconnected, "writer disconnect");

    let error =
        take_error(sink.write_block_i16(&iq_block(1)), "producer write");
    let display = error.to_string();
    assert_writer_error(&error);
    assert!(!display.contains("closed channel"), "{display}");
    assert!(
        !display.contains("sending on a closed channel"),
        "{display}"
    );
    assert!(sink.finish().is_ok());
    let observations = signals.observations();
    assert!(observations.contains(&WorkerObservation::ResultObserved));
    assert!(observations.contains(&WorkerObservation::Joined));
    assert!(!observations.iter().any(|observation| matches!(
        observation,
        WorkerObservation::Detached { .. }
    )));

    let events = events.snapshot();
    assert_eq!(
        events
            .iter()
            .filter(|event| **event == Event::Activate)
            .count(),
        1
    );
    assert_eq!(stop_count(&events), 1);
}

#[test]
fn send_races_promote_root_after_enqueue_and_disconnect() {
    for (trigger, disconnect_before_release, operation) in [
        (
            WorkerObservation::Enqueued,
            false,
            "post-enqueue writer failure",
        ),
        (
            WorkerObservation::BeforeTrySend,
            true,
            "disconnected writer failure",
        ),
    ] {
        let (mut sink, signals) =
            build_racing_failure_sink(trigger, disconnect_before_release);
        let error = take_error(sink.write_block_i16(&iq_block(1)), operation);
        assert_writer_error(&error);
        assert!(signals.observations().contains(&trigger));
        assert!(sink.finish().is_ok());
    }
}

#[test]
fn consumed_writer_root_keeps_stop_failure_as_secondary() {
    let options = RootSinkOptions {
        stop_failures: 2,
        ..RootSinkOptions::default()
    };
    let (mut sink, events, signals) = build_active_failure_sink(|| {}, options);
    wait_for(&signals.disconnected, "writer disconnect");

    let run = sink.write_block_i16(&iq_block(1));
    let finish = sink.finish();
    let error = take_error(resolve_run_and_finish(run, finish), "combined run");
    assert_standard_source_is_writer(&error);
    assert_eq!(error.to_string().matches(WRITER_FAILURE_MESSAGE).count(), 1);
    let Error::RunAndFinalizationFailed {
        primary,
        finalization,
    } = error
    else {
        panic!("writer and stop failures were not composed");
    };
    assert_writer_error(&primary);
    assert!(finalization.to_string().contains("rollback stop failure"));
    assert_eq!(stop_count(&events.snapshot()), 2);
}

#[test]
fn consumed_writer_root_keeps_shutdown_deadline_as_secondary() {
    let (release_sender, release_receiver) = mpsc::channel();
    let mut release = ReleaseGuard::new(release_sender);
    let options = RootSinkOptions {
        policy: short_policy(),
        ..RootSinkOptions::default()
    };
    let (mut sink, _, signals) = build_active_failure_sink(
        move || {
            if release_receiver.recv().is_err() {
                tracing::debug!("deadline release owner dropped");
            }
        },
        options,
    );
    wait_for(&signals.published, "writer result publication");

    let run = sink.write_block_i16(&iq_block(1));
    let finish = sink.finish();
    let error = take_error(resolve_run_and_finish(run, finish), "deadline run");
    assert_standard_source_is_writer(&error);
    let Error::RunAndFinalizationFailed { finalization, .. } = error else {
        panic!("writer and deadline failures were not composed");
    };
    assert!(
        finalization
            .to_string()
            .contains("writer shutdown timed out")
    );
    let observations = signals.observations();
    assert!(observations.contains(&WorkerObservation::Detached {
        was_finished: false,
    }));
    assert!(!observations.contains(&WorkerObservation::Joined));
    release.release();
}

#[test]
fn consumed_root_retains_deadline_before_stop_failure() {
    let (release_sender, release_receiver) = mpsc::channel();
    let mut release = ReleaseGuard::new(release_sender);
    let options = RootSinkOptions {
        policy: short_policy(),
        stop_failures: 2,
        ..RootSinkOptions::default()
    };
    let (mut sink, events, signals) = build_active_failure_sink(
        move || {
            if release_receiver.recv().is_err() {
                tracing::debug!("deadline-stop release owner dropped");
            }
        },
        options,
    );
    wait_for(&signals.published, "writer result publication");

    let run = sink.write_block_i16(&iq_block(1));
    let finish = sink.finish();
    let error = take_error(
        resolve_run_and_finish(run, finish),
        "deadline and stop run",
    );
    assert_standard_source_is_writer(&error);
    assert_eq!(error.to_string().matches(WRITER_FAILURE_MESSAGE).count(), 1);
    let Error::RunAndFinalizationFailed { finalization, .. } = error else {
        panic!("writer cleanup failures were not composed");
    };
    let Error::MultipleFinalizationFailures { first, additional } =
        *finalization
    else {
        panic!("deadline and stop failures were not retained together");
    };
    assert!(first.to_string().contains("writer shutdown timed out"));
    assert_eq!(additional.len(), 1);
    assert!(additional[0].to_string().contains("rollback stop failure"));
    assert_eq!(stop_count(&events.snapshot()), 2);
    let observations = signals.observations();
    assert!(observations.contains(&WorkerObservation::Detached {
        was_finished: false,
    }));
    assert!(!observations.contains(&WorkerObservation::Joined));
    release.release();
}

#[cfg(panic = "unwind")]
#[test]
fn consumed_writer_root_keeps_later_writer_panic_as_secondary() {
    let (panic_sender, panic_receiver) = mpsc::channel();
    let mut panic_release = ReleaseGuard::new(panic_sender);
    let (mut sink, _, signals) = build_active_failure_sink(
        move || {
            assert!(
                panic_receiver.recv().is_err(),
                "synthetic post-publication writer panic"
            );
        },
        RootSinkOptions::default(),
    );
    wait_for(&signals.published, "writer result publication");

    let run = sink.write_block_i16(&iq_block(1));
    panic_release.release();
    let finish = sink.finish();
    let error = take_error(resolve_run_and_finish(run, finish), "panic run");
    assert_standard_source_is_writer(&error);
    let Error::RunAndFinalizationFailed { finalization, .. } = error else {
        panic!("writer root and panic were not composed");
    };
    assert!(finalization.to_string().contains("writer thread panicked"));
    let observations = signals.observations();
    assert!(observations.contains(&WorkerObservation::Joined));
    assert!(!observations.iter().any(|observation| matches!(
        observation,
        WorkerObservation::Detached { .. }
    )));
}

#[test]
fn finish_recovers_unconsumed_root_and_orders_stop_failure() {
    let (mut sink, _, signals) =
        build_active_failure_sink(|| {}, RootSinkOptions::default());
    wait_for(&signals.disconnected, "writer disconnect");
    let error = take_error(sink.finish(), "root-only finish");
    assert_writer_error(&error);

    let options = RootSinkOptions {
        stop_failures: 1,
        ..RootSinkOptions::default()
    };
    let (mut sink, _, signals) = build_active_failure_sink(|| {}, options);
    wait_for(&signals.disconnected, "writer disconnect");
    let error = take_error(sink.finish(), "root and stop finish");
    let Error::MultipleFinalizationFailures { first, additional } = error
    else {
        panic!("unconsumed root and stop were not ordered");
    };
    assert_writer_error(&first);
    assert_eq!(additional.len(), 1);
    assert!(additional[0].to_string().contains("rollback stop failure"));
}

#[test]
fn external_cancellation_precedes_available_writer_root() {
    let external = Arc::new(AtomicBool::new(true));
    let options = RootSinkOptions {
        external_cancellation: Some(external.clone()),
        ..RootSinkOptions::default()
    };
    let (mut sink, _, signals) = build_active_failure_sink(|| {}, options);
    wait_for(&signals.disconnected, "writer disconnect");

    let run = sink.write_block_i16(&iq_block(1));
    assert!(matches!(run, Err(Error::RunCancelled)));
    assert!(external.load(Ordering::Acquire));
    sink.request_cancel();
    let error = take_error(
        resolve_run_and_finish(run, sink.finish()),
        "cancelled writer run",
    );
    let Error::RunAndFinalizationFailed {
        primary,
        finalization,
    } = error
    else {
        panic!("cancellation and writer root were not composed");
    };
    assert!(matches!(*primary, Error::RunCancelled));
    assert_writer_error(&finalization);
}

#[test]
fn independent_run_error_precedes_ordered_sink_finalization() {
    let (sink, _, signals) =
        build_active_failure_sink(|| {}, RootSinkOptions::default());
    wait_for(&signals.disconnected, "writer disconnect");
    let mut tee = TxTee::new(vec![
        Box::new(sink),
        Box::new(FailingFinishSink {
            backend: "file",
            message: "synthetic file finalization failure",
        }),
    ]);
    let run = Err(Error::tx_backend_msg(
        "generator",
        "independent generator failure",
    ));
    let error = take_error(
        resolve_run_and_finish::<()>(run, tee.finish()),
        "independent run",
    );
    let Some(source) = StdError::source(&error) else {
        panic!("independent run error has no standard source");
    };
    assert!(source.to_string().contains("independent generator failure"));
    let Error::RunAndFinalizationFailed {
        primary,
        finalization,
    } = error
    else {
        panic!("independent run and finalization were not composed");
    };
    assert!(
        primary
            .to_string()
            .contains("independent generator failure")
    );
    let Error::MultipleFinalizationFailures { first, additional } =
        *finalization
    else {
        panic!("ordered sink finalization was not retained");
    };
    assert_writer_error(&first);
    assert_eq!(additional.len(), 1);
    assert!(
        additional[0]
            .to_string()
            .contains("file finalization failure")
    );
}

#[test]
fn hackrf_run_root_precedes_ordered_cross_sink_failures() {
    let (sink, _, signals) =
        build_active_failure_sink(|| {}, RootSinkOptions::default());
    wait_for(&signals.disconnected, "writer disconnect");
    let mut tee = TxTee::new(vec![
        Box::new(sink),
        Box::new(FailingFinishSink {
            backend: "first",
            message: "first sink finalization failure",
        }),
        Box::new(FailingFinishSink {
            backend: "second",
            message: "second sink finalization failure",
        }),
    ]);

    let run = tee.write_block_i16(&iq_block(1));
    let error =
        take_error(resolve_run_and_finish(run, tee.finish()), "cross-sink run");
    assert_standard_source_is_writer(&error);
    let Error::RunAndFinalizationFailed {
        primary,
        finalization,
    } = error
    else {
        panic!("HackRF run root and sink failures were not composed");
    };
    assert_writer_error(&primary);
    let Error::MultipleFinalizationFailures { first, additional } =
        *finalization
    else {
        panic!("cross-sink failures were not ordered");
    };
    assert!(
        first
            .to_string()
            .contains("first sink finalization failure")
    );
    assert_eq!(additional.len(), 1);
    assert!(
        additional[0]
            .to_string()
            .contains("second sink finalization failure")
    );
}
