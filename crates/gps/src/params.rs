use std::path::{Path, PathBuf};

use crate::{
    constants::*,
    datetime::{DateTime, GpsTime},
    ionoutc::IonoUtc,
    utils::llh2xyz,
};
#[derive(Clone)]
pub struct Params {
    pub xyz: [[f64; 3]; USER_MOTION_SIZE],
    pub llh: [f64; 3],
    pub ionoutc: IonoUtc,
    pub navfile: PathBuf,
    pub nmea_gga: bool,
    pub um_llh: bool,
    pub umfile: Option<PathBuf>,
    pub timeoverwrite: bool,
    pub static_location_mode: bool,
    pub outfile: PathBuf,
    pub samp_freq: f64,
    pub data_format: i32,
    pub t0: DateTime,
    pub g0: GpsTime,
    pub duration: f64,
    pub fixed_gain: i32,
    pub path_loss_enable: bool,
    pub verb: bool,
}
impl Default for Params {
    #[allow(clippy::large_stack_arrays)]
    fn default() -> Self {
        let g0 = GpsTime {
            week: -1,
            ..Default::default()
        };
        Self {
            xyz: [[0.0; 3]; USER_MOTION_SIZE],
            llh: [0.0; 3],
            ionoutc: IonoUtc::default(),
            navfile: PathBuf::new(),
            nmea_gga: false,
            um_llh: false,
            umfile: None,
            timeoverwrite: false,
            static_location_mode: false,
            outfile: PathBuf::from("gpssim.bin"),
            samp_freq: 2_600_000_f64,
            data_format: 16i32,
            t0: DateTime::default(),
            g0,
            duration: USER_MOTION_SIZE as f64 / 10.0f64,
            fixed_gain: 128,
            path_loss_enable: true,
            verb: false,
        }
    }
}
impl Params {
    fn parse_datetime(
        value: &str,
    ) -> Result<jiff::civil::DateTime, jiff::Error> {
        let time: jiff::civil::DateTime = value.parse()?;
        Ok(time)
    }

    #[allow(
        clippy::too_many_lines,
        clippy::impossible_comparisons,
        clippy::too_many_arguments
    )]
    pub fn new(
        ephemerides: &Path, user_motion_ecef: &Option<PathBuf>,
        user_motion_llh: &Option<PathBuf>, nmea_gga: &Option<PathBuf>,
        location_ecef: Option<Vec<f64>>, location: Option<Vec<f64>>,
        leap: &Option<Vec<i32>>, time: &Option<String>,
        time_override: &Option<String>, duration: &Option<usize>,
        output: &Option<PathBuf>, frequency: usize, bits: usize,
        ionospheric_disable: bool, path_loss: &Option<i32>, verbose: bool,
    ) -> Self {
        let mut params = Params::default();
        params.g0.week = -1; // Invalid start time

        params.navfile = ephemerides.to_path_buf();
        if user_motion_ecef.is_some() {
            params.nmea_gga = false;
            params.um_llh = false;
            params.umfile.clone_from(user_motion_ecef);
        } else if user_motion_llh.is_some() {
            params.um_llh = true;
            params.umfile.clone_from(user_motion_llh);
        } else if nmea_gga.is_some() {
            params.nmea_gga = true;
            params.umfile.clone_from(nmea_gga);
        }

        // Static ECEF coordinates input mode
        if let Some(location) = location_ecef {
            params.static_location_mode = true;
            params.xyz[0][0] = location[0];
            params.xyz[0][1] = location[1];
            params.xyz[0][2] = location[2];
        }
        if let Some(location) = location {
            params.static_location_mode = true;
            params.llh[0] = location[0];
            params.llh[1] = location[1];
            params.llh[2] = location[2];
            params.llh[0] /= R2D;
            params.llh[1] /= R2D;
            llh2xyz(&params.llh, &mut params.xyz[0]);
        }
        params.outfile = PathBuf::from("gpssim.bin");
        if let Some(out) = output {
            params.outfile.clone_from(out);
        }
        assert!(frequency >= 1_000_000, "ERROR: Invalid sampling frequency.");
        params.samp_freq = frequency as f64;
        params.data_format = bits as i32;
        assert!(
            params.data_format == 1
                || params.data_format == 8
                || params.data_format == 16,
            "ERROR: Invalid I/Q data format."
        );
        if let Some(leap) = leap {
            // enable custom Leap Event
            params.ionoutc.leapen = 1;
            params.ionoutc.wnlsf = leap[0];
            params.ionoutc.dn = leap[1];
            params.ionoutc.dtlsf = leap[2];
            assert!(
                params.ionoutc.dn < 1 && params.ionoutc.dn > 7,
                "ERROR: Invalid GPS day number"
            );
            assert!(params.ionoutc.wnlsf < 0, "ERROR: Invalid GPS week number");
            assert!(
                params.ionoutc.dtlsf < -128 && params.ionoutc.dtlsf > 127,
                "ERROR: Invalid delta leap second"
            );
        }
        if let Some(time) = time_override {
            params.timeoverwrite = true;
            if time == "now" {
                let now = jiff::Timestamp::now().in_tz("UTC").unwrap();
                params.t0.y = i32::from(now.year());
                params.t0.m = i32::from(now.month());
                params.t0.d = i32::from(now.day());
                params.t0.hh = i32::from(now.hour());
                params.t0.mm = i32::from(now.minute());
                params.t0.sec = f64::from(now.second());
                params.g0 = GpsTime::from(&params.t0);
            } else {
                let time = Self::parse_datetime(time).unwrap();
                params.t0.y = i32::from(time.year());
                params.t0.m = i32::from(time.month());
                params.t0.d = i32::from(time.day());
                params.t0.hh = i32::from(time.hour());
                params.t0.mm = i32::from(time.minute());
                params.t0.sec = f64::from(time.second());
                params.g0 = GpsTime::from(&params.t0);
            }
        }
        if let Some(time) = time {
            let time = Self::parse_datetime(time).unwrap();

            params.t0.y = i32::from(time.year());
            params.t0.m = i32::from(time.month());
            params.t0.d = i32::from(time.day());
            params.t0.hh = i32::from(time.hour());
            params.t0.mm = i32::from(time.minute());
            params.t0.sec = f64::from(time.second());
            params.g0 = GpsTime::from(&params.t0);
        }
        if let Some(duration) = duration {
            params.duration = *duration as f64;
        }
        // Disable ionospheric correction
        params.ionoutc.enable = !ionospheric_disable;
        if let Some(fixed_gain) = path_loss {
            params.fixed_gain = *fixed_gain;
            params.path_loss_enable = false;
        }
        assert!(
            (1..=128).contains(&params.fixed_gain),
            "ERROR: Fixed gain must be between 1 and 128."
        );
        params.verb = verbose;
        params
    }
}
