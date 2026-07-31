use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

use super::{
    HackrfTxSink, StartupState, TxSink,
    shutdown::{
        CancellationToken, EnqueueMode, HACKRF_BULK_WRITE_TIMEOUT,
        HACKRF_DATA_PLANE_FINISH_TIMEOUT, HACKRF_ENQUEUE_CANCEL_POLL,
        ShutdownPolicy, WorkerObservation, WriterResultMailbox, WriterTerminal,
        WriterWorker,
    },
    shutdown_test_support::*,
    test_support::{iq_block, valid_config},
};
use crate::Error;

fn expect_sink(result: Result<HackrfTxSink, Error>) -> HackrfTxSink {
    match result {
        Ok(sink) => sink,
        Err(error) => panic!("shutdown sink construction failed: {error}"),
    }
}

fn short_policy() -> ShutdownPolicy {
    ShutdownPolicy {
        bulk_write_timeout: HACKRF_BULK_WRITE_TIMEOUT,
        enqueue_poll: Duration::from_millis(2),
        finish_timeout: Duration::from_millis(50),
    }
}

fn wait_for_observation(
    receiver: &mpsc::Receiver<WorkerObservation>, expected: WorkerObservation,
) {
    loop {
        match receiver.recv_timeout(Duration::from_secs(1)) {
            Ok(observation) if observation == expected => return,
            Ok(_) => {}
            Err(error) => panic!("missing {expected:?} observation: {error}"),
        }
    }
}

#[test]
fn production_shutdown_policy_and_writer_timeout_are_exact() {
    assert_eq!(HACKRF_BULK_WRITE_TIMEOUT, Duration::from_secs(1));
    assert_eq!(HACKRF_ENQUEUE_CANCEL_POLL, Duration::from_millis(10));
    assert_eq!(HACKRF_DATA_PLANE_FINISH_TIMEOUT, Duration::from_secs(2));

    let config = valid_config();
    let transfer_bytes = config.usb_transfer_bytes;
    let transfers = config.usb_transfers;
    let (result, log) = build_shutdown_sink(
        config,
        |log| Box::new(RecordingWriter::new(log)),
        SinkOptions::default(),
    );
    let mut sink = expect_sink(result);
    assert!(sink.finish().is_ok());
    assert!(log.snapshot().contains(&ShutdownEvent::Prepare {
        transfer_bytes,
        transfers,
        timeout: Duration::from_secs(1),
    }));
}

