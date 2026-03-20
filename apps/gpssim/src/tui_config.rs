use std::path::PathBuf;

use geometry::{Ecef, Location};

use crate::cli::{Args, TxBackend};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum MotionSource {
    #[default]
    Preconfigured,
    Manual,
}

impl MotionSource {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Preconfigured => "preconfigured",
            Self::Manual => "manual",
        }
    }

    pub(crate) fn toggle(self) -> Self {
        match self {
            Self::Preconfigured => Self::Manual,
            Self::Manual => Self::Preconfigured,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ManualMotionConfig {
    pub(crate) initial_llh: Option<[f64; 3]>,
    pub(crate) initial_heading_deg: f64,
    pub(crate) cruise_speed_mps: f64,
    pub(crate) accel_limit_mps2: f64,
    pub(crate) turn_rate_limit_dps: f64,
}

impl Default for ManualMotionConfig {
    fn default() -> Self {
        Self {
            initial_llh: None,
            initial_heading_deg: 0.0,
            cruise_speed_mps: 1.0,
            accel_limit_mps2: 1.0,
            turn_rate_limit_dps: 45.0,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TuiConfig {
    pub(crate) motion_source: MotionSource,
    pub(crate) manual_motion: ManualMotionConfig,
    pub(crate) ephemerides: Option<PathBuf>,
    pub(crate) user_motion_ecef: Option<PathBuf>,
    pub(crate) user_motion_llh: Option<PathBuf>,
    pub(crate) nmea_gga: Option<PathBuf>,
    pub(crate) location_ecef: Option<Vec<f64>>,
    pub(crate) location: Option<Vec<f64>>,
    pub(crate) leap: Option<Vec<i32>>,
    pub(crate) time: Option<String>,
    pub(crate) time_override: Option<bool>,
    pub(crate) duration: Option<f64>,
    pub(crate) output: Option<PathBuf>,
    pub(crate) tx: Vec<TxBackend>,
    pub(crate) frequency: usize,
    pub(crate) bits: usize,
    pub(crate) ionospheric_disable: bool,
    pub(crate) path_loss: Option<i32>,
    pub(crate) verbose: bool,

    pub(crate) hackrf_serial: Option<String>,
    pub(crate) hackrf_rf_freq_hz: u64,
    pub(crate) hackrf_txvga_gain: u16,
    pub(crate) hackrf_amp_enable: bool,
    pub(crate) hackrf_usb_transfer_bytes: usize,
    pub(crate) hackrf_usb_transfers: usize,
    pub(crate) hackrf_queue_blocks: usize,
    pub(crate) hackrf_prefill_blocks: usize,
    pub(crate) hackrf_drop_on_underrun: bool,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            motion_source: MotionSource::Preconfigured,
            manual_motion: ManualMotionConfig::default(),
            ephemerides: None,
            user_motion_ecef: None,
            user_motion_llh: None,
            nmea_gga: None,
            location_ecef: None,
            location: None,
            leap: None,
            time: None,
            time_override: None,
            duration: None,
            output: None,
            tx: Vec::new(),
            frequency: 2_600_000,
            bits: 16,
            ionospheric_disable: false,
            path_loss: None,
            verbose: false,

            hackrf_serial: None,
            hackrf_rf_freq_hz: 1_575_420_000,
            hackrf_txvga_gain: 20,
            hackrf_amp_enable: false,
            hackrf_usb_transfer_bytes: 256 * 1024,
            hackrf_usb_transfers: 16,
            hackrf_queue_blocks: 8,
            hackrf_prefill_blocks: 2,
            hackrf_drop_on_underrun: false,
        }
    }
}

impl TuiConfig {
    pub(crate) fn apply_overrides_from_args(&mut self, args: &Args) {
        self.ephemerides.clone_from(&args.ephemerides);
        self.user_motion_ecef.clone_from(&args.user_motion_ecef);
        self.user_motion_llh.clone_from(&args.user_motion_llh);
        self.nmea_gga.clone_from(&args.nmea_gga);
        self.location_ecef.clone_from(&args.location_ecef);
        self.location.clone_from(&args.location);
        self.leap.clone_from(&args.leap);
        self.time.clone_from(&args.time);
        self.time_override = args.time_override;
        self.duration = args.duration;
        self.output.clone_from(&args.output);
        self.tx.clone_from(&args.tx);
        self.frequency = args.frequency;
        self.bits = args.bits;
        self.ionospheric_disable = args.ionospheric_disable;
        self.path_loss = args.path_loss;
        self.verbose = args.verbose;

        self.hackrf_serial.clone_from(&args.hackrf_serial);
        self.hackrf_rf_freq_hz = args.hackrf_rf_freq_hz;
        self.hackrf_txvga_gain = args.hackrf_txvga_gain;
        self.hackrf_amp_enable = args.hackrf_amp_enable;
        self.hackrf_usb_transfer_bytes = args.hackrf_usb_transfer_bytes;
        self.hackrf_usb_transfers = args.hackrf_usb_transfers;
        self.hackrf_queue_blocks = args.hackrf_queue_blocks;
        self.hackrf_prefill_blocks = args.hackrf_prefill_blocks;
        self.hackrf_drop_on_underrun = args.hackrf_drop_on_underrun;

        self.manual_motion.initial_llh = args
            .location
            .as_ref()
            .and_then(|values| triplet_from_vec(values))
            .or_else(|| {
                args.location_ecef.as_ref().and_then(|location_ecef| {
                    let ecef = Ecef::from(&triplet_from_vec(location_ecef)?);
                    let location = Location::from(&ecef);
                    Some([
                        location.latitude.to_degrees(),
                        location.longitude.to_degrees(),
                        location.height,
                    ])
                })
            });
    }

    pub(crate) fn uses_manual_motion(&self) -> bool {
        self.motion_source == MotionSource::Manual
    }

    pub(crate) fn effective_duration(&self) -> Option<f64> {
        if self.uses_manual_motion() {
            None
        } else {
            self.duration
        }
    }

    pub(crate) fn validate_for_run(&self) -> Result<(), String> {
        if self.ephemerides.is_none() {
            return Err("ephemerides is required".to_string());
        }

        if self
            .tx
            .iter()
            .any(|backend| matches!(backend, TxBackend::Hackrf))
        {
            if self.hackrf_usb_transfer_bytes == 0 {
                return Err("hackrf_usb_transfer_bytes must be > 0".to_string());
            }
            if self.hackrf_usb_transfers == 0 {
                return Err("hackrf_usb_transfers must be > 0".to_string());
            }
            if self.hackrf_queue_blocks == 0 {
                return Err("hackrf_queue_blocks must be > 0".to_string());
            }
            if self.hackrf_txvga_gain > 47 {
                return Err("hackrf_txvga_gain must be in 0..=47".to_string());
            }
        }

        if self.uses_manual_motion() {
            self.validate_manual_motion()?;
        }

        Ok(())
    }

    fn validate_manual_motion(&self) -> Result<(), String> {
        let Some(initial_llh) = self.manual_motion.initial_llh else {
            return Err("manual mode requires initial LLH position".to_string());
        };

        let [latitude_deg, longitude_deg, height_m] = initial_llh;
        if !latitude_deg.is_finite()
            || !longitude_deg.is_finite()
            || !height_m.is_finite()
        {
            return Err("manual mode initial LLH must be finite".to_string());
        }
        if !(-90.0..=90.0).contains(&latitude_deg) {
            return Err("manual mode latitude must be in -90..=90".to_string());
        }
        if !(-180.0..=180.0).contains(&longitude_deg) {
            return Err(
                "manual mode longitude must be in -180..=180".to_string()
            );
        }

        if !self.manual_motion.initial_heading_deg.is_finite() {
            return Err("manual mode heading must be finite".to_string());
        }
        if !self.manual_motion.cruise_speed_mps.is_finite()
            || self.manual_motion.cruise_speed_mps < 0.0
        {
            return Err("manual mode cruise speed must be >= 0".to_string());
        }
        if !self.manual_motion.accel_limit_mps2.is_finite()
            || self.manual_motion.accel_limit_mps2 <= 0.0
        {
            return Err("manual mode accel limit must be > 0".to_string());
        }
        if !self.manual_motion.turn_rate_limit_dps.is_finite()
            || self.manual_motion.turn_rate_limit_dps <= 0.0
        {
            return Err("manual mode turn rate limit must be > 0".to_string());
        }

        if self.user_motion_ecef.is_some()
            || self.user_motion_llh.is_some()
            || self.nmea_gga.is_some()
        {
            return Err("manual mode cannot be combined with \
                        user_motion_ecef, user_motion_llh, or nmea_gga"
                .to_string());
        }

        Ok(())
    }
}

fn triplet_from_vec(values: &[f64]) -> Option<[f64; 3]> {
    (values.len() == 3).then(|| [values[0], values[1], values[2]])
}
