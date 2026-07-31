//! `HackRF` transmission backend.

use std::{
    collections::VecDeque,
    sync::{Arc, atomic::AtomicU64},
    time::{Duration, Instant},
};

use gps::pack_bits8_into;

use super::TxSink;
use crate::Error;

mod shutdown;
mod startup;
mod writer;

#[cfg(test)]
mod drop_tests;
#[cfg(test)]
mod hardware_tests;
#[cfg(test)]
mod lifecycle_tests;
#[cfg(test)]
mod shutdown_test_support;
#[cfg(test)]
mod shutdown_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod validation_tests;
#[cfg(test)]
mod writer_tests;

use shutdown::{EnqueueMode, WriterWorker};
use startup::ActivationGuard;

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

struct DispatchFailure {
    error: Error,
    rollback_attempted: bool,
}

type BufferObserver = Arc<dyn Fn(usize) + Send + Sync>;

/// A TX sink that streams SC8 samples to a `HackRF` device.
pub struct HackrfTxSink {
    config: HackrfTxConfig,
    expected_i16_len: usize,
    worker: WriterWorker,
    activation: ActivationGuard,
    startup_state: StartupState,
    finalization_failed: bool,
    prefill: VecDeque<Vec<u8>>,
    buffered_blocks: usize,
    buffer_observer: Option<BufferObserver>,
}

impl HackrfTxSink {
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
            self.activate_and_dispatch_prefill(EnqueueMode::Streaming)
                .map_err(|failure| failure.error)?;
        }
        Ok(())
    }

    fn activate_and_dispatch_prefill(
        &mut self, mode: EnqueueMode,
    ) -> Result<(), DispatchFailure> {
        if self.startup_state != StartupState::Prepared {
            return Err(DispatchFailure {
                error: Error::tx_backend_msg(
                    self.backend(),
                    format!(
                        "invalid startup activation state {:?} ({})",
                        self.startup_state,
                        self.backend_context()
                    ),
                ),
                rollback_attempted: false,
            });
        }

        self.startup_state = StartupState::Failed;
        if let Err(failure) = self.activation.activate() {
            self.prefill.clear();
            return Err(DispatchFailure {
                error: failure.error,
                rollback_attempted: failure.rollback_attempted,
            });
        }
        self.startup_state = StartupState::Active;

        while let Some(block) = self.prefill.pop_front() {
            let context = self.backend_context();
            if let Err(error) =
                self.worker.send(block, mode, self.backend(), &context)
            {
                self.enter_failed_state();
                let rollback_attempted = if matches!(error, Error::RunCancelled)
                {
                    false
                } else {
                    self.activation.rollback_best_effort()
                };
                return Err(DispatchFailure {
                    error,
                    rollback_attempted,
                });
            }
        }
        Ok(())
    }

    fn send_to_writer(
        &self, block: Vec<u8>, mode: EnqueueMode,
    ) -> Result<(), Error> {
        self.worker
            .send(block, mode, self.backend(), &self.backend_context())
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

    fn record_finalization_error(
        &self, primary: &mut Option<Error>, error: Error, operation: &str,
    ) {
        if primary.is_none() {
            *primary = Some(error);
        } else {
            tracing::warn!(
                error = %error,
                context = %self.backend_context(),
                "hackrf {operation} also failed"
            );
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
                if let Err(error) =
                    self.send_to_writer(out, EnqueueMode::Streaming)
                {
                    self.enter_failed_state();
                    if !matches!(error, Error::RunCancelled) {
                        self.activation.rollback_best_effort();
                    }
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

    fn request_cancel(&mut self) {
        self.worker.request_cancel();
    }

    fn finish(&mut self) -> Result<(), Error> {
        if self.startup_state == StartupState::Finished {
            return Ok(());
        }

        let deadline = Instant::now() + self.worker.policy().finish_timeout;
        let mut primary = None;
        let mut stop_attempted = false;
        let cancellation_requested = self.worker.is_cancel_requested();

        if self.startup_state == StartupState::Prepared {
            if cancellation_requested {
                self.prefill.clear();
            } else if !self.prefill.is_empty() {
                match self.activate_and_dispatch_prefill(
                    EnqueueMode::Finalizing { deadline },
                ) {
                    Ok(()) => {}
                    Err(failure) => {
                        stop_attempted = failure.rollback_attempted;
                        primary = Some(failure.error);
                    }
                }
            }
        } else if self.startup_state == StartupState::Failed {
            self.prefill.clear();
            self.worker.request_cancel();
        }

        if primary.is_some() {
            self.worker.request_cancel();
        }

        let context = self.backend_context();
        if let Err(error) =
            self.worker
                .close_and_wait(deadline, self.backend(), &context)
        {
            self.record_finalization_error(
                &mut primary,
                error,
                "writer finalization",
            );
        }

        if self.activation.is_armed() && !stop_attempted {
            tracing::info!(context = %context, "stopping hackrf tx");
            match self.activation.stop() {
                Ok(()) => {
                    tracing::info!(context = %context, "hackrf tx stopped");
                }
                Err(error) => {
                    self.record_finalization_error(&mut primary, error, "stop");
                }
            }
        }

        if let Some(error) = primary {
            self.finalization_failed = true;
            self.enter_failed_state();
            Err(error)
        } else {
            self.prefill.clear();
            self.startup_state = if self.finalization_failed {
                StartupState::Failed
            } else {
                StartupState::Finished
            };
            Ok(())
        }
    }
}

impl Drop for HackrfTxSink {
    fn drop(&mut self) {
        let context = self.backend_context();
        self.worker.detach_on_drop(&context);
    }
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
