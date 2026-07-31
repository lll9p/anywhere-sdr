use std::{
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
        mpsc::{SyncSender, TrySendError},
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

enum WriterResultState {
    Running,
    Available(WriterTerminal),
    FailureConsumed,
    Finalized,
}

pub(super) enum WriterFinalResult {
    Missing,
    Terminal(WriterTerminal),
    FailureConsumed,
}

#[derive(Clone)]
pub(super) struct WriterResultMailbox {
    state: Arc<Mutex<WriterResultState>>,
}

pub(super) struct WriterResultPublisher {
    state: Arc<Mutex<WriterResultState>>,
}

impl WriterResultMailbox {
    pub(super) fn new() -> (Self, WriterResultPublisher) {
        let state = Arc::new(Mutex::new(WriterResultState::Running));
        (
            Self {
                state: state.clone(),
            },
            WriterResultPublisher { state },
        )
    }

    pub(super) fn take_failure(&self) -> Option<Error> {
        let mut state = lock_recover(&self.state);
        match std::mem::replace(&mut *state, WriterResultState::Running) {
            WriterResultState::Available(WriterTerminal::Failed(error)) => {
                *state = WriterResultState::FailureConsumed;
                Some(error)
            }
            previous => {
                *state = previous;
                None
            }
        }
    }

    pub(super) fn take_for_finalization(&self) -> WriterFinalResult {
        let mut state = lock_recover(&self.state);
        match std::mem::replace(&mut *state, WriterResultState::Finalized) {
            WriterResultState::Running => {
                *state = WriterResultState::Running;
                WriterFinalResult::Missing
            }
            WriterResultState::Available(terminal) => {
                WriterFinalResult::Terminal(terminal)
            }
            WriterResultState::FailureConsumed => {
                WriterFinalResult::FailureConsumed
            }
            WriterResultState::Finalized => WriterFinalResult::Missing,
        }
    }

    fn is_published(&self) -> bool {
        matches!(
            *lock_recover(&self.state),
            WriterResultState::Available(_)
                | WriterResultState::FailureConsumed
        )
    }

    #[cfg(test)]
    pub(super) fn poison_for_test(&self) -> bool {
        let state = self.state.clone();
        thread::spawn(move || {
            let _guard = lock_recover(&state);
            panic!("synthetic writer-result mailbox poison");
        })
        .join()
        .is_err()
    }
}

impl WriterResultPublisher {
    pub(super) fn publish(self, terminal: WriterTerminal) {
        *lock_recover(&self.state) = WriterResultState::Available(terminal);
    }
}

fn lock_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
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
    Enqueued,
    QueueFull,
    ResultObserved,
    Joined,
    Detached { was_finished: bool },
}

pub(super) type WorkerObserver = Arc<dyn Fn(WorkerObservation) + Send + Sync>;

pub(super) struct WriterWorker {
    sender: Option<SyncSender<Vec<u8>>>,
    result: WriterResultMailbox,
    handle: Option<JoinHandle<()>>,
    cancellation: CancellationToken,
    external_cancellation: Option<Arc<AtomicBool>>,
    policy: ShutdownPolicy,
    observer: Option<WorkerObserver>,
}

impl WriterWorker {
    pub(super) fn new(
        sender: SyncSender<Vec<u8>>, result: WriterResultMailbox,
        handle: JoinHandle<()>, cancellation: CancellationToken,
        external_cancellation: Option<Arc<AtomicBool>>, policy: ShutdownPolicy,
        observer: Option<WorkerObserver>,
    ) -> Self {
        Self {
            sender: Some(sender),
            result,
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
        loop {
            self.check_enqueue_state(mode, backend, context)?;
            if let Some(error) = self.result.take_failure() {
                return Err(error);
            }
            let Some(sender) = &self.sender else {
                return Err(closed_channel_error(backend, context));
            };

            self.observe(WorkerObservation::BeforeTrySend);
            match sender.try_send(block) {
                Ok(()) => {
                    self.observe(WorkerObservation::Enqueued);
                    return match self.result.take_failure() {
                        Some(error) => Err(error),
                        None => Ok(()),
                    };
                }
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
                    if let Some(error) = self.result.take_failure() {
                        return Err(error);
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
        let mut result_observed = false;

        loop {
            self.observe_result(&mut result_observed);

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
                    self.result.take_for_finalization(),
                    backend,
                    context,
                );
            }

            let now = Instant::now();
            if now >= deadline {
                self.request_cancel();
                self.observe_result(&mut result_observed);
                let timeout = shutdown_timeout(backend, context);
                if let Some(handle) = self.handle.take() {
                    if handle.is_finished() {
                        self.observe_result(&mut result_observed);
                        self.observe(WorkerObservation::Joined);
                        return self.classify_join(
                            handle.join(),
                            self.result.take_for_finalization(),
                            backend,
                            context,
                        );
                    }
                    self.observe(WorkerObservation::Detached {
                        was_finished: false,
                    });
                    drop(handle);
                }
                if let WriterFinalResult::Terminal(WriterTerminal::Failed(
                    error,
                )) = self.result.take_for_finalization()
                {
                    tracing::warn!(
                        error = %timeout,
                        context,
                        "hackrf writer remained alive after reporting failure"
                    );
                    return Err(error);
                }
                return Err(timeout);
            }

            thread::park_timeout(self.policy.enqueue_poll.min(deadline - now));
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
        &self, join: thread::Result<()>, result: WriterFinalResult,
        backend: &'static str, context: &str,
    ) -> Result<(), Error> {
        if join.is_err() {
            return Err(Error::tx_backend_msg(
                backend,
                format!("writer thread panicked ({context})"),
            ));
        }

        match result {
            WriterFinalResult::Terminal(WriterTerminal::Completed)
            | WriterFinalResult::FailureConsumed => Ok(()),
            WriterFinalResult::Terminal(WriterTerminal::Cancelled)
                if self.cancellation.is_requested() =>
            {
                Ok(())
            }
            WriterFinalResult::Terminal(WriterTerminal::Cancelled) => {
                Err(protocol_error(
                    backend,
                    context,
                    "writer reported cancellation without a cancellation \
                     request",
                ))
            }
            WriterFinalResult::Terminal(WriterTerminal::Failed(error)) => {
                Err(error)
            }
            WriterFinalResult::Missing => Err(protocol_error(
                backend,
                context,
                "writer exited without publishing a terminal result",
            )),
        }
    }

    fn observe_result(&self, observed: &mut bool) {
        if !*observed && self.result.is_published() {
            *observed = true;
            self.observe(WorkerObservation::ResultObserved);
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
