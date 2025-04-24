use std::path::PathBuf;

use constants::{EPHEM_ARRAY_SIZE, MAX_CHAN, MAX_SAT, R2D, SECONDS_IN_HOUR};
use geometry::{Ecef, Location};
use parsing::{read_nmea_gga, read_user_motion, read_user_motion_llh};

use crate::{
    Error,
    datetime::{DateTime, GpsTime},
    ephemeris::Ephemeris,
    generator::{
        signal_generator::SignalGenerator,
        utils::{MotionMode, read_navigatioin_data},
    },
    io::DataFormat,
    ionoutc::IonoUtc,
};
type EphemerisRelatedData = (
    usize,
    IonoUtc,
    Box<[[Ephemeris; MAX_SAT]; EPHEM_ARRAY_SIZE]>,
);
#[derive(Default)]
pub struct SignalGeneratorBuilder {
    output_file: Option<PathBuf>,
    ephemerides_data: Option<EphemerisRelatedData>,
    leap: Option<Vec<i32>>,
    positions: Option<Vec<Ecef>>,
    sample_rate: Option<f64>,
    mode: Option<MotionMode>,
    duration: Option<f64>,
    frequency: Option<f64>,
    time_override: Option<bool>,
    receiver_gps_time: Option<GpsTime>,
    data_format: Option<DataFormat>,
    path_loss: Option<i32>,
    ionospheric_disable: Option<bool>,
    verbose: Option<bool>,
}
impl SignalGeneratorBuilder {
    fn parse_datetime(value: &str) -> Result<jiff::Timestamp, jiff::Error> {
        let time: jiff::Timestamp = value.parse()?;
        Ok(time)
    }

    pub fn navigation_file(
        mut self, navigation_file: Option<PathBuf>,
    ) -> Result<Self, Error> {
        // Read ephemeris
        if let Some(file) = navigation_file {
            let (count, iono_utc, ephemerides) = read_navigatioin_data(&file)
                .map_err(|_| {
                Error::msg("ERROR: ephemeris file not found or error.")
            })?;
            if count == 0 {
                return Err(Error::NoEphemeris);
            }
            self.ephemerides_data = Some((count, iono_utc, ephemerides));
        }
        Ok(self)
    }

    pub fn time_override(mut self, time_override: Option<bool>) -> Self {
        self.time_override = time_override;
        self
    }

    pub fn time(mut self, time: Option<String>) -> Result<Self, Error> {
        if let Some(time) = time {
            let time_parsed = match time.to_lowercase().as_str() {
                "now" => jiff::Timestamp::now().in_tz("UTC"),
                time => Self::parse_datetime(time)?.in_tz("UTC"),
            }?;
            let time = DateTime {
                y: i32::from(time_parsed.year()),
                m: i32::from(time_parsed.month()),
                d: i32::from(time_parsed.day()),
                hh: i32::from(time_parsed.hour()),
                mm: i32::from(time_parsed.minute()),
                sec: f64::from(time_parsed.second()), // TODO: add floor?
            };
            self.receiver_gps_time = Some(GpsTime::from(&time));
        }
        Ok(self)
    }

    pub fn duration(mut self, duration: Option<f64>) -> Self {
        self.duration = duration;
        self
    }

    pub fn ionospheric_disable(mut self, disable: Option<bool>) -> Self {
        self.ionospheric_disable = disable;
        self
    }

    pub fn leap(mut self, leap: Option<Vec<i32>>) -> Self {
        self.leap = leap;
        self
    }

    pub fn data_format(
        mut self, data_format: Option<usize>,
    ) -> Result<Self, Error> {
        match data_format {
            Some(1) => self.data_format = Some(DataFormat::Bits1),
            Some(8) => self.data_format = Some(DataFormat::Bits8),
            Some(16) => self.data_format = Some(DataFormat::Bits16),
            None => {}
            _ => return Err(Error::invalid_data_format()),
        }
        Ok(self)
    }

    pub fn output_file(mut self, file: Option<PathBuf>) -> Self {
        self.output_file = file;
        self
    }

    pub fn frequency(
        mut self, frequency: Option<usize>,
    ) -> Result<Self, Error> {
        match frequency {
            Some(freq) if freq >= 1_000_000 => {
                self.frequency = Some(freq as f64);
            }
            None => {}
            _ => return Err(Error::invalid_sampling_frequency()),
        }
        Ok(self)
    }