#[test]
fn healthy_full_queue_preserves_every_block_in_order() {
    let mut config = valid_config();
    config.prefill_blocks = 0;
    config.queue_blocks = 1;
    let (entered_sender, entered_receiver) = mpsc::channel();
    let (release_sender, release_receiver) = mpsc::channel();
    let mut release = ReleaseGuard::new(release_sender);
    let (observation_sender, observation_receiver) = mpsc::channel();
    let options = SinkOptions {
        observation_sender: Some(observation_sender),
        ..SinkOptions::default()
    };
    let (result, log) = build_shutdown_sink(
        config,
        move |log| {
            Box::new(BlockingWriter::write(
                log,
                entered_sender,
                release_receiver,
            ))
        },
        options,
    );
    let mut sink = expect_sink(result);
    assert!(sink.write_block_i16(&iq_block(1)).is_ok());
    assert!(
        entered_receiver
            .recv_timeout(Duration::from_secs(1))
            .is_ok()
    );
    assert!(sink.write_block_i16(&iq_block(2)).is_ok());

    let join = thread::spawn(move || {
        let write = sink.write_block_i16(&iq_block(3));
        let finish = sink.finish();
        (write, finish)
    });
    wait_for_observation(&observation_receiver, WorkerObservation::QueueFull);
    release.release();
    let Ok((write, finish)) = join.join() else {
        panic!("queue writer panicked");
    };
    assert!(write.is_ok());
    assert!(finish.is_ok());
    let writes = log
        .snapshot()
        .into_iter()
        .filter_map(|event| match event {
            ShutdownEvent::Write(value) => Some(value),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(writes, vec![1, 2, 3]);
}

#[test]
fn full_queue_user_cancellation_unblocks_and_cleans_up() {
    let mut config = valid_config();
    config.prefill_blocks = 0;
    config.queue_blocks = 1;
    let external = Arc::new(AtomicBool::new(false));
    let (entered_sender, entered_receiver) = mpsc::channel();
    let (release_sender, release_receiver) = mpsc::channel();
    let mut release = ReleaseGuard::new(release_sender);
    let (observation_sender, observation_receiver) = mpsc::channel();
    let options = SinkOptions {
        external_cancellation: Some(external.clone()),
        observation_sender: Some(observation_sender),
        ..SinkOptions::default()
    };
    let (result, _) = build_shutdown_sink(
        config,
        move |log| {
            Box::new(BlockingWriter::write(
                log,
                entered_sender,
                release_receiver,
            ))
        },
        options,
    );
    let mut sink = expect_sink(result);
    assert!(sink.write_block_i16(&iq_block(1)).is_ok());
    assert!(
        entered_receiver
            .recv_timeout(Duration::from_secs(1))
            .is_ok()
    );
    assert!(sink.write_block_i16(&iq_block(2)).is_ok());
    let (write_done_sender, write_done_receiver) = mpsc::channel();
    let join = thread::spawn(move || {
        let write = sink.write_block_i16(&iq_block(3));
        if write_done_sender
            .send(matches!(write, Err(Error::RunCancelled)))
            .is_err()
        {
            return Err(Error::msg("write observer dropped"));
        }
        if write.is_err() {
            sink.request_cancel();
        }
        let finish = sink.finish();
        Ok((write, finish))
    });
    wait_for_observation(&observation_receiver, WorkerObservation::QueueFull);
    external.store(true, Ordering::Release);
    assert_eq!(
        write_done_receiver.recv_timeout(Duration::from_secs(1)),
        Ok(true)
    );
    release.release();
    let Ok(join_result) = join.join() else {
        panic!("cancelled queue writer panicked");
    };
    let Ok((write, finish)) = join_result else {
        panic!("cancelled queue protocol failed");
    };
    assert!(matches!(write, Err(Error::RunCancelled)));
    assert!(finish.is_ok());
}

#[test]
fn disconnected_send_rechecks_external_cancellation() {
    let (sender, receiver) = mpsc::sync_channel(1);
    let receiver = Arc::new(Mutex::new(Some(receiver)));
    let external = Arc::new(AtomicBool::new(false));
    let observer_receiver = receiver.clone();
    let observer_external = external.clone();
    let observer = Arc::new(move |event| {
        if event == WorkerObservation::BeforeTrySend {
            observer_external.store(true, Ordering::Release);
            match observer_receiver.lock() {
                Ok(mut receiver) => receiver.take(),
                Err(poisoned) => poisoned.into_inner().take(),
            };
        }
    });
    let (result, result_publisher) = WriterResultMailbox::new();
    let handle = thread::spawn(move || drop(result_publisher));
    let mut worker = WriterWorker::new(
        sender,
        result,
        handle,
        CancellationToken::default(),
        Some(external),
        short_policy(),
        Some(observer),
    );
    assert!(matches!(
        worker.send(vec![1], EnqueueMode::Streaming, "hackrf", "test"),
        Err(Error::RunCancelled)
    ));
    worker.detach_on_drop("test");
}

#[test]
fn prepared_prefill_is_abortive_only_when_internal_cancel_is_latched() {
    let external = Arc::new(AtomicBool::new(false));
    let options = SinkOptions {
        external_cancellation: Some(external.clone()),
        ..SinkOptions::default()
    };
    let (result, log) = build_shutdown_sink(
        valid_config(),
        |log| Box::new(RecordingWriter::new(log)),
        options,
    );
    let mut sink = expect_sink(result);
    assert!(sink.write_block_i16(&iq_block(1)).is_ok());
    sink.request_cancel();
    assert!(sink.finish().is_ok());
    assert_eq!(sink.startup_state, StartupState::Finished);
    let events = log.snapshot();
    assert!(!events.contains(&ShutdownEvent::Activate));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, ShutdownEvent::Write(_)))
    );
    assert!(!events.contains(&ShutdownEvent::Flush));
    assert!(!events.contains(&ShutdownEvent::Stop));

    let options = SinkOptions {
        external_cancellation: Some(external.clone()),
        ..SinkOptions::default()
    };
    let (result, log) = build_shutdown_sink(
        valid_config(),
        |log| Box::new(RecordingWriter::new(log)),
        options,
    );
    let mut sink = expect_sink(result);
    assert!(sink.write_block_i16(&iq_block(1)).is_ok());
    external.store(true, Ordering::Release);
    assert!(sink.finish().is_ok());
    let events = log.snapshot();
    assert!(events.contains(&ShutdownEvent::Activate));
    assert!(events.contains(&ShutdownEvent::Write(1)));
    assert!(events.contains(&ShutdownEvent::Flush));
    assert!(events.contains(&ShutdownEvent::Joined));
    assert!(events.contains(&ShutdownEvent::Stop));
}

