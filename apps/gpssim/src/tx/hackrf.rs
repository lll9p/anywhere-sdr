//! `HackRF` transmission backend.

use std::{
    collections::VecDeque,
    io::{self, Write},
    sync::{Arc, atomic::AtomicU64, mpsc},
    thread,
    time::Duration,
};

use gps::pack_bits8_into;

use super::TxSink;
use crate::Error;

mod startup;
mod writer;

#[cfg(test)]
mod hardware_tests;
#[cfg(test)]
mod lifecycle_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod validation_tests;

use startup::{
    ActivationGuard, HackrfDeviceControl, open_real_device, validate_config,
};
use writer::writer_thread_main;

#[derive(Clone, Debug)]
/// Configuration for `HackRF` TX.
pub struct HackrfTxConfig {
    /// Optional `HackRF` serial number (hex). When `None`, uses the first
    /// available device.
    pub serial: Option<String>,
    /// RF center frequency in Hz.
    pub rf_freq_hz: u64,
    /// Baseband sample rate in Hz.
    pub sample_frequency_hz: f64,
    /// Duration represented by each generator-produced block.
    pub step_duration: Duration,

    /// TXVGA gain (0..=47).
    pub txvga_gain: u16,
    /// Enable the `HackRF` RF amplifier.
    pub amp_enable: bool,

    /// USB bulk transfer size (bytes).
    pub usb_transfer_bytes: usize,
    /// Number of in-flight USB transfers.
    pub usb_transfers: usize,

    /// Bounded queue depth in generator blocks.
    pub queue_blocks: usize,
    /// Number of blocks to prefill before continuous TX.
    pub prefill_blocks: usize,
    /// If `true`, transmit silence (zeros) on underrun.
    pub silence_on_underrun: bool,

    /// Shared underrun counter incremented by the writer thread.
    pub underrun_counter: Option<Arc<AtomicU64>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StartupState {
    Prepared,
    Active,
    Failed,
    Finished,
}

type BufferObserver = Arc<dyn Fn(usize) + Send + Sync>;
type WriterThread = thread::JoinHandle<Result<(), Error>>;

/// A TX sink that streams SC8 samples to a `HackRF` device.
pub struct HackrfTxSink {
    config: HackrfTxConfig,
    expected_i16_len: usize,
    sender: Option<mpsc::SyncSender<Vec<u8>>>,
    writer_thread: Option<WriterThread>,
    activation: ActivationGuard,
    startup_state: StartupState,
    prefill: VecDeque<Vec<u8>>,
    buffered_blocks: usize,
    buffer_observer: Option<BufferObserver>,
}

impl HackrfTxSink {
    /// Creates a new `HackRF` TX sink.
    pub fn new(
        config: HackrfTxConfig, expected_i16_len: usize,
    ) -> Result<Self, Error> {
        Self::new_with(
            config,
            expected_i16_len,
            open_real_device,
            spawn_writer_thread,
            None,
        )
    }

    fn new_with<Open, Spawn>(
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
        )?;

        let (sender, receiver) =
            mpsc::sync_channel::<Vec<u8>>(config.queue_blocks);
        let writer_thread =
            spawn(config.clone(), receiver, writer).map_err(|error| {
                Error::tx_backend_with_source(
                    "hackrf",
                    config_context(&config),
                    error,
                )
            })?;

        Ok(Self {
            config,
            expected_i16_len,
            sender: Some(sender),
            writer_thread: Some(writer_thread),
            activation,
            startup_state: StartupState::Prepared,
            prefill: VecDeque::new(),
            buffered_blocks: 0,
            buffer_observer,
        })
    }

    fn backend_context(&self) -> String {
        config_context(&self.config)
    }

    fn buffer_until_ready(&mut self, block: Vec<u8>) -> Result<(), Error> {
        self.prefill.push_back(block);
        self.buffered_blocks = self.buffered_blocks.wrapping_add(1);
        if let Some(observer) = &self.buffer_observer {
            observer(self.buffered_blocks);
        }

        if self.prefill.len() >= self.config.prefill_blocks.max(1) {
            self.activate_and_dispatch_prefill()?;
        }
        Ok(())
    }

    fn activate_and_dispatch_prefill(&mut self) -> Result<(), Error> {
        if self.startup_state != StartupState::Prepared {
            return Err(Error::tx_backend_msg(
                self.backend(),
                format!(
                    "invalid startup activation state {:?} ({})",
                    self.startup_state,
                    self.backend_context()
                ),
            ));
        }

        // Make the activation attempt terminal before external mutation so
        // unwinding cannot retry it from `finish` or `Drop`.
        self.startup_state = StartupState::Failed;
        if let Err(error) = self.activation.activate() {
            self.prefill.clear();
            return Err(error);
        }
        self.startup_state = StartupState::Active;

        while let Some(block) = self.prefill.pop_front() {
            if let Err(error) = self.send_to_writer(block) {
                self.enter_failed_state();
                self.activation.rollback_best_effort();
                return Err(error);
            }
        }
        Ok(())
    }

    fn send_to_writer(&self, block: Vec<u8>) -> Result<(), Error> {
        let Some(sender) = &self.sender else {
            return Err(Error::tx_backend_msg(
                self.backend(),
                format!(
                    "writer channel is closed ({})",
                    self.backend_context()
                ),
            ));
        };
        sender.send(block).map_err(|error| {
            Error::tx_backend_with_source(
                self.backend(),
                self.backend_context(),
                error,
            )
        })
    }

