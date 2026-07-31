use std::{
    collections::VecDeque,
    error::Error as StdError,
    io,
    sync::{Arc, Barrier, Mutex, atomic::AtomicBool, mpsc},
    thread,
    time::Duration,
};

use super::{
    HackrfTxSink, StartupState, config_context,
    shutdown::{
        CancellationToken, ShutdownPolicy, WorkerObservation, WorkerObserver,
        WriterResultMailbox, WriterTerminal, WriterWorker,
    },
    test_support::{EventLog, Scenario, fake_activation_guard, valid_config},
};
use crate::Error;

pub(super) const WRITER_FAILURE_MESSAGE: &str =
    "synthetic asynchronous USB failure";

pub(super) struct RootSinkOptions {
    pub(super) policy: ShutdownPolicy,
    pub(super) external_cancellation: Option<Arc<AtomicBool>>,
    pub(super) stop_failures: usize,
}

impl Default for RootSinkOptions {
    fn default() -> Self {
        Self {
            policy: ShutdownPolicy::production(),
            external_cancellation: None,
            stop_failures: 0,
        }
    }
}

pub(super) struct WorkerSignals {
    pub(super) published: mpsc::Receiver<()>,
    pub(super) disconnected: mpsc::Receiver<()>,
    observations: Arc<Mutex<Vec<WorkerObservation>>>,
}

