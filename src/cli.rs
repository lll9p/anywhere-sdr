use crate::constants::*;
use crate::datetime::datetime_t;
use crate::datetime::gpstime_t;
use crate::ionoutc::ionoutc_t;
use crate::process::date2gps;
use clap::ArgAction;
use clap::Parser;
use jiff::{Timestamp, civil::DateTime};
use std::path::PathBuf;

pub fn parse_datetime(value: String) -> Result<DateTime, jiff::Error> {
    let time: DateTime = value.parse()?;
    Ok(time)
}
/*

Options:
  -e <gps_nav>     RINEX navigation file for GPS ephemerides (required)
  -u <user_motion> User motion file in ECEF x, y, z format (dynamic mode)
  -x <user_motion> User motion file in lat, lon, height format (dynamic mode)
  -g <nmea_gga>    NMEA GGA stream (dynamic mode)
  -c <location>    ECEF X,Y,Z in meters (static mode) e.g. 3967283.154,1022538.181,4872414.484
  -l <location>    Lat, lon, height (static mode) e.g. 35.681298,139.766247,10.0
  -L <wnslf,dn,dtslf> User leap future event in GPS week number, day number, next leap second e.g. 2347,3,19
  -t <date,time>   Scenario start time YYYY/MM/DD,hh:mm:ss
  -T <date,time>   Overwrite TOC and TOE to scenario start time
  -d <duration>    Duration [sec] (dynamic mode max: {}, static mode max: {})
  -o <output>      I/Q sampling data file (default: gpssim.bin)
  -s <frequency>   Sampling frequency [Hz] (default: 2600000)
  -b <iq_bits>     I/Q data format [1/8/16] (default: 16)
  -i               Disable ionospheric delay for spacecraft scenario
  -p [fixed_gain]  Disable path loss and hold power level constant
  -v               Show details about simulated channels
"#,
*/
#[derive(Parser, Debug)]
#[command(term_width = 0)]
#[command(version, about="gps-sdr-sim compatible", long_about = None)]
#[command(propagate_version = true)]
pub struct Args {
    /// RINEX navigation file for GPS ephemerides (required)
    #[arg(short, long, value_hint = clap::ValueHint::FilePath)]
    ephemerides: std::path::PathBuf,

    /// User motion file in ECEF x, y, z format (dynamic mode)
    #[arg(short = 'u', long, value_hint = clap::ValueHint::FilePath)]
    user_motion_ecef: Option<PathBuf>,

    /// User motion file in lat, lon, height format (dynamic mode)
    #[arg(short = 'x', long, value_hint = clap::ValueHint::FilePath)]
    user_motion_llh: Option<PathBuf>,

    /// NMEA GGA stream (dynamic mode)
    #[arg(short = 'g', long, value_hint = clap::ValueHint::FilePath)]
    nmea_gga: Option<PathBuf>,

    /// ECEF X,Y,Z in meters (static mode) e.g. 3967283.154,1022538.181,4872414.484
    #[arg(short = 'c', long, value_parser, value_delimiter = ',')]
    location_ecef: Option<Vec<f64>>,

    /// Lat, lon, height (static mode) e.g. 35.681298,139.766247,10.0
    #[arg(short = 'l', long, value_parser, value_delimiter = ',')]
    location: Option<Vec<f64>>,

    /// User leap future event in GPS week number, day number, next leap second e.g. 2347,3,19
    #[arg(short = 'L', long, value_parser, value_delimiter = ',')]
    leap: Option<Vec<i32>>,

    /// Scenario start time YYYY-MM-DDTHH:MM:SSZ
    #[arg(short = 't', long)]
    time: Option<String>,

    /// Overwrite TOC and TOE to scenario start time
    #[arg(short = 'T', long)]
    time_override: Option<String>,

    /// Duration [sec] (dynamic mode max: {}, static mode max: {})
    #[arg(short = 'd', long)]
    duration: Option<usize>,

    /// I/Q sampling data file (default: gpssim.bin)
    #[arg(short = 'o', long)]
    output: Option<PathBuf>,

    /// Sampling frequency [Hz] (default: 2600000)
    #[arg(short = 's', long, default_value_t = 2600000)]
    frequency: usize,

    /// I/Q data format [1/8/16] (default: 16)
    #[arg(short = 'b', long, default_value_t = 16)]
    bits: usize,

    /// Disable ionospheric delay for spacecraft scenario
    #[arg(short = 'i', long, default_value_t = false, action = ArgAction::SetFalse)]
    ionospheric_disable: bool,

    /// Disable path loss and hold power level constant [fixed_gain]
    #[arg(short = 'p', long)]
    path_loss: Option<i32>,

