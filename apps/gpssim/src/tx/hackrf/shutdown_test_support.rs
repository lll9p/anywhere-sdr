use std::{
    io::{self, Write},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Duration,
};

use super::{
    HackrfTxConfig, HackrfTxSink,
    shutdown::{
        CancellationToken, ShutdownPolicy, WorkerObservation, WorkerObserver,
        WriterTerminal,
    },
    startup::{ConstructionOptions, HackrfDeviceControl},
    writer::{WriterOutcome, writer_thread_main},
};
use crate::Error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum ShutdownEvent {
    Prepare {
        transfer_bytes: usize,
        transfers: usize,
        timeout: Duration,
    },
    Activate,
    Write(u8),
    Flush,
    Stop,
    QueueFull,
    CancelRequested,
    BeforeTrySend,
    ResultObserved,
    Joined,
    Detached(bool),
    WriterExited,
}

#[derive(Clone, Default)]
pub(super) struct ShutdownLog(Arc<Mutex<Vec<ShutdownEvent>>>);

impl ShutdownLog {
    pub(super) fn push(&self, event: ShutdownEvent) {
        match self.0.lock() {
            Ok(mut events) => events.push(event),
            Err(poisoned) => poisoned.into_inner().push(event),
        }
    }

    pub(super) fn snapshot(&self) -> Vec<ShutdownEvent> {
        match self.0.lock() {
            Ok(events) => events.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    pub(super) fn count(&self, expected: &ShutdownEvent) -> usize {
        self.snapshot()
            .iter()
            .filter(|event| *event == expected)
            .count()
    }
}

pub(super) struct SinkOptions {
    pub(super) policy: ShutdownPolicy,
    pub(super) external_cancellation:
        Option<Arc<std::sync::atomic::AtomicBool>>,
    pub(super) stop_failures: usize,
    pub(super) observation_sender: Option<mpsc::Sender<WorkerObservation>>,
}

impl Default for SinkOptions {
    fn default() -> Self {
        Self {
            policy: ShutdownPolicy::production(),
            external_cancellation: None,
            stop_failures: 0,
            observation_sender: None,
        }
    }
}

pub(super) fn build_shutdown_sink<MakeWriter>(
    config: HackrfTxConfig, make_writer: MakeWriter, options: SinkOptions,
) -> (Result<HackrfTxSink, Error>, ShutdownLog)
where
    MakeWriter: FnOnce(ShutdownLog) -> Box<dyn Write + Send>,
{
    let SinkOptions {
        policy,
        external_cancellation,
        stop_failures,
        observation_sender,
    } = options;
    let log = ShutdownLog::default();
    let writer = make_writer(log.clone());
    let device_log = log.clone();
    let worker_log = log.clone();
    let observer: WorkerObserver = Arc::new(move |observation| {
        if let Some(sender) = &observation_sender
            && sender.send(observation).is_err()
        {
            tracing::debug!("shutdown observation receiver dropped");
        }
        worker_log.push(match observation {
            WorkerObservation::CancelRequested => {
                ShutdownEvent::CancelRequested
            }
            WorkerObservation::BeforeTrySend => ShutdownEvent::BeforeTrySend,
            WorkerObservation::QueueFull => ShutdownEvent::QueueFull,
            WorkerObservation::ResultObserved => ShutdownEvent::ResultObserved,
            WorkerObservation::Joined => ShutdownEvent::Joined,
            WorkerObservation::Detached { was_finished } => {
                ShutdownEvent::Detached(was_finished)
            }
        });
    });
    let construction = ConstructionOptions {
        buffer_observer: None,
        external_cancellation,
        policy,
        worker_observer: Some(observer),
    };
    let result = HackrfTxSink::new_with_options(
        config,
        4,
        move |_| {
            Ok(Box::new(ShutdownDevice {
                log: device_log,
                writer: Some(writer),
                stop_failures,
            }))
        },
        spawn_standard_writer,
        construction,
    );
    (result, log)
}

pub(super) fn spawn_standard_writer(
    config: HackrfTxConfig, receiver: mpsc::Receiver<Vec<u8>>,
    writer: Box<dyn Write + Send>, cancellation: CancellationToken,
    terminal_sender: mpsc::Sender<WriterTerminal>, cancellation_poll: Duration,
) -> io::Result<thread::JoinHandle<()>> {
    thread::Builder::new()
        .name("hackrf-shutdown-test-writer".to_string())
        .spawn(move || {
            publish_terminal(
                writer_thread_main(
                    config,
                    receiver,
                    writer,
                    cancellation,
                    cancellation_poll,
                ),
                terminal_sender,
            );
        })
}

pub(super) fn publish_terminal(
    result: Result<WriterOutcome, Error>,
    terminal_sender: mpsc::Sender<WriterTerminal>,
) {
    let terminal = match result {
        Ok(WriterOutcome::Completed) => WriterTerminal::Completed,
        Ok(WriterOutcome::Cancelled) => WriterTerminal::Cancelled,
        Err(error) => WriterTerminal::Failed(error),
    };
    if terminal_sender.send(terminal).is_err() {
        tracing::debug!("shutdown test owner dropped before terminal result");
    }
}

pub(super) struct ShutdownDevice {
    pub(super) log: ShutdownLog,
    pub(super) writer: Option<Box<dyn Write + Send>>,
    pub(super) stop_failures: usize,
}

impl HackrfDeviceControl for ShutdownDevice {
    fn set_freq(&mut self, _frequency_hz: u64) -> Result<(), Error> {
        Ok(())
    }

    fn set_sample_rate_auto(
        &mut self, _frequency_hz: f64,
    ) -> Result<(), Error> {
        Ok(())
    }

    fn set_amp_enable(&mut self, _enabled: bool) -> Result<(), Error> {
        Ok(())
    }

    fn set_txvga_gain(&mut self, _gain: u16) -> Result<(), Error> {
        Ok(())
    }

    fn prepare_tx_writer(
        &mut self, transfer_bytes: usize, transfers: usize,
        write_timeout: Duration,
    ) -> Result<Box<dyn Write + Send>, Error> {
        self.log.push(ShutdownEvent::Prepare {
            transfer_bytes,
            transfers,
            timeout: write_timeout,
        });
        self.writer
            .take()
            .ok_or_else(|| Error::tx_backend_msg("hackrf", "missing writer"))
    }

    fn enter_tx_mode(&mut self) -> Result<(), Error> {
        self.log.push(ShutdownEvent::Activate);
        Ok(())
    }

    fn stop_tx(&mut self) -> Result<(), Error> {
        self.log.push(ShutdownEvent::Stop);
        if self.stop_failures > 0 {
            self.stop_failures -= 1;
            Err(Error::tx_backend_msg("hackrf", "synthetic stop failure"))
        } else {
            Ok(())
        }
    }
}

pub(super) struct RecordingWriter {
    log: ShutdownLog,
    maximum_write: usize,
    zero_write: bool,
    failure: Option<io::ErrorKind>,
}

impl RecordingWriter {
    pub(super) fn new(log: ShutdownLog) -> Self {
        Self {
            log,
            maximum_write: usize::MAX,
            zero_write: false,
            failure: None,
        }
    }

    pub(super) fn partial(log: ShutdownLog, maximum_write: usize) -> Self {
        Self {
            log,
            maximum_write,
            zero_write: false,
            failure: None,
        }
    }

    pub(super) fn zero(log: ShutdownLog) -> Self {
        Self {
            log,
            maximum_write: usize::MAX,
            zero_write: true,
            failure: None,
        }
    }

    pub(super) fn failing(log: ShutdownLog, kind: io::ErrorKind) -> Self {
        Self {
            log,
            maximum_write: usize::MAX,
            zero_write: false,
            failure: Some(kind),
        }
    }
}

impl Write for RecordingWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.log
            .push(ShutdownEvent::Write(buffer.first().copied().unwrap_or(0)));
        if let Some(kind) = self.failure {
            return Err(io::Error::new(kind, "synthetic writer failure"));
        }
        if self.zero_write {
            return Ok(0);
        }
        Ok(buffer.len().min(self.maximum_write))
    }

