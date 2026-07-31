use std::{
    collections::VecDeque,
    io::{self, Write},
    sync::{Arc, atomic::AtomicBool, mpsc},
    thread,
    time::Duration,
};

use gps::IqBlockSizing;
use libhackrf::hackrf::HackRF;

use super::{
    BufferObserver, HackrfTxConfig, HackrfTxSink, StartupState, config_context,
    open_context,
    shutdown::{
        CancellationToken, ShutdownPolicy, WorkerObserver, WriterResultMailbox,
        WriterResultPublisher, WriterTerminal, WriterWorker,
    },
    writer::{WriterOutcome, writer_thread_main},
};
use crate::Error;

const RF_MIN_HZ: u64 = 1_000_000;
const RF_MAX_HZ: u64 = 6_000_000_000;
const SAMPLE_RATE_MIN_HZ: f64 = 2_000_000.0;
const SAMPLE_RATE_MAX_HZ: f64 = 20_000_000.0;
const TXVGA_GAIN_MAX: u16 = 47;

type WriterThread = thread::JoinHandle<()>;

pub(super) struct ConstructionOptions {
    pub(super) buffer_observer: Option<BufferObserver>,
    pub(super) external_cancellation: Option<Arc<AtomicBool>>,
    pub(super) policy: ShutdownPolicy,
    pub(super) worker_observer: Option<WorkerObserver>,
}

impl ConstructionOptions {
    pub(super) fn production() -> Self {
        Self {
            buffer_observer: None,
            external_cancellation: None,
            policy: ShutdownPolicy::production(),
            worker_observer: None,
        }
    }
}

pub(super) trait HackrfDeviceControl: Send {
    fn set_freq(&mut self, frequency_hz: u64) -> Result<(), Error>;
    fn set_sample_rate_auto(&mut self, frequency_hz: f64) -> Result<(), Error>;
    fn set_amp_enable(&mut self, enabled: bool) -> Result<(), Error>;
    fn set_txvga_gain(&mut self, gain: u16) -> Result<(), Error>;
    fn prepare_tx_writer(
        &mut self, transfer_bytes: usize, transfers: usize,
        write_timeout: Duration,
    ) -> Result<Box<dyn Write + Send>, Error>;
    fn enter_tx_mode(&mut self) -> Result<(), Error>;
    fn stop_tx(&mut self) -> Result<(), Error>;
}

struct RealHackrfDevice {
    hackrf: HackRF,
    context: String,
}

impl RealHackrfDevice {
    fn map_error(&self, error: libhackrf::error::Error) -> Error {
        Error::tx_backend_with_source("hackrf", self.context.clone(), error)
    }
}

impl HackrfDeviceControl for RealHackrfDevice {
    fn set_freq(&mut self, frequency_hz: u64) -> Result<(), Error> {
        self.hackrf
            .set_freq(frequency_hz)
            .map_err(|error| self.map_error(error))
    }

    fn set_sample_rate_auto(&mut self, frequency_hz: f64) -> Result<(), Error> {
        self.hackrf
            .set_sample_rate_auto(frequency_hz)
            .map_err(|error| self.map_error(error))
    }

    fn set_amp_enable(&mut self, enabled: bool) -> Result<(), Error> {
        self.hackrf
            .set_amp_enable(enabled)
            .map_err(|error| self.map_error(error))
    }

    fn set_txvga_gain(&mut self, gain: u16) -> Result<(), Error> {
        self.hackrf
            .set_txvga_gain(gain)
            .map_err(|error| self.map_error(error))
    }

    fn prepare_tx_writer(
        &mut self, transfer_bytes: usize, transfers: usize,
        write_timeout: Duration,
    ) -> Result<Box<dyn Write + Send>, Error> {
        let endpoint = self
            .hackrf
            .tx_queue()
            .map_err(|error| self.map_error(error))?;
        Ok(Box::new(
            endpoint
                .writer(transfer_bytes)
                .with_num_transfers(transfers)
                .with_write_timeout(write_timeout),
        ))
    }

    fn enter_tx_mode(&mut self) -> Result<(), Error> {
        self.hackrf
            .enter_tx_mode()
            .map_err(|error| self.map_error(error))
    }

    fn stop_tx(&mut self) -> Result<(), Error> {
        self.hackrf.stop_tx().map_err(|error| self.map_error(error))
    }
}

pub(super) fn open_real_device(
    config: &HackrfTxConfig,
) -> Result<Box<dyn HackrfDeviceControl>, Error> {
    let hackrf = if let Some(serial) = &config.serial {
        HackRF::new(serial).map_err(|error| {
            Error::tx_backend_with_source("hackrf", open_context(config), error)
        })?
    } else {
        HackRF::new_auto().map_err(|error| {
            Error::tx_backend_with_source("hackrf", open_context(config), error)
        })?
    };

    Ok(Box::new(RealHackrfDevice {
        hackrf,
        context: config_context(config),
    }))
}

