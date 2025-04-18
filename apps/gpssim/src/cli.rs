use std::path::PathBuf;

use clap::{ArgAction, Parser};
use gps::SignalGeneratorBuilder;

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

    /// ECEF X,Y,Z in meters (static mode) e.g.
    /// 3967283.154,1022538.181,4872414.484
    #[arg(short = 'c', long, value_parser, value_delimiter = ',')]
    location_ecef: Option<Vec<f64>>,

    /// Lat, lon, height (static mode) e.g. 35.681298,139.766247,10.0
    #[arg(short = 'l', long, value_parser, value_delimiter = ',')]
    location: Option<Vec<f64>>,

    /// User leap future event in GPS week number, day number, next leap second
    /// e.g. 2347,3,19
    #[arg(short = 'L', long, value_parser, value_delimiter = ',')]
    leap: Option<Vec<i32>>,

    /// Scenario start time YYYY-MM-DDTHH:MM:SSZ
    #[arg(short = 't', long)]
    time: Option<String>,

    /// Overwrite TOC and TOE to scenario start time
    #[arg(short = 'T', long)]
    time_override: Option<bool>,

    /// Duration [sec] (dynamic mode max: {}, static mode max: {})
    #[arg(short = 'd', long)]
    duration: Option<f64>,

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

    /// Disable path loss and hold power level constant [`fixed_gain`]
    #[arg(short = 'p', long)]
    path_loss: Option<i32>,

    /// Show details about simulated channels
    #[arg(short = 'v', long,default_value_t = false, action = ArgAction::SetTrue)]
    verbose: bool,
}

impl Args {
    pub fn run(&self) -> anyhow::Result<()> {
        let builder = SignalGeneratorBuilder::default()
            .navigation_file(Some(self.ephemerides.clone()))?
            .user_mothon_file(self.user_motion_ecef.clone())?
            .user_mothon_llh_file(self.user_motion_llh.clone())?
            .user_mothon_nmea_gga_file(self.nmea_gga.clone())?
            .location_ecef(self.location_ecef.clone())?
            .location(self.location.clone())?
            .leap(self.leap.clone())
            .time(self.time.clone())?
            .time_override(self.time_override)
            .duration(self.duration)
            .output_file(self.output.clone())
            .frequency(Some(self.frequency))?
            .data_format(Some(self.bits))?
            .ionospheric_disable(Some(self.ionospheric_disable))
            .path_loss(self.path_loss)
            .verbose(Some(self.verbose));
        let mut generator = builder.build()?;
        generator.initiallize();
        generator.generate()?;
        Ok(())
    }
}