impl WorkerSignals {
    pub(super) fn observations(&self) -> Vec<WorkerObservation> {
        match self.observations.lock() {
            Ok(observations) => observations.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }
}

pub(super) fn short_policy() -> ShutdownPolicy {
    ShutdownPolicy {
        bulk_write_timeout: Duration::from_secs(1),
        enqueue_poll: Duration::from_millis(2),
        finish_timeout: Duration::from_millis(40),
    }
}

pub(super) fn build_active_failure_sink<AfterPublish>(
    after_publish: AfterPublish, options: RootSinkOptions,
) -> (HackrfTxSink, EventLog, WorkerSignals)
where
    AfterPublish: FnOnce() + Send + 'static,
{
    let mut config = valid_config();
    config.prefill_blocks = 0;
    let events = EventLog::default();
    let scenario = Scenario {
        stop_failures: options.stop_failures,
        ..Scenario::default()
    };
    let mut activation =
        fake_activation_guard(&config, scenario, events.clone());
    if let Err(failure) = activation.activate() {
        panic!("synthetic activation failed: {}", failure.error);
    }

    let (sender, receiver) = mpsc::sync_channel(config.queue_blocks);
    let (result, result_publisher) = WriterResultMailbox::new();
    let (published_sender, published) = mpsc::channel();
    let (disconnected_sender, disconnected) = mpsc::channel();
    let observations = Arc::new(Mutex::new(Vec::new()));
    let observer_observations = observations.clone();
    let observer: WorkerObserver =
        Arc::new(move |observation| match observer_observations.lock() {
            Ok(mut observations) => observations.push(observation),
            Err(poisoned) => poisoned.into_inner().push(observation),
        });
    let handle = thread::spawn(move || {
        result_publisher.publish(WriterTerminal::Failed(writer_error()));
        if published_sender.send(()).is_err() {
            tracing::debug!("writer-result publication observer dropped");
        }
        after_publish();
        drop(receiver);
        if disconnected_sender.send(()).is_err() {
            tracing::debug!("writer disconnect observer dropped");
        }
    });
    let worker = WriterWorker::new(
        sender,
        result,
        handle,
        CancellationToken::default(),
        options.external_cancellation,
        options.policy,
        Some(observer),
    );

    (
        HackrfTxSink {
            config,
            expected_i16_len: 4,
            worker,
            activation,
            startup_state: StartupState::Active,
            finalization_failed: false,
            prefill: VecDeque::new(),
            buffered_blocks: 0,
            buffer_observer: None,
        },
        events,
        WorkerSignals {
            published,
            disconnected,
            observations,
        },
    )
}

pub(super) fn build_racing_failure_sink(
    trigger: WorkerObservation, disconnect_before_release: bool,
) -> (HackrfTxSink, WorkerSignals) {
    let mut config = valid_config();
    config.prefill_blocks = 0;
    let events = EventLog::default();
    let mut activation =
        fake_activation_guard(&config, Scenario::default(), events);
    if let Err(failure) = activation.activate() {
        panic!("synthetic activation failed: {}", failure.error);
    }

    let (sender, receiver) = mpsc::sync_channel(config.queue_blocks);
    let (result, result_publisher) = WriterResultMailbox::new();
    let (trigger_sender, trigger_receiver) = mpsc::channel();
    let rendezvous = Arc::new(Barrier::new(2));
    let writer_rendezvous = rendezvous.clone();
    let (published_sender, published) = mpsc::channel();
    let (disconnected_sender, disconnected) = mpsc::channel();
    let observations = Arc::new(Mutex::new(Vec::new()));
    let observer_observations = observations.clone();
    let observer: WorkerObserver = Arc::new(move |observation| {
        match observer_observations.lock() {
            Ok(mut observations) => observations.push(observation),
            Err(poisoned) => poisoned.into_inner().push(observation),
        }
        if observation == trigger {
            assert!(
                trigger_sender.send(()).is_ok(),
                "writer race trigger receiver dropped"
            );
            rendezvous.wait();
        }
    });
    let handle = thread::spawn(move || {
        if trigger_receiver.recv().is_err() {
            return;
        }
        result_publisher.publish(WriterTerminal::Failed(writer_error()));
        if published_sender.send(()).is_err() {
            tracing::debug!("writer-result publication observer dropped");
        }
        if disconnect_before_release {
            drop(receiver);
            if disconnected_sender.send(()).is_err() {
                tracing::debug!("writer disconnect observer dropped");
            }
            writer_rendezvous.wait();
        } else {
            writer_rendezvous.wait();
            drop(receiver);
            if disconnected_sender.send(()).is_err() {
                tracing::debug!("writer disconnect observer dropped");
            }
        }
    });
    let worker = WriterWorker::new(
        sender,
        result,
        handle,
        CancellationToken::default(),
        None,
        ShutdownPolicy::production(),
        Some(observer),
    );

    (
        HackrfTxSink {
            config,
            expected_i16_len: 4,
            worker,
            activation,
            startup_state: StartupState::Active,
            finalization_failed: false,
            prefill: VecDeque::new(),
            buffered_blocks: 0,
            buffer_observer: None,
        },
        WorkerSignals {
            published,
            disconnected,
            observations,
        },
    )
}

pub(super) fn writer_error() -> Error {
    Error::tx_backend_with_source(
        "hackrf",
        config_context(&valid_config()),
        io::Error::new(io::ErrorKind::ConnectionReset, WRITER_FAILURE_MESSAGE),
    )
}

pub(super) fn assert_writer_error(error: &Error) {
    let Error::TxBackendWithSource {
        backend,
        context,
        source,
    } = error
    else {
        panic!("expected contextual writer root, got {error}");
    };
    assert_eq!(*backend, "hackrf");
    assert!(context.contains("rf_freq_hz=1575420000"), "{context}");
    let Some(source) = source.downcast_ref::<io::Error>() else {
        panic!("writer root did not retain std::io::Error source");
    };
    assert_eq!(source.kind(), io::ErrorKind::ConnectionReset);
    assert_eq!(source.to_string(), WRITER_FAILURE_MESSAGE);
}

pub(super) fn assert_standard_source_is_writer(error: &Error) {
    let Some(source) = StdError::source(error) else {
        panic!("combined error has no standard source");
    };
    let Some(source) = StdError::source(source) else {
        panic!("writer root has no I/O source");
    };
    let Some(source) = source.downcast_ref::<io::Error>() else {
        panic!("standard source is not the writer I/O error");
    };
    assert_eq!(source.kind(), io::ErrorKind::ConnectionReset);
    assert_eq!(source.to_string(), WRITER_FAILURE_MESSAGE);
}
