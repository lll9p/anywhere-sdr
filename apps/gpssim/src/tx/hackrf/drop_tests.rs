use std::{
    error::Error as StdError,
    io,
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};

use super::{
    HackrfTxSink, StartupState, TxSink,
    shutdown::{
        CancellationToken, ShutdownPolicy, WorkerObservation, WriterTerminal,
        WriterWorker,
    },
    shutdown_test_support::{
        BlockingWriter, RecordingWriter, ReleaseGuard, ShutdownEvent,
        SinkOptions, build_shutdown_sink,
    },
    test_support::{iq_block, valid_config},
};
use crate::Error;

fn expect_sink(result: Result<HackrfTxSink, Error>) -> HackrfTxSink {
    match result {
        Ok(sink) => sink,
        Err(error) => panic!("drop sink construction failed: {error}"),
    }
}

#[test]
fn drop_detaches_blocked_active_writer_without_joining() {
    let mut config = valid_config();
    config.prefill_blocks = 0;
    let (entered_sender, entered_receiver) = mpsc::channel();
    let (release_sender, release_receiver) = mpsc::channel();
    let mut release = ReleaseGuard::new(release_sender);
    let (result, log) = build_shutdown_sink(
        config,
        move |log| {
            Box::new(BlockingWriter::write(
                log,
                entered_sender,
                release_receiver,
            ))
        },
        SinkOptions::default(),
    );
    let mut sink = expect_sink(result);
    assert!(sink.write_block_i16(&iq_block(1)).is_ok());
    assert!(
        entered_receiver
            .recv_timeout(Duration::from_secs(1))
            .is_ok()
    );

    let (dropped_sender, dropped_receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        drop(sink);
        if dropped_sender.send(()).is_err() {
            tracing::debug!("drop completion receiver disappeared");
        }
    });
    assert!(
        dropped_receiver
            .recv_timeout(Duration::from_secs(1))
            .is_ok()
    );
    assert!(handle.join().is_ok(), "drop thread panicked");
    let events = log.snapshot();
    assert!(events.contains(&ShutdownEvent::Detached(false)));
    assert!(!events.contains(&ShutdownEvent::Joined));
    assert!(!events.contains(&ShutdownEvent::ResultObserved));
    assert_eq!(log.count(&ShutdownEvent::Stop), 1);
    release.release();
}

#[test]
fn prepared_drop_never_flushes_joins_or_stops() {
    let (result, log) = build_shutdown_sink(
        valid_config(),
        |log| Box::new(RecordingWriter::new(log)),
        SinkOptions::default(),
    );
    drop(expect_sink(result));
    let events = log.snapshot();
    assert!(!events.contains(&ShutdownEvent::Flush));
    assert!(!events.contains(&ShutdownEvent::Joined));
    assert!(!events.contains(&ShutdownEvent::ResultObserved));
    assert!(!events.contains(&ShutdownEvent::Stop));
}

#[test]
fn writer_error_precedes_stop_failure() {
    let mut config = valid_config();
    config.prefill_blocks = 0;
    let options = SinkOptions {
        stop_failures: 1,
        ..SinkOptions::default()
    };
    let (result, log) = build_shutdown_sink(
        config,
        |log| Box::new(RecordingWriter::failing(log, io::ErrorKind::TimedOut)),
        options,
    );
    let mut sink = expect_sink(result);
    assert!(sink.write_block_i16(&iq_block(1)).is_ok());
    let error = match sink.finish() {
        Ok(()) => panic!("writer timeout succeeded"),
        Err(error) => error,
    };
    assert_eq!(sink.startup_state, StartupState::Failed);
    assert!(error.to_string().contains("synthetic writer failure"));
    let source = StdError::source(&error)
        .and_then(|source| source.downcast_ref::<io::Error>());
    assert!(
        source.is_some_and(|error| error.kind() == io::ErrorKind::TimedOut)
    );
    let events = log.snapshot();
    let write_index = events
        .iter()
        .position(|event| matches!(event, ShutdownEvent::Write(_)));
    let stop_index = events
        .iter()
        .position(|event| *event == ShutdownEvent::Stop);
    assert!(
        matches!((write_index, stop_index), (Some(write), Some(stop)) if write < stop)
    );
    assert_eq!(log.count(&ShutdownEvent::Stop), 1);
    drop(sink);
    assert_eq!(log.count(&ShutdownEvent::Stop), 2);
}

#[test]
fn successful_worker_without_terminal_result_is_a_protocol_error() {
    let (sender, receiver) = mpsc::sync_channel(1);
    let (terminal_sender, terminal_receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        drop(receiver);
        drop(terminal_sender);
    });
    let mut worker = WriterWorker::new(
        sender,
        terminal_receiver,
        handle,
        CancellationToken::default(),
        None,
        ShutdownPolicy::production(),
        None,
    );
    let error = match worker.close_and_wait(
        Instant::now() + Duration::from_secs(1),
        "hackrf",
        "test",
    ) {
        Ok(()) => panic!("missing terminal result succeeded"),
        Err(error) => error,
    };
    assert!(
        error
            .to_string()
            .contains("writer exited without publishing a terminal result")
    );
}

#[test]
fn cancelled_terminal_without_request_is_a_protocol_error() {
    let (sender, receiver) = mpsc::sync_channel(1);
    let (terminal_sender, terminal_receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        drop(receiver);
        if terminal_sender.send(WriterTerminal::Cancelled).is_err() {
            tracing::debug!("cancelled terminal receiver dropped");
        }
    });
    let mut worker = WriterWorker::new(
        sender,
        terminal_receiver,
        handle,
        CancellationToken::default(),
        None,
        ShutdownPolicy::production(),
        None,
    );
    let error = match worker.close_and_wait(
        Instant::now() + Duration::from_secs(1),
        "hackrf",
        "test",
    ) {
        Ok(()) => panic!("unrequested cancellation succeeded"),
        Err(error) => error,
    };
    assert!(error.to_string().contains(
        "writer reported cancellation without a cancellation request"
    ));
}

#[test]
fn finished_handle_drop_does_not_join_or_consume_result() {
    let (sender, receiver) = mpsc::sync_channel(1);
    let (terminal_sender, terminal_receiver) = mpsc::channel();
    let (done_sender, done_receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        if terminal_sender.send(WriterTerminal::Completed).is_err() {
            return;
        }
        drop(receiver);
        if done_sender.send(()).is_err() {
            tracing::debug!("finished-worker observer dropped");
        }
    });
    assert!(done_receiver.recv_timeout(Duration::from_secs(1)).is_ok());
    let deadline = std::time::Instant::now() + Duration::from_secs(1);
    while !handle.is_finished() {
        assert!(
            std::time::Instant::now() < deadline,
            "finished worker did not exit"
        );
        thread::yield_now();
    }
    let observations = Arc::new(Mutex::new(Vec::new()));
    let observer_events = observations.clone();
    let observer = Arc::new(move |event| match observer_events.lock() {
        Ok(mut events) => events.push(event),
        Err(poisoned) => poisoned.into_inner().push(event),
    });
    let mut worker = WriterWorker::new(
        sender,
        terminal_receiver,
        handle,
        CancellationToken::default(),
        None,
        ShutdownPolicy::production(),
        Some(observer),
    );
    worker.detach_on_drop("test");
    let events = match observations.lock() {
        Ok(events) => events.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    };
    assert!(
        events.contains(&WorkerObservation::Detached { was_finished: true })
    );
    assert!(!events.contains(&WorkerObservation::Joined));
    assert!(!events.contains(&WorkerObservation::ResultObserved));
}