    pub fn location_ecef(
        mut self, location: Option<Vec<f64>>,
    ) -> Result<Self, Error> {
        if self.positions.is_some() && location.is_some() {
            return Err(Error::duplicate_position());
        }
        if let Some(location) = location {
            self.mode = Some(MotionMode::Static);
            let location = Ecef::from(&[location[0], location[1], location[2]]);
            self.positions = Some(vec![location]);
        }
        Ok(self)
    }

    pub fn location(
        mut self, location: Option<Vec<f64>>,
    ) -> Result<Self, Error> {
        if self.positions.is_some() && location.is_some() {
            return Err(Error::duplicate_position());
        }
        if let Some(location) = location {
            self.mode = Some(MotionMode::Static);
            let mut location = [location[0], location[1], location[2]];
            location[0] /= R2D;
            location[1] /= R2D;
            let xyz = Ecef::from(&Location::from(&location));
            // let mut xyz = [0.0, 0.0, 0.0];
            // llh2xyz(&location, &mut xyz);
            self.positions = Some(vec![xyz]);
        }
        Ok(self)
    }

    pub fn verbose(mut self, verbose: Option<bool>) -> Self {
        self.verbose = verbose;
        self
    }

    pub fn path_loss(mut self, loss: Option<i32>) -> Self {
        self.path_loss = loss;
        self
    }

    pub fn user_mothon_file(
        mut self, file: Option<PathBuf>,
    ) -> Result<Self, Error> {
        if self.positions.is_some() && file.is_some() {
            return Err(Error::duplicate_position());
        }
        if let Some(file) = file {
            self.mode = Some(MotionMode::Dynamic);
            self.positions = Some(read_user_motion(&file).map_err(|e| {
                Error::ParsingError(format!("User motion file error: {e}"))
            })?);
        }
        Ok(self)
    }

    pub fn user_mothon_llh_file(
        mut self, file: Option<PathBuf>,
    ) -> Result<Self, Error> {
        if self.positions.is_some() && file.is_some() {
            return Err(Error::duplicate_position());
        }
        if let Some(file) = file {
            self.mode = Some(MotionMode::Dynamic);
            self.positions =
                Some(read_user_motion_llh(&file).map_err(|e| {
                    Error::ParsingError(format!(
                        "User motion LLH file error: {e}"
                    ))
                })?);
        }
        Ok(self)
    }

    pub fn user_mothon_nmea_gga_file(
        mut self, file: Option<PathBuf>,
    ) -> Result<Self, Error> {
        if self.positions.is_some() && file.is_some() {
            return Err(Error::duplicate_position());
        }
        if let Some(file) = file {
            self.mode = Some(MotionMode::Dynamic);
            self.positions = Some(read_nmea_gga(&file).map_err(|e| {
                Error::ParsingError(format!("NMEA GGA file error: {e}"))
            })?);
        }
        Ok(self)
    }

    pub fn sample_rate(mut self, rate: Option<f64>) -> Self {
        self.sample_rate = rate;
        self
    }

