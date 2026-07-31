use std::{
    error::Error as StdError,
    io::{self, Write},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    time::Duration,
};

use super::{
    shutdown::CancellationToken,
    shutdown_test_support::{RecordingWriter, ShutdownEvent, ShutdownLog},
    test_support::valid_config,
    writer::{
        WriterObservation, WriterObserver, WriterOutcome, writer_thread_main,
        writer_thread_main_observed,
    },
};

#[test]
fn writer_observes_receive_silence_and_flush_cancellation_boundaries() {
    for target in [
        WriterObservation::WaitingForFirstBlock,
        WriterObservation::WaitingBetweenBlocks,
        WriterObservation::BeforeSilenceWrite,
        WriterObservation::BeforeFlush,
    ] {
        let cancellation = CancellationToken::default();
        let observer_cancellation = cancellation.clone();
        let observer: WriterObserver = Arc::new(move |event| {
            if event == target {
                observer_cancellation.request();
            }
        });
        let (sender, receiver) = mpsc::channel();
        let mut config = valid_config();
        config.step_duration =
            if target == WriterObservation::WaitingBetweenBlocks {
                Duration::MAX
            } else {
                Duration::from_millis(2)
            };
        config.underrun_counter = Some(Arc::new(AtomicU64::new(0)));
        if target != WriterObservation::WaitingForFirstBlock {
            assert!(sender.send(vec![1, 2, 3, 4]).is_ok());
        }
        if target == WriterObservation::BeforeFlush {
            drop(sender);
        }
        let log = ShutdownLog::default();
        let outcome = writer_thread_main_observed(
            config.clone(),
            receiver,
            RecordingWriter::new(log.clone()),
            cancellation,
            Duration::from_millis(1),
            observer,
        );
        assert!(matches!(outcome, Ok(WriterOutcome::Cancelled)));
        let events = log.snapshot();
        assert!(!events.contains(&ShutdownEvent::Flush));
        if target == WriterObservation::WaitingForFirstBlock {
            assert!(
                !events
                    .iter()
                    .any(|event| matches!(event, ShutdownEvent::Write(_)))
            );
            assert_eq!(
                config
                    .underrun_counter
                    .as_ref()
                    .map(|counter| counter.load(Ordering::Relaxed)),
                Some(0)
            );
        } else {
            assert_eq!(
                events
                    .iter()
                    .filter(|event| matches!(event, ShutdownEvent::Write(_)))
                    .count(),
                1
            );
        }
    }
}

struct CancelAfterFirstWrite {
    cancellation: CancellationToken,
    log: ShutdownLog,
}

impl Write for CancelAfterFirstWrite {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.log.push(ShutdownEvent::Write(buffer[0]));
        self.cancellation.request();
        Ok(1)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.log.push(ShutdownEvent::Flush);
        Ok(())
    }
}

struct CancelDuringFlush {
    cancellation: CancellationToken,
    log: ShutdownLog,
}

impl Write for CancelDuringFlush {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.log.push(ShutdownEvent::Write(buffer[0]));
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.log.push(ShutdownEvent::Flush);
        self.cancellation.request();
        Ok(())
    }
}

#[test]
fn cooperative_cancellation_during_flush_is_clean() {
    let cancellation = CancellationToken::default();
    let (sender, receiver) = mpsc::channel();
    assert!(sender.send(vec![1]).is_ok());
    drop(sender);
    let log = ShutdownLog::default();
    let outcome = writer_thread_main(
        valid_config(),
        receiver,
        CancelDuringFlush {
            cancellation: cancellation.clone(),
            log: log.clone(),
        },
        cancellation,
        Duration::from_millis(1),
    );
    assert!(matches!(outcome, Ok(WriterOutcome::Cancelled)));
    assert_eq!(log.snapshot(), vec![
        ShutdownEvent::Write(1),
        ShutdownEvent::Flush
    ]);
}

#[test]
fn partial_write_and_write_zero_semantics_are_preserved() {
    let (sender, receiver) = mpsc::channel();
    assert!(sender.send(vec![1, 2, 3]).is_ok());
    drop(sender);
    let log = ShutdownLog::default();
    let outcome = writer_thread_main(
        valid_config(),
        receiver,
        RecordingWriter::partial(log.clone(), 1),
        CancellationToken::default(),
        Duration::from_millis(1),
    );
    assert!(matches!(outcome, Ok(WriterOutcome::Completed)));
    assert_eq!(log.snapshot(), vec![
        ShutdownEvent::Write(1),
        ShutdownEvent::Write(2),
        ShutdownEvent::Write(3),
        ShutdownEvent::Flush,
    ]);

    let cancellation = CancellationToken::default();
    let (sender, receiver) = mpsc::channel();
    assert!(sender.send(vec![1, 2]).is_ok());
    let log = ShutdownLog::default();
    let outcome = writer_thread_main(
        valid_config(),
        receiver,
        CancelAfterFirstWrite {
            cancellation: cancellation.clone(),
            log: log.clone(),
        },
        cancellation,
        Duration::from_millis(1),
    );
    assert!(matches!(outcome, Ok(WriterOutcome::Cancelled)));
    assert_eq!(log.snapshot(), vec![ShutdownEvent::Write(1)]);

    let (sender, receiver) = mpsc::channel();
    assert!(sender.send(vec![1]).is_ok());
    let Err(error) = writer_thread_main(
        valid_config(),
        receiver,
        RecordingWriter::zero(ShutdownLog::default()),
        CancellationToken::default(),
        Duration::from_millis(1),
    ) else {
        panic!("zero write unexpectedly succeeded");
    };
    let source = StdError::source(&error)
        .and_then(|source| source.downcast_ref::<io::Error>());
    assert!(
        source.is_some_and(|error| error.kind() == io::ErrorKind::WriteZero)
    );
}