#[test]
fn normal_finish_drains_flushes_joins_and_stops_in_order() {
    let mut config = valid_config();
    config.prefill_blocks = 0;
    let (result, log) = build_shutdown_sink(
        config,
        |log| Box::new(RecordingWriter::new(log)),
        SinkOptions::default(),
    );
    let mut sink = expect_sink(result);
    assert!(sink.write_block_i16(&iq_block(1)).is_ok());
    assert!(sink.write_block_i16(&iq_block(2)).is_ok());
    assert!(sink.finish().is_ok());
    let all_events = log.snapshot();
    assert!(!all_events.contains(&ShutdownEvent::CancelRequested));
    let events = all_events
        .into_iter()
        .filter(|event| {
            matches!(
                event,
                ShutdownEvent::Activate
                    | ShutdownEvent::Write(_)
                    | ShutdownEvent::Flush
                    | ShutdownEvent::ResultObserved
                    | ShutdownEvent::Joined
                    | ShutdownEvent::Stop
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(events, vec![
        ShutdownEvent::Activate,
        ShutdownEvent::Write(1),
        ShutdownEvent::Write(2),
        ShutdownEvent::Flush,
        ShutdownEvent::ResultObserved,
        ShutdownEvent::Joined,
        ShutdownEvent::Stop,
    ]);
}

#[test]
fn blocking_write_and_flush_detach_at_the_absolute_deadline() {
    for block_flush in [false, true] {
        let mut config = valid_config();
        config.prefill_blocks = 0;
        let (entered_sender, entered_receiver) = mpsc::channel();
        let (release_sender, release_receiver) = mpsc::channel();
        let mut release = ReleaseGuard::new(release_sender);
        let options = SinkOptions {
            policy: short_policy(),
            ..SinkOptions::default()
        };
        let (result, log) = build_shutdown_sink(
            config,
            move |log| {
                if block_flush {
                    Box::new(BlockingWriter::flush(
                        log,
                        entered_sender,
                        release_receiver,
                    ))
                } else {
                    Box::new(BlockingWriter::write(
                        log,
                        entered_sender,
                        release_receiver,
                    ))
                }
            },
            options,
        );
        let mut sink = expect_sink(result);
        assert!(sink.write_block_i16(&iq_block(1)).is_ok());
        if block_flush {
            let join = thread::spawn(move || sink.finish());
            assert!(
                entered_receiver
                    .recv_timeout(Duration::from_secs(1))
                    .is_ok()
            );
            let Ok(finish_result) = join.join() else {
                panic!("flush finish panicked");
            };
            let Err(error) = finish_result else {
                panic!("flush finish succeeded");
            };
            assert!(error.to_string().contains("writer shutdown timed out"));
        } else {
            assert!(
                entered_receiver
                    .recv_timeout(Duration::from_secs(1))
                    .is_ok()
            );
            let error = match sink.finish() {
                Ok(()) => panic!("write finish succeeded"),
                Err(error) => error,
            };
            assert_eq!(sink.startup_state, StartupState::Failed);
            assert!(sink.finish().is_ok());
            assert_eq!(sink.startup_state, StartupState::Failed);
            assert!(error.to_string().contains("writer shutdown timed out"));
        }
        assert!(log.snapshot().contains(&ShutdownEvent::Detached(false)));
        assert_eq!(log.count(&ShutdownEvent::Stop), 1);
        release.release();
    }
}

#[test]
fn terminal_result_without_thread_exit_is_still_detached() {
    let (sender, receiver) = mpsc::sync_channel(1);
    let (result, result_publisher) = WriterResultMailbox::new();
    let (release_sender, release_receiver) = mpsc::channel();
    let mut release = ReleaseGuard::new(release_sender);
    let handle = thread::spawn(move || {
        result_publisher.publish(WriterTerminal::Completed);
        if release_receiver.recv().is_err() {
            tracing::debug!("post-result release sender dropped");
        }
        drop(receiver);
    });
    let mut worker = WriterWorker::new(
        sender,
        result,
        handle,
        CancellationToken::default(),
        None,
        short_policy(),
        None,
    );
    let error = match worker.close_and_wait(
        Instant::now() + Duration::from_millis(30),
        "hackrf",
        "test",
    ) {
        Ok(()) => panic!("live post-result thread was joined"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("writer shutdown timed out"));
    release.release();
}

#[test]
fn published_writer_failure_precedes_detach_timeout() {
    let (sender, receiver) = mpsc::sync_channel(1);
    let (result, result_publisher) = WriterResultMailbox::new();
    let (release_sender, release_receiver) = mpsc::channel();
    let mut release = ReleaseGuard::new(release_sender);
    let handle = thread::spawn(move || {
        let error = Error::tx_backend_msg("hackrf", "published failure");
        result_publisher.publish(WriterTerminal::Failed(error));
        if release_receiver.recv().is_err() {
            tracing::debug!("published-failure release sender dropped");
        }
        drop(receiver);
    });
    let mut worker = WriterWorker::new(
        sender,
        result,
        handle,
        CancellationToken::default(),
        None,
        short_policy(),
        None,
    );
    let error = match worker.close_and_wait(
        Instant::now() + Duration::from_millis(30),
        "hackrf",
        "test",
    ) {
        Ok(()) => panic!("published writer failure succeeded"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("published failure"));
    assert!(!error.to_string().contains("writer shutdown timed out"));
    release.release();
}

#[test]
fn partial_prefill_dispatch_deadline_owns_only_one_stop_attempt() {
    let mut config = valid_config();
    config.prefill_blocks = 4;
    config.queue_blocks = 1;
    let (entered_sender, _entered_receiver) = mpsc::channel();
    let (release_sender, release_receiver) = mpsc::channel();
    let mut release = ReleaseGuard::new(release_sender);
    let options = SinkOptions {
        policy: short_policy(),
        stop_failures: 1,
        ..SinkOptions::default()
    };
    let (result, log) = build_shutdown_sink(
        config,
        move |log| {
            Box::new(BlockingWriter::write(
                log,
                entered_sender,
                release_receiver,
            ))
        },
        options,
    );
    let mut sink = expect_sink(result);
    for index in 1..=3 {
        assert!(sink.write_block_i16(&iq_block(index)).is_ok());
    }
    let error = match sink.finish() {
        Ok(()) => panic!("prefill deadline succeeded"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("writer shutdown timed out"));
    assert_eq!(log.count(&ShutdownEvent::Stop), 1);
    drop(sink);
    assert_eq!(log.count(&ShutdownEvent::Stop), 2);
    release.release();
}
