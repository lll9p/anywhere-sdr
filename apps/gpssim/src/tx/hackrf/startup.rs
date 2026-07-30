use std::io::Write;

use gps::IqBlockSizing;
use libhackrf::hackrf::HackRF;

use super::{HackrfTxConfig, config_context, open_context};
use crate::Error;

const RF_MIN_HZ: u64 = 1_000_000;
const RF_MAX_HZ: u64 = 6_000_000_000;
const SAMPLE_RATE_MIN_HZ: f64 = 2_000_000.0;
const SAMPLE_RATE_MAX_HZ: f64 = 20_000_000.0;
const TXVGA_GAIN_MAX: u16 = 47;

pub(super) trait HackrfDeviceControl: Send {
    fn set_freq(&mut self, frequency_hz: u64) -> Result<(), Error>;
    fn set_sample_rate_auto(&mut self, frequency_hz: f64) -> Result<(), Error>;
    fn set_amp_enable(&mut self, enabled: bool) -> Result<(), Error>;
    fn set_txvga_gain(&mut self, gain: u16) -> Result<(), Error>;
    fn prepare_tx_writer(
        &mut self, transfer_bytes: usize, transfers: usize,
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
    ) -> Result<Box<dyn Write + Send>, Error> {
        let endpoint = self
            .hackrf
            .tx_queue()
            .map_err(|error| self.map_error(error))?;
        Ok(Box::new(
            endpoint
                .writer(transfer_bytes)
                .with_num_transfers(transfers),
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

    pub(super) fn activate(&mut self) -> Result<(), Error> {
        self.rollback_armed = true;
        match self.device.enter_tx_mode() {
            Ok(()) => Ok(()),
            Err(primary) => {
                self.rollback_best_effort();
                Err(primary)
            }
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

    pub(super) fn rollback_best_effort(&mut self) {
        if !self.rollback_armed {
            return;
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
    }
}

impl Drop for ActivationGuard {
    fn drop(&mut self) {
        self.rollback_best_effort();
    }
}
