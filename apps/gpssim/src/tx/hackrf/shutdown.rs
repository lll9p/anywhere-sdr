use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use crate::Error;

pub(super) const HACKRF_BULK_WRITE_TIMEOUT: Duration = Duration::from_secs(1);
pub(super) const HACKRF_ENQUEUE_CANCEL_POLL: Duration =
    Duration::from_millis(10);
pub(super) const HACKRF_DATA_PLANE_FINISH_TIMEOUT: Duration =
    Duration::from_secs(2);

#[derive(Clone, Copy)]
pub(super) struct ShutdownPolicy {
    pub(super) bulk_write_timeout: Duration,
    pub(super) enqueue_poll: Duration,
    pub(super) finish_timeout: Duration,
}

impl ShutdownPolicy {
    pub(super) const fn production() -> Self {
        Self {
            bulk_write_timeout: HACKRF_BULK_WRITE_TIMEOUT,
            enqueue_poll: HACKRF_ENQUEUE_CANCEL_POLL,
            finish_timeout: HACKRF_DATA_PLANE_FINISH_TIMEOUT,
        }
    }
}

#[derive(Clone, Default)]
pub(super) struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub(super) fn request(&self) -> bool {
        !self.0.swap(true, Ordering::Release)
    }

    pub(super) fn is_requested(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

pub(super) enum WriterTerminal {
    Completed,
    Cancelled,
    Failed(Error),
}

#[derive(Clone, Copy)]
pub(super) enum EnqueueMode {
    Streaming,
    Finalizing { deadline: Instant },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum WorkerObservation {
    CancelRequested,
    BeforeTrySend,
    QueueFull,
    ResultObserved,
    Joined,
    Detached { was_finished: bool },
}

pub(super) type WorkerObserver = Arc<dyn Fn(WorkerObservation) + Send + Sync>;

pub(super) struct WriterWorker {
    sender: Option<SyncSender<Vec<u8>>>,
    terminal_receiver: Receiver<WriterTerminal>,
    handle: Option<JoinHandle<()>>,
    cancellation: CancellationToken,
    external_cancellation: Option<Arc<AtomicBool>>,
    policy: ShutdownPolicy,
    observer: Option<WorkerObserver>,
}

impl WriterWorker {
    pub(super) fn new(
        sender: SyncSender<Vec<u8>>,
        terminal_receiver: Receiver<WriterTerminal>, handle: JoinHandle<()>,
        cancellation: CancellationToken,
        external_cancellation: Option<Arc<AtomicBool>>, policy: ShutdownPolicy,
        observer: Option<WorkerObserver>,
    ) -> Self {
        Self {
            sender: Some(sender),
            terminal_receiver,
            handle: Some(handle),
            cancellation,
            external_cancellation,
            policy,
            observer,
        }
    }

    pub(super) fn policy(&self) -> ShutdownPolicy {
        self.policy
    }

    pub(super) fn request_cancel(&self) {
        if self.cancellation.request() {
            self.observe(WorkerObservation::CancelRequested);
        }
    }

    pub(super) fn is_cancel_requested(&self) -> bool {
        self.cancellation.is_requested()
    }

    pub(super) fn send(
        &self, mut block: Vec<u8>, mode: EnqueueMode, backend: &'static str,
        context: &str,
    ) -> Result<(), Error> {
        let Some(sender) = &self.sender else {
            return Err(closed_channel_error(backend, context));
        };

        loop {
            self.check_enqueue_state(mode, backend, context)?;
            self.observe(WorkerObservation::BeforeTrySend);
            match sender.try_send(block) {
                Ok(()) => return Ok(()),
                Err(TrySendError::Full(returned)) => {
                    block = returned;
                    self.observe(WorkerObservation::QueueFull);
                    self.check_enqueue_state(mode, backend, context)?;
                    thread::park_timeout(self.enqueue_wait(mode));
                }
                Err(error @ TrySendError::Disconnected(_)) => {
                    if matches!(mode, EnqueueMode::Streaming)
                        && self.external_cancelled()
                    {
                        return Err(Error::RunCancelled);
                    }
                    return Err(Error::tx_backend_with_source(
                        backend,
                        context.to_string(),
                        error,
                    ));
                }
            }
        }
    }

    pub(super) fn close_and_wait(
        &mut self, deadline: Instant, backend: &'static str, context: &str,
    ) -> Result<(), Error> {
        self.sender.take();
        if self.handle.is_none() {
            return Ok(());
        }
        let mut terminal = None;
        let mut terminal_disconnected = false;

        loop {
            if terminal.is_none() && !terminal_disconnected {
                match self.terminal_receiver.try_recv() {
                    Ok(result) => {
                        terminal = Some(result);
                        self.observe(WorkerObservation::ResultObserved);
                    }
                    Err(mpsc::TryRecvError::Empty) => {}
                    Err(mpsc::TryRecvError::Disconnected) => {
                        terminal_disconnected = true;
                    }
                }
            }

            if self.handle.as_ref().is_some_and(JoinHandle::is_finished) {
                let Some(handle) = self.handle.take() else {
                    return Err(protocol_error(
                        backend,
                        context,
                        "writer handle disappeared before join",
                    ));
                };
                self.observe(WorkerObservation::Joined);
                return self.classify_join(
                    handle.join(),
                    terminal,
                    backend,
                    context,
                );
            }

            let now = Instant::now();
            if now >= deadline {
                self.request_cancel();
                if terminal.is_none()
                    && let Ok(result) = self.terminal_receiver.try_recv()
                {
                    terminal = Some(result);
                    self.observe(WorkerObservation::ResultObserved);
                }
                let timeout = shutdown_timeout(backend, context);
                if let Some(handle) = self.handle.take() {
                    if handle.is_finished() {
                        if terminal.is_none()
                            && let Ok(result) =
                                self.terminal_receiver.try_recv()
                        {
                            terminal = Some(result);
                            self.observe(WorkerObservation::ResultObserved);
                        }
                        self.observe(WorkerObservation::Joined);
                        return self.classify_join(
                            handle.join(),
                            terminal,
                            backend,
                            context,
                        );
                    }
                    self.observe(WorkerObservation::Detached {
                        was_finished: false,
                    });
                    drop(handle);
                }
                if let Some(WriterTerminal::Failed(error)) = terminal {
                    tracing::warn!(
                        error = %timeout,
                        context,
                        "hackrf writer remained alive after reporting failure"
                    );
                    return Err(error);
                }
                return Err(timeout);
            }

            let wait = self.policy.enqueue_poll.min(deadline - now);
            if terminal.is_none() && !terminal_disconnected {
                match self.terminal_receiver.recv_timeout(wait) {
                    Ok(result) => {
                        terminal = Some(result);
                        self.observe(WorkerObservation::ResultObserved);
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        terminal_disconnected = true;
                    }
                }
            } else {
                thread::park_timeout(wait);
            }
        }
    }

    pub(super) fn detach_on_drop(&mut self, context: &str) {
        if self.handle.is_none() && self.sender.is_none() {
            return;
        }
        self.request_cancel();
        self.sender.take();
        let Some(handle) = self.handle.take() else {
            return;
        };
        let was_finished = handle.is_finished();
        self.observe(WorkerObservation::Detached { was_finished });
        if !was_finished {
            tracing::warn!(
                context,
                "detaching unfinished hackrf writer during sink drop"
            );
        }
        drop(handle);
    }

    fn classify_join(
        &self, join: thread::Result<()>, terminal: Option<WriterTerminal>,
        backend: &'static str, context: &str,
    ) -> Result<(), Error> {
        if join.is_err() {
            return Err(Error::tx_backend_msg(
                backend,
                format!("writer thread panicked ({context})"),
            ));
        }

        match terminal {
            Some(WriterTerminal::Completed) => Ok(()),
            Some(WriterTerminal::Cancelled)
                if self.cancellation.is_requested() =>
            {
                Ok(())
            }
            Some(WriterTerminal::Cancelled) => Err(protocol_error(
                backend,
                context,
                "writer reported cancellation without a cancellation request",
            )),
            Some(WriterTerminal::Failed(error)) => Err(error),
            None => Err(protocol_error(
                backend,
                context,
                "writer exited without publishing a terminal result",
            )),
        }
    }

    fn check_enqueue_state(
        &self, mode: EnqueueMode, backend: &'static str, context: &str,
    ) -> Result<(), Error> {
        match mode {
            EnqueueMode::Streaming => {
                if self.external_cancelled() {
                    return Err(Error::RunCancelled);
                }
                if self.cancellation.is_requested() {
                    return Err(Error::tx_backend_msg(
                        backend,
                        format!("writer cancellation requested ({context})"),
                    ));
                }
            }
            EnqueueMode::Finalizing { deadline } => {
                if Instant::now() >= deadline {
                    self.request_cancel();
                    return Err(shutdown_timeout(backend, context));
                }
                if self.cancellation.is_requested() {
                    return Err(Error::tx_backend_msg(
                        backend,
                        format!("writer cancellation requested ({context})"),
                    ));
                }
            }
        }
        Ok(())
    }

    fn enqueue_wait(&self, mode: EnqueueMode) -> Duration {
        match mode {
            EnqueueMode::Streaming => self.policy.enqueue_poll,
            EnqueueMode::Finalizing { deadline } => deadline
                .saturating_duration_since(Instant::now())
                .min(self.policy.enqueue_poll),
        }
    }

    fn external_cancelled(&self) -> bool {
        self.external_cancellation
            .as_ref()
            .is_some_and(|cancel| cancel.load(Ordering::Acquire))
    }

    fn observe(&self, observation: WorkerObservation) {
        if let Some(observer) = &self.observer {
            observer(observation);
        }
    }
}

fn shutdown_timeout(backend: &'static str, context: &str) -> Error {
    Error::tx_backend_msg(
        backend,
        format!("writer shutdown timed out ({context})"),
    )
}

fn closed_channel_error(backend: &'static str, context: &str) -> Error {
    Error::tx_backend_msg(
        backend,
        format!("writer channel is closed ({context})"),
    )
}

fn protocol_error(
    backend: &'static str, context: &str, message: &str,
) -> Error {
    Error::tx_backend_msg(backend, format!("{message} ({context})"))
}