    fn enter_failed_state(&mut self) {
        self.startup_state = StartupState::Failed;
        self.prefill.clear();
    }

    fn fail_block_write(&mut self) {
        match self.startup_state {
            StartupState::Prepared => self.enter_failed_state(),
            StartupState::Active => {
                self.enter_failed_state();
                self.activation.rollback_best_effort();
            }
            StartupState::Failed | StartupState::Finished => {}
        }
    }

    fn join_writer(&mut self, primary: &mut Option<Error>) {
        let Some(handle) = self.writer_thread.take() else {
            return;
        };
        let writer_error = match handle.join() {
            Ok(Ok(())) => None,
            Ok(Err(error)) => Some(error),
            Err(_) => Some(Error::tx_backend_msg(
                self.backend(),
                "writer thread panicked".to_string(),
            )),
        };

        if let Some(error) = writer_error {
            if primary.is_none() {
                *primary = Some(error);
            } else {
                tracing::warn!(
                    error = %error,
                    context = %self.backend_context(),
                    "hackrf writer finalization also failed"
                );
            }
        }
    }
}

impl TxSink for HackrfTxSink {
    fn backend(&self) -> &'static str {
        "hackrf"
    }

    fn write_block_i16(
        &mut self, interleaved_iq_i16: &[i16],
    ) -> Result<(), Error> {
        if interleaved_iq_i16.len() > self.expected_i16_len
            || !interleaved_iq_i16.len().is_multiple_of(2)
        {
            let error = Error::tx_backend_msg(
                self.backend(),
                format!(
                    "IQ block length invalid: got {} i16, maximum {} i16 ({})",
                    interleaved_iq_i16.len(),
                    self.expected_i16_len,
                    self.backend_context(),
                ),
            );
            self.fail_block_write();
            return Err(error);
        }

        let mut out = vec![0_u8; interleaved_iq_i16.len()];
        if let Err(source) = pack_bits8_into(interleaved_iq_i16, &mut out) {
            let error = Error::tx_backend_with_source(
                self.backend(),
                self.backend_context(),
                source,
            );
            self.fail_block_write();
            return Err(error);
        }

        match self.startup_state {
            StartupState::Prepared => self.buffer_until_ready(out),
            StartupState::Active => {
                if let Err(error) = self.send_to_writer(out) {
                    self.enter_failed_state();
                    self.activation.rollback_best_effort();
                    return Err(error);
                }
                Ok(())
            }
            StartupState::Failed => Err(Error::tx_backend_msg(
                self.backend(),
                format!(
                    "attempted to write after HackRF startup/stream failure \
                     ({})",
                    self.backend_context()
                ),
            )),
            StartupState::Finished => Err(Error::tx_backend_msg(
                self.backend(),
                "attempted to write after finish".to_string(),
            )),
        }
    }

    fn finish(&mut self) -> Result<(), Error> {
        if self.startup_state == StartupState::Finished {
            return Ok(());
        }

        let mut primary = None;
        if self.startup_state == StartupState::Prepared
            && !self.prefill.is_empty()
        {
            if let Err(error) = self.activate_and_dispatch_prefill() {
                primary = Some(error);
            }
        } else if self.startup_state == StartupState::Failed {
            self.prefill.clear();
        }

        self.sender.take();
        self.join_writer(&mut primary);

        if let Some(error) = primary {
            self.enter_failed_state();
            self.activation.rollback_best_effort();
            return Err(error);
        }

        match self.startup_state {
            StartupState::Active => {
                tracing::info!(
                    context = %self.backend_context(),
                    "stopping hackrf tx"
                );
                if let Err(error) = self.activation.stop() {
                    self.enter_failed_state();
                    return Err(error);
                }
                tracing::info!(
                    context = %self.backend_context(),
                    "hackrf tx stopped"
                );
                self.startup_state = StartupState::Finished;
            }
            StartupState::Prepared => {
                self.startup_state = StartupState::Finished;
            }
            StartupState::Failed => {
                self.activation.rollback_best_effort();
            }
            StartupState::Finished => {}
        }
        Ok(())
    }
}

impl Drop for HackrfTxSink {
    fn drop(&mut self) {
        if let Err(error) = self.finish() {
            tracing::warn!(error = %error, "HackrfTxSink dropped with error");
        }
    }
}

fn spawn_writer_thread(
    config: HackrfTxConfig, receiver: mpsc::Receiver<Vec<u8>>,
    writer: Box<dyn Write + Send>,
) -> io::Result<WriterThread> {
    thread::Builder::new()
        .name("hackrf-tx".to_string())
        .spawn(move || writer_thread_main(config, receiver, writer))
}

fn open_context(config: &HackrfTxConfig) -> String {
    let mut base = config_context(config);
    if cfg!(windows) {
        base.push_str(
            " | Windows tip: bind HackRF to WinUSB (Zadig) so nusb can open it",
        );
    }
    base
}

fn config_context(config: &HackrfTxConfig) -> String {
    format!(
        "serial={} rf_freq_hz={} sample_frequency_hz={} step_duration={:?} \
         txvga_gain={} amp_enable={} usb_transfer_bytes={} usb_transfers={} \
         queue_blocks={} prefill_blocks={} silence_on_underrun={}",
        config.serial.as_deref().unwrap_or("auto"),
        config.rf_freq_hz,
        config.sample_frequency_hz,
        config.step_duration,
        config.txvga_gain,
        config.amp_enable,
        config.usb_transfer_bytes,
        config.usb_transfers,
        config.queue_blocks,
        config.prefill_blocks,
        config.silence_on_underrun,
    )
}