impl HackrfTxSink {
    /// Creates a new `HackRF` TX sink.
    pub fn new(
        config: HackrfTxConfig, expected_i16_len: usize,
    ) -> Result<Self, Error> {
        Self::new_with_options(
            config,
            expected_i16_len,
            open_real_device,
            spawn_writer_thread,
            ConstructionOptions::production(),
        )
    }

    pub(crate) fn new_with_cancellation(
        config: HackrfTxConfig, expected_i16_len: usize,
        external_cancellation: Arc<AtomicBool>,
    ) -> Result<Self, Error> {
        let mut options = ConstructionOptions::production();
        options.external_cancellation = Some(external_cancellation);
        Self::new_with_options(
            config,
            expected_i16_len,
            open_real_device,
            spawn_writer_thread,
            options,
        )
    }

    #[cfg(test)]
    pub(super) fn new_with<Open, Spawn>(
        config: HackrfTxConfig, expected_i16_len: usize, open: Open,
        spawn: Spawn, buffer_observer: Option<BufferObserver>,
    ) -> Result<Self, Error>
    where
        Open: FnOnce(
            &HackrfTxConfig,
        ) -> Result<Box<dyn HackrfDeviceControl>, Error>,
        Spawn: FnOnce(
            HackrfTxConfig,
            mpsc::Receiver<Vec<u8>>,
            Box<dyn Write + Send>,
            CancellationToken,
            WriterResultPublisher,
            Duration,
        ) -> io::Result<WriterThread>,
    {
        let mut options = ConstructionOptions::production();
        options.buffer_observer = buffer_observer;
        Self::new_with_options(config, expected_i16_len, open, spawn, options)
    }

    pub(super) fn new_with_options<Open, Spawn>(
        config: HackrfTxConfig, expected_i16_len: usize, open: Open,
        spawn: Spawn, options: ConstructionOptions,
    ) -> Result<Self, Error>
    where
        Open: FnOnce(
            &HackrfTxConfig,
        ) -> Result<Box<dyn HackrfDeviceControl>, Error>,
        Spawn: FnOnce(
            HackrfTxConfig,
            mpsc::Receiver<Vec<u8>>,
            Box<dyn Write + Send>,
            CancellationToken,
            WriterResultPublisher,
            Duration,
        ) -> io::Result<WriterThread>,
    {
        validate_config(&config, expected_i16_len)?;

        let device = open(&config)?;
        let mut activation =
            ActivationGuard::new(device, config_context(&config));
        activation.device_mut().set_freq(config.rf_freq_hz)?;
        activation
            .device_mut()
            .set_sample_rate_auto(config.sample_frequency_hz)?;
        activation.device_mut().set_amp_enable(config.amp_enable)?;
        activation.device_mut().set_txvga_gain(config.txvga_gain)?;
        let writer = activation.device_mut().prepare_tx_writer(
            config.usb_transfer_bytes,
            config.usb_transfers,
            options.policy.bulk_write_timeout,
        )?;

        let (sender, receiver) =
            mpsc::sync_channel::<Vec<u8>>(config.queue_blocks);
        let (result, result_publisher) = WriterResultMailbox::new();
        let cancellation = CancellationToken::default();
        let writer_thread = spawn(
            config.clone(),
            receiver,
            writer,
            cancellation.clone(),
            result_publisher,
            options.policy.enqueue_poll,
        )
        .map_err(|error| {
            Error::tx_backend_with_source(
                "hackrf",
                config_context(&config),
                error,
            )
        })?;
        let worker = WriterWorker::new(
            sender,
            result,
            writer_thread,
            cancellation,
            options.external_cancellation,
            options.policy,
            options.worker_observer,
        );

        Ok(Self {
            config,
            expected_i16_len,
            worker,
            activation,
            startup_state: StartupState::Prepared,
            finalization_failed: false,
            prefill: VecDeque::new(),
            buffered_blocks: 0,
            buffer_observer: options.buffer_observer,
        })
    }
}

fn spawn_writer_thread(
    config: HackrfTxConfig, receiver: mpsc::Receiver<Vec<u8>>,
    writer: Box<dyn Write + Send>, cancellation: CancellationToken,
    result_publisher: WriterResultPublisher, cancellation_poll: Duration,
) -> io::Result<WriterThread> {
    thread::Builder::new()
        .name("hackrf-tx".to_string())
        .spawn(move || {
            let terminal = match writer_thread_main(
                config,
                &receiver,
                writer,
                cancellation,
                cancellation_poll,
            ) {
                Ok(WriterOutcome::Completed) => WriterTerminal::Completed,
                Ok(WriterOutcome::Cancelled) => WriterTerminal::Cancelled,
                Err(error) => WriterTerminal::Failed(error),
            };
            result_publisher.publish(terminal);
            drop(receiver);
        })
}