    fn flush(&mut self) -> io::Result<()> {
        self.log.push(ShutdownEvent::Flush);
        Ok(())
    }
}

pub(super) struct BlockingWriter {
    log: ShutdownLog,
    block_flush: bool,
    entered: Option<mpsc::Sender<()>>,
    release: mpsc::Receiver<()>,
}

impl BlockingWriter {
    pub(super) fn write(
        log: ShutdownLog, entered: mpsc::Sender<()>,
        release: mpsc::Receiver<()>,
    ) -> Self {
        Self {
            log,
            block_flush: false,
            entered: Some(entered),
            release,
        }
    }

    pub(super) fn flush(
        log: ShutdownLog, entered: mpsc::Sender<()>,
        release: mpsc::Receiver<()>,
    ) -> Self {
        Self {
            log,
            block_flush: true,
            entered: Some(entered),
            release,
        }
    }

    fn wait(&mut self) -> io::Result<()> {
        let Some(entered) = self.entered.take() else {
            return Ok(());
        };
        if entered.send(()).is_err() {
            return Err(io::Error::other("entered observer dropped"));
        }
        self.release
            .recv()
            .map_err(|error| io::Error::other(error.to_string()))
    }
}

impl Write for BlockingWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.log
            .push(ShutdownEvent::Write(buffer.first().copied().unwrap_or(0)));
        if !self.block_flush {
            self.wait()?;
        }
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.log.push(ShutdownEvent::Flush);
        if self.block_flush {
            self.wait()?;
        }
        Ok(())
    }
}

impl Drop for BlockingWriter {
    fn drop(&mut self) {
        self.log.push(ShutdownEvent::WriterExited);
    }
}

pub(super) struct ReleaseGuard {
    sender: Option<mpsc::Sender<()>>,
}

impl ReleaseGuard {
    pub(super) fn new(sender: mpsc::Sender<()>) -> Self {
        Self {
            sender: Some(sender),
        }
    }

    pub(super) fn release(&mut self) {
        if let Some(sender) = self.sender.take()
            && sender.send(()).is_err()
        {
            tracing::debug!("blocked test writer already exited");
        }
    }
}

impl Drop for ReleaseGuard {
    fn drop(&mut self) {
        self.release();
    }
}