    #[allow(clippy::too_many_lines)]
    pub fn build(mut self) -> Result<SignalGenerator, Error> {
        // ensure navigation data is read
        let Some((count, mut ionoutc, mut ephemerides)) = self.ephemerides_data
        else {
            return Err(Error::navigation_not_set());
        };
        // check and set defaults
        // leap setting
        if let Some(leap) = self.leap {
            ionoutc.leapen = 1;
            ionoutc.wnlsf = leap[0];
            ionoutc.day_number = leap[1];
            ionoutc.dtlsf = leap[2];
            #[allow(clippy::impossible_comparisons)]
            if ionoutc.day_number < 1 && ionoutc.day_number > 7 {
                return Err(Error::invalid_gps_day());
            }
            if ionoutc.wnlsf < 0 {
                return Err(Error::invalid_gps_week());
            }
            #[allow(clippy::impossible_comparisons)]
            if ionoutc.dtlsf < -128 && ionoutc.dtlsf > 127 {
                return Err(Error::invalid_delta_leap_second());
            }
        }
        // positions
        let positions = if let Some(positions) = self.positions {
            if positions.len() == 1 {
                self.mode = Some(MotionMode::Static);
            } else if positions.is_empty() {
                return Err(Error::wrong_positions());
            }
            positions
        } else {
            // Default static location; Tokyo
            self.mode = Some(MotionMode::Static);
            let llh = [35.681_298 / R2D, 139.766_247 / R2D, 10.0];
            let xyz = Ecef::from(&Location::from(&llh));
            // let mut xyz = [0.0, 0.0, 0.0];
            // llh2xyz(&llh, &mut xyz);
            vec![xyz]
        };
        // sample_rate, default is 0.1/10HZ
        let sample_rate = self.sample_rate.unwrap_or(0.1);
        // mode
        let mode = self.mode.unwrap_or(MotionMode::Static);
        // check duration
        if self.duration.is_some_and(|d| d < 0.0) {
            return Err(Error::invalid_duration());
        }
        let user_motion_count = if let Some(duration) = self.duration {
            let duration_count = (duration * 10.0 + 0.5) as usize;
            if matches!(mode, MotionMode::Static) {
                // if is static mode just return it
                duration_count
            } else {
                // if not static mode need to set to min of them
                positions.len().min(duration_count)
            }
        } else {
            // not set, it is positions' len
            positions.len()
        };
        // frequency
        let sample_frequency = self.frequency.unwrap_or(2_600_000.0);
        // is override time?

        let antenna_gains: [i32; MAX_CHAN] = [0; MAX_CHAN];
        let antenna_pattern: [f64; 37] = [0.; 37];
        let mut gpstime_min = GpsTime::default();
        let mut gpstime_max = GpsTime::default();
        // get min time of ephemerides
        for sv in 0..MAX_SAT {
            if ephemerides[0][sv].vflg {
                gpstime_min = ephemerides[0][sv].toc.clone();
                break;
            }
        }
        // get max time of ephemerides
        for sv in 0..MAX_SAT {
            if ephemerides[count - 1][sv].vflg {
                gpstime_max = ephemerides[count - 1][sv].toc.clone();
                break;
            }
        }
        let time_override = self.time_override.unwrap_or(false);
        let receiver_gps_time = if let Some(gps_time_0) = self.receiver_gps_time
        {
            // Scenario start time has been set.
            if time_override {
                let mut gtmp = GpsTime {
                    week: gps_time_0.week,
                    sec: f64::from(
                        gps_time_0.sec as i32 / SECONDS_IN_HOUR as i32 * 2,
                    ) * SECONDS_IN_HOUR
                        * 2.0,
                };
                // let mut gtmp: GpsTime = GpsTime::default();
                // gtmp.week = g0.week;
                // gtmp.sec = f64::from(g0.sec as i32 / 7200) * 7200.0;
                // Overwrite the UTC reference week number
                let dsec = gtmp.diff_secs(&gpstime_min);
                ionoutc.week_number = gtmp.week;
                ionoutc.tot = gtmp.sec as i32;
                // Iono/UTC parameters may no longer valid
                //ionoutc.vflg = FALSE;
                for sv in 0..MAX_SAT {
                    for i_eph in ephemerides.iter_mut().take(count) {
                        if i_eph[sv].vflg {
                            gtmp = i_eph[sv].toc.add_secs(dsec);
                            let ttmp = DateTime::from(&gtmp);
                            i_eph[sv].toc = gtmp;
                            i_eph[sv].t = ttmp;
                            gtmp = i_eph[sv].toe.add_secs(dsec);
                            i_eph[sv].toe = gtmp;
                        }
                    }
                }
            } else if gps_time_0.diff_secs(&gpstime_min) < 0.0
                || gpstime_max.diff_secs(&gps_time_0) < 0.0f64
            {
                return Err(Error::invalid_start_time());
            }
            gps_time_0
        } else {
            gpstime_min
        };
        let mut valid_ephemerides_index = None;

        // Select the current set of ephemerides
        for (i, eph_item) in ephemerides.iter().enumerate().take(count) {
            for e in eph_item.iter().take(MAX_SAT) {
                if e.vflg {
                    let dt = receiver_gps_time.diff_secs(&e.toc);
                    if (-SECONDS_IN_HOUR..SECONDS_IN_HOUR).contains(&dt) {
                        valid_ephemerides_index = Some(i);
                        break;
                    }
                }
            }
            if valid_ephemerides_index.is_some() {
                // ieph has been set
                break;
            }
        }

        let Some(valid_ephemerides_index) = valid_ephemerides_index else {
            return Err(Error::no_current_ephemerides());
        };
        // Disable ionospheric correction
        ionoutc.enable = self.ionospheric_disable.unwrap_or(true);
        let Some(data_format) = self.data_format else {
            return Err(Error::data_format_not_set());
        };

        let generator = SignalGenerator {
            ephemerides,
            valid_ephemerides_index,
            ionoutc,
            positions,
            user_motion_count,
            receiver_gps_time,
            antenna_gains,
            antenna_pattern,
            mode,
            sample_frequency,
            sample_rate,
            data_format,
            fixed_gain: self.path_loss,
            out_file: self.output_file,
            verbose: false,
            ..Default::default()
        };
        Ok(generator)
    }
}