    /// Show details about simulated channels
    #[arg(short = 'v', long,default_value_t = false, action = ArgAction::SetTrue)]
    verbose: bool,
}
pub struct Params {
    pub xyz: [[f64; 3]; USER_MOTION_SIZE],
    pub llh: [f64; 3],
    pub ionoutc: ionoutc_t,
    pub navfile: PathBuf,
    pub nmea_gga: bool,
    pub um_llh: bool,
    pub umfile: Option<PathBuf>,
    pub timeoverwrite: bool,
    pub static_location_mode: bool,
    pub outfile: PathBuf,
    pub samp_freq: f64,
    pub data_format: i32,
    pub t0: datetime_t,
    pub g0: gpstime_t,
    pub duration: f64,
    pub fixed_gain: i32,
    pub path_loss_enable: bool,
    pub verb: bool,
}
impl Default for Params {
    fn default() -> Self {
        let g0 = gpstime_t {
            week: -1,
            ..Default::default()
        };
        Self {
            xyz: [[0.0; 3]; USER_MOTION_SIZE],
            llh: [0.0; 3],
            ionoutc: ionoutc_t::default(),
            navfile: PathBuf::new(),
            nmea_gga: false,
            um_llh: false,
            umfile: None,
            timeoverwrite: false,
            static_location_mode: false,
            outfile: PathBuf::from("gpssim.bin"),
            samp_freq: 2600000f64,
            data_format: 16i32,
            t0: datetime_t::default(),
            g0,
            duration: USER_MOTION_SIZE as f64 / 10.0f64,
            fixed_gain: 128,
            path_loss_enable: true,
            verb: false,
        }
    }
}

impl Args {
    #[allow(clippy::impossible_comparisons)]
    pub fn get_params(self) -> Params {
        let mut params = Params::default();
        params.g0.week = -1; // Invalid start time

        params.navfile = self.ephemerides;
        if self.user_motion_ecef.is_some() {
            params.nmea_gga = false;
            params.um_llh = false;
            params.umfile = self.user_motion_ecef.clone();
        } else if self.user_motion_llh.is_some() {
            params.um_llh = true;
            params.umfile = self.user_motion_llh.clone();
        } else if self.nmea_gga.is_some() {
            params.nmea_gga = true;
            params.umfile = self.nmea_gga.clone();
        }

        // Static ECEF coordinates input mode
        if let Some(location) = self.location_ecef {
            params.static_location_mode = true;
            params.xyz[0][0] = location[0];
            params.xyz[0][1] = location[1];
            params.xyz[0][2] = location[2];
        }
        if let Some(location) = self.location {
            params.static_location_mode = true;
            params.llh[0] = location[0];
            params.llh[1] = location[1];
            params.llh[2] = location[2];
            params.llh[0] /= R2D;
            params.llh[1] /= R2D;
            crate::process::llh2xyz(&params.llh, &mut params.xyz[0]);
        }
        params.outfile = PathBuf::from("gpssim.bin");
        if let Some(out) = self.output {
            params.outfile = out;
        }
        assert!(
            self.frequency >= 1000000,
            "ERROR: Invalid sampling frequency."
        );
        params.samp_freq = self.frequency as f64;
        params.data_format = self.bits as i32;
        assert!(
            params.data_format == 1 || params.data_format == 8 || params.data_format == 16,
            "ERROR: Invalid I/Q data format."
        );
        if let Some(leap) = self.leap {
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
        if let Some(time) = self.time_override {
            params.timeoverwrite = true;
            if time == "now" {
                let now = Timestamp::now().in_tz("UTC").unwrap();
                params.t0.y = now.year() as i32;
                params.t0.m = now.month() as i32;
                params.t0.d = now.day() as i32;
                params.t0.hh = now.hour() as i32;
                params.t0.mm = now.minute() as i32;
                params.t0.sec = now.second() as f64;
                date2gps(&params.t0, &mut params.g0);
            } else {
                let time = parse_datetime(time).unwrap();
                params.t0.y = time.year() as i32;
                params.t0.m = time.month() as i32;
                params.t0.d = time.day() as i32;
                params.t0.hh = time.hour() as i32;
                params.t0.mm = time.minute() as i32;
                params.t0.sec = time.second() as f64;
                date2gps(&params.t0, &mut params.g0);
            }
        }
        if let Some(time) = self.time {
            let time = parse_datetime(time).unwrap();

            params.t0.y = time.year() as i32;
            params.t0.m = time.month() as i32;
            params.t0.d = time.day() as i32;
            params.t0.hh = time.hour() as i32;
            params.t0.mm = time.minute() as i32;
            params.t0.sec = time.second() as f64;

            date2gps(&params.t0, &mut params.g0);
        }
        if let Some(duration) = self.duration {
            params.duration = duration as f64;
        }
        // Disable ionospheric correction
        params.ionoutc.enable = !self.ionospheric_disable;
        if let Some(fixed_gain) = self.path_loss {
            params.fixed_gain = fixed_gain;
            params.path_loss_enable = false;
        }
        assert!(
            (1..=128).contains(&params.fixed_gain),
            "ERROR: Fixed gain must be between 1 and 128."
        );
        params.verb = self.verbose;
        params
    }
}
