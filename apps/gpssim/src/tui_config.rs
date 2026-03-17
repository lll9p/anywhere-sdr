use std::path::PathBuf;

use crate::cli::{Args, TxBackend};

#[derive(Clone, Debug)]
pub(crate) struct TuiConfig {
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
    }
}
