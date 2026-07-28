//! `HackRF` transmission backend.

use std::{
    collections::VecDeque,
    io::Write,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

use gps::{IqBlockSizing, pack_bits8_into};
use libhackrf::hackrf::HackRF;

use super::TxSink;
use crate::Error;

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

/// A TX sink that streams SC8 samples to a `HackRF` device.
pub struct HackrfTxSink {
    /// Backend configuration.
    config: HackrfTxConfig,
    /// Expected interleaved i16 block length.
    expected_i16_len: usize,

    /// Producer channel used to feed the writer thread.
    sender: Option<mpsc::SyncSender<Vec<u8>>>,
    /// Writer thread handle.
    writer_thread: Option<thread::JoinHandle<Result<(), Error>>>,
    /// Device handle used for best-effort stop on shutdown.
    hackrf: Option<HackRF>,
}

impl HackrfTxSink {
    /// Creates a new `HackRF` TX sink.
    pub fn new(
        config: HackrfTxConfig, expected_i16_len: usize,
    ) -> Result<Self, Error> {
        if expected_i16_len == 0 || !expected_i16_len.is_multiple_of(2) {
            return Err(Error::tx_backend_msg(
                "hackrf",
                format!(
                    "invalid expected_i16_len={expected_i16_len} (must be \
                     non-zero and even)"
                ),
            ));
        }
        IqBlockSizing::from_interleaved_i16_len(expected_i16_len)?;

        let mut hackrf = if let Some(serial) = &config.serial {
            HackRF::new(serial).map_err(|err| {
                Error::tx_backend_with_source(
                    "hackrf",
                    open_context(&config),
                    err,
                )
            })?
        } else {
            HackRF::new_auto().map_err(|err| {
                Error::tx_backend_with_source(
                    "hackrf",
                    open_context(&config),
                    err,
                )
            })?
        };

        hackrf.set_freq(config.rf_freq_hz).map_err(|err| {
            Error::tx_backend_with_source(
                "hackrf",
                config_context(&config),
                err,
            )
        })?;
        hackrf
            .set_sample_rate_auto(config.sample_frequency_hz)
            .map_err(|err| {
                Error::tx_backend_with_source(
                    "hackrf",
                    config_context(&config),
                    err,
                )
            })?;
        hackrf.set_amp_enable(config.amp_enable).map_err(|err| {
            Error::tx_backend_with_source(
                "hackrf",
                config_context(&config),
                err,
            )
        })?;
        hackrf.set_txvga_gain(config.txvga_gain).map_err(|err| {
            Error::tx_backend_with_source(
                "hackrf",
                config_context(&config),
                err,
            )
        })?;
        hackrf.enter_tx_mode().map_err(|err| {
            Error::tx_backend_with_source(
                "hackrf",
                config_context(&config),
                err,
            )
        })?;

        let endpoint = hackrf.tx_queue().map_err(|err| {
            Error::tx_backend_with_source(
                "hackrf",
                config_context(&config),
                err,
            )
        })?;

        let writer = endpoint
            .writer(config.usb_transfer_bytes)
            .with_num_transfers(config.usb_transfers);

        let (sender, receiver) =
            mpsc::sync_channel::<Vec<u8>>(config.queue_blocks);
        let writer_thread = thread::Builder::new()
            .name("hackrf-tx".to_string())
            .spawn({
                let config = config.clone();
                move || writer_thread_main(config, receiver, writer)
            })
            .map_err(|err| {
                Error::tx_backend_with_source(
                    "hackrf",
                    config_context(&config),
                    err,
                )
            })?;

        Ok(Self {
            config,
            expected_i16_len,
            sender: Some(sender),
            writer_thread: Some(writer_thread),
            hackrf: Some(hackrf),
        })
    }

    /// Returns formatted context used in backend error messages.
    fn backend_context(&self) -> String {
        config_context(&self.config)
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
            return Err(Error::tx_backend_msg(
                self.backend(),
                format!(
                    "IQ block length invalid: got {} i16, maximum {} i16 ({})",
                    interleaved_iq_i16.len(),
                    self.expected_i16_len,
                    self.backend_context(),
                ),
            ));
        }

        let mut out = vec![0u8; interleaved_iq_i16.len()];
        pack_bits8_into(interleaved_iq_i16, &mut out).map_err(|err| {
            Error::tx_backend_with_source(
                self.backend(),
                self.backend_context(),
                err,
            )
        })?;

        let Some(sender) = &self.sender else {
            return Err(Error::tx_backend_msg(
                self.backend(),
                "attempted to write after finish".to_string(),
            ));
        };

        sender.send(out).map_err(|err| {
            Error::tx_backend_with_source(
                self.backend(),
                self.backend_context(),
                err,
            )
        })
    }

    fn finish(&mut self) -> Result<(), Error> {
        self.sender.take();

        if let Some(handle) = self.writer_thread.take() {
            match handle.join() {
                Ok(Ok(())) => {}
                Ok(Err(err)) => {
                    self.stop_tx_best_effort();
                    return Err(err);
                }
                Err(_) => {
                    self.stop_tx_best_effort();
                    return Err(Error::tx_backend_msg(
                        self.backend(),
                        "writer thread panicked".to_string(),
                    ));
                }
            }
        }

        if let Some(mut hackrf) = self.hackrf.take() {
            tracing::info!(context = %self.backend_context(), "stopping hackrf tx");
            hackrf.stop_tx().map_err(|err| {
                Error::tx_backend_with_source(
                    self.backend(),
                    self.backend_context(),
                    err,
                )
            })?;
            tracing::info!(context = %self.backend_context(), "hackrf tx stopped");
        }
        Ok(())
    }
}