pub(super) fn validate_config(
    config: &HackrfTxConfig, expected_i16_len: usize,
) -> Result<(), Error> {
    let context = config_context(config);
    if expected_i16_len == 0 || !expected_i16_len.is_multiple_of(2) {
        return Err(invalid_config(
            format!(
                "expected_i16_len={expected_i16_len} (must be non-zero and \
                 even)"
            ),
            &context,
        ));
    }
    IqBlockSizing::from_interleaved_i16_len(expected_i16_len).map_err(
        |error| {
            Error::tx_backend_with_source(
                "hackrf",
                format!("expected_i16_len={expected_i16_len} | {context}"),
                error,
            )
        },
    )?;

    if !(RF_MIN_HZ..=RF_MAX_HZ).contains(&config.rf_freq_hz) {
        return Err(invalid_config(
            format!(
                "rf_freq_hz={} (must be {RF_MIN_HZ}..={RF_MAX_HZ})",
                config.rf_freq_hz
            ),
            &context,
        ));
    }
    if !config.sample_frequency_hz.is_finite()
        || !(SAMPLE_RATE_MIN_HZ..=SAMPLE_RATE_MAX_HZ)
            .contains(&config.sample_frequency_hz)
    {
        return Err(invalid_config(
            format!(
                "sample_frequency_hz={} (must be finite and \
                 {SAMPLE_RATE_MIN_HZ}..={SAMPLE_RATE_MAX_HZ})",
                config.sample_frequency_hz
            ),
            &context,
        ));
    }
    if config.step_duration.is_zero() {
        return Err(invalid_config(
            "step_duration must be non-zero".to_string(),
            &context,
        ));
    }
    if config.txvga_gain > TXVGA_GAIN_MAX {
        return Err(invalid_config(
            format!(
                "txvga_gain={} (must be 0..={TXVGA_GAIN_MAX})",
                config.txvga_gain
            ),
            &context,
        ));
    }
    if config.usb_transfer_bytes == 0 {
        return Err(invalid_config(
            "usb_transfer_bytes must be greater than zero".to_string(),
            &context,
        ));
    }
    if config.usb_transfers == 0 {
        return Err(invalid_config(
            "usb_transfers must be greater than zero".to_string(),
            &context,
        ));
    }
    if config.queue_blocks == 0 {
        return Err(invalid_config(
            "queue_blocks must be greater than zero".to_string(),
            &context,
        ));
    }
    Ok(())
}

fn invalid_config(message: String, context: &str) -> Error {
    Error::tx_backend_msg("hackrf", format!("invalid {message} ({context})"))
}

pub(super) struct ActivationFailure {
    pub(super) error: Error,
    pub(super) rollback_attempted: bool,
}

pub(super) struct ActivationGuard {
    device: Box<dyn HackrfDeviceControl>,
    context: String,
    rollback_armed: bool,
}

impl ActivationGuard {
    pub(super) fn new(
        device: Box<dyn HackrfDeviceControl>, context: String,
    ) -> Self {
        Self {
            device,
            context,
            rollback_armed: false,
        }
    }

    pub(super) fn device_mut(&mut self) -> &mut dyn HackrfDeviceControl {
        self.device.as_mut()
    }

    pub(super) fn activate(&mut self) -> Result<(), ActivationFailure> {
        self.rollback_armed = true;
        match self.device.enter_tx_mode() {
            Ok(()) => Ok(()),
            Err(error) => Err(ActivationFailure {
                error,
                rollback_attempted: self.rollback_best_effort(),
            }),
        }
    }

    pub(super) fn stop(&mut self) -> Result<(), Error> {
        if !self.rollback_armed {
            return Ok(());
        }
        self.device.stop_tx()?;
        self.rollback_armed = false;
        Ok(())
    }

    pub(super) fn rollback_best_effort(&mut self) -> bool {
        if !self.rollback_armed {
            return false;
        }
        match self.device.stop_tx() {
            Ok(()) => self.rollback_armed = false,
            Err(error) => {
                tracing::warn!(
                    error = %error,
                    context = %self.context,
                    "failed to return hackrf to Off"
                );
            }
        }
        true
    }

    pub(super) fn is_armed(&self) -> bool {
        self.rollback_armed
    }
}

impl Drop for ActivationGuard {
    fn drop(&mut self) {
        self.rollback_best_effort();
    }
}