impl HackrfTxSink {
    /// Attempts to stop TX, logging on failure.
    fn stop_tx_best_effort(&mut self) {
        if let Some(hackrf) = self.hackrf.as_mut()
            && let Err(err) = hackrf.stop_tx()
        {
            tracing::warn!(error = %err, "failed to stop hackrf tx");
        }
    }
}

impl Drop for HackrfTxSink {
    fn drop(&mut self) {
        if let Err(err) = self.finish() {
            tracing::warn!(error = %err, "HackrfTxSink dropped with error");
        }
    }
}

/// Main loop for the writer thread.
fn writer_thread_main(
    config: HackrfTxConfig, receiver: mpsc::Receiver<Vec<u8>>,
    mut writer: impl Write,
) -> Result<(), Error> {
    let mut buffered: VecDeque<Vec<u8>> = VecDeque::new();
    for _ in 0..config.prefill_blocks {
        match receiver.recv() {
            Ok(block) => buffered.push_back(block),
            Err(_) => break,
        }
    }

    let mut silence: Vec<u8> = Vec::new();
    let underrun_timeout = Duration::from_secs_f64(
        (config.step_duration.as_secs_f64() / 2.0).max(0.001),
    );

    let mut bytes_sent_since_log: u64 = 0;
    let mut bytes_sent_total: u64 = 0;
    let mut underruns: u64 = 0;
    let mut last_log = Instant::now();
    let mut last_log_total = Instant::now();
    let log_interval = Duration::from_secs(1);

    loop {
        let next = if let Some(block) = buffered.pop_front() {
            Some(block)
        } else {
            match receiver.recv_timeout(underrun_timeout) {
                Ok(block) => Some(block),
                Err(mpsc::RecvTimeoutError::Timeout) => None,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        };

        if let Some(block) = next {
            if silence.is_empty() {
                silence.resize(block.len(), 0);
            }
            writer.write_all(&block).map_err(|err| {
                Error::tx_backend_with_source(
                    "hackrf",
                    config_context(&config),
                    err,
                )
            })?;
            bytes_sent_since_log += block.len() as u64;
            bytes_sent_total += block.len() as u64;
        } else {
            underruns = underruns.wrapping_add(1);
            if let Some(counter) = &config.underrun_counter {
                counter.fetch_add(1, Ordering::Relaxed);
            }

            if config.silence_on_underrun {
                if silence.is_empty() {
                    // Can't fill until we know the block size; keep waiting.
                    continue;
                }
                writer.write_all(&silence).map_err(|err| {
                    Error::tx_backend_with_source(
                        "hackrf",
                        config_context(&config),
                        err,
                    )
                })?;
                bytes_sent_since_log += silence.len() as u64;
                bytes_sent_total += silence.len() as u64;
            }
        }

        if last_log.elapsed() >= log_interval {
            let elapsed = last_log_total.elapsed().as_secs_f64().max(1e-6);
            let mbps =
                (bytes_sent_since_log as f64 / elapsed) / (1024.0 * 1024.0);
            tracing::info!(
                mbps = %format_args!("{mbps:.2}"),
                underruns,
                bytes_sent_total,
                "hackrf tx"
            );
            bytes_sent_since_log = 0;
            last_log = Instant::now();
            last_log_total = Instant::now();
        }
    }

    writer.flush().map_err(|err| {
        Error::tx_backend_with_source("hackrf", config_context(&config), err)
    })?;
    Ok(())
}

/// Extra context for device open errors (including Windows driver hints).
fn open_context(config: &HackrfTxConfig) -> String {
    let mut base = config_context(config);
    if cfg!(windows) {
        base.push_str(
            " | Windows tip: bind HackRF to WinUSB (Zadig) so nusb can open it",
        );
    }
    base
}

/// Formats backend configuration for logging/error context.
fn config_context(config: &HackrfTxConfig) -> String {
    format!(
        "serial={} rf_freq_hz={} sample_frequency_hz={} txvga_gain={} \
         amp_enable={} usb_transfer_bytes={} usb_transfers={} queue_blocks={} \
         prefill_blocks={} silence_on_underrun={}",
        config.serial.as_deref().unwrap_or("auto"),
        config.rf_freq_hz,
        config.sample_frequency_hz,
        config.txvga_gain,
        config.amp_enable,
        config.usb_transfer_bytes,
        config.usb_transfers,
        config.queue_blocks,
        config.prefill_blocks,
        config.silence_on_underrun,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "Requires HackRF hardware"]
    fn hackrf_tx_smoke() -> Result<(), Error> {
        let config = HackrfTxConfig {
            serial: None,
            rf_freq_hz: 1_575_420_000,
            sample_frequency_hz: 2_600_000.0,
            step_duration: Duration::from_millis(100),
            txvga_gain: 20,
            amp_enable: false,
            usb_transfer_bytes: 256 * 1024,
            usb_transfers: 16,
            queue_blocks: 8,
            prefill_blocks: 2,
            silence_on_underrun: true,
            underrun_counter: None,
        };

        let sizing = IqBlockSizing::new(1024)?;
        let mut sink = HackrfTxSink::new(config, sizing.interleaved_i16_len())?;
        let block = vec![0i16; sizing.interleaved_i16_len()];
        for _ in 0..10 {
            sink.write_block_i16(&block)?;
        }
        sink.finish()?;
        Ok(())
    }
}
