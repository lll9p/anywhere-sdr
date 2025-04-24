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
        utils::{MotionMode, read_navigation_data},
    },
    io::DataFormat,
    ionoutc::IonoUtc,
};
/// Type alias for ephemeris-related data used in the builder.
///
/// This tuple contains:
/// - The number of valid ephemeris sets
/// - Ionospheric and UTC parameters
/// - A 2D array of ephemeris data organized by time set and satellite PRN
///
/// This is the same structure as the `Data` type in the utils module,
/// but defined here for use within the builder.
type EphemerisRelatedData = (
    usize,
    IonoUtc,
    Box<[[Ephemeris; MAX_SAT]; EPHEM_ARRAY_SIZE]>,
);
/// Builder for creating and configuring a `SignalGenerator`.
///
/// This struct implements the builder pattern for creating a `SignalGenerator`
/// with a fluent API. It allows setting various simulation parameters through
/// method chaining, with reasonable defaults for optional parameters.
///
/// # Example
/// ```no_run
/// use std::path::PathBuf;
///
/// use gps::SignalGeneratorBuilder;
///
/// let builder = SignalGeneratorBuilder::default()
///     .navigation_file(Some(PathBuf::from("brdc0010.22n")))
///     .unwrap()
///     .location(Some(vec![35.6813, 139.7662, 10.0]))
///     .unwrap()
///     .duration(Some(60.0))
///     .data_format(Some(8))
///     .unwrap()
///     .output_file(Some(PathBuf::from("output.bin")));
///
/// let mut generator = builder.build().unwrap();
/// generator.initialize().unwrap();
/// generator.run_simulation().unwrap();
/// ```
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
    /// Parses a datetime string into a timestamp.
    ///
    /// Used internally to convert user-provided date/time strings into a format
    /// that can be used for simulation timing.
    ///
    /// # Arguments
    /// * `value` - A string representing a date and time in the format
    ///   "YYYY-MM-DD HH:MM:SS"
    ///
    /// # Returns
    /// A Result containing either the parsed timestamp or a parsing error
    fn parse_datetime(value: &str) -> Result<jiff::Timestamp, jiff::Error> {
        let time: jiff::Timestamp = value.parse()?;
        Ok(time)
    }

    /// Sets the RINEX navigation file for GPS ephemerides.
    ///
    /// This file contains satellite orbit and clock parameters needed for the
    /// simulation. The function reads and processes the navigation data,
    /// extracting ephemeris sets and ionospheric/UTC parameters.
    ///
    /// # Arguments
    /// * `navigation_file` - Optional path to a RINEX navigation file
    ///   (typically with .nav or .n extension)
    ///
    /// # Returns
    /// * `Ok(Self)` - Builder with navigation data loaded
    /// * `Err(Error)` - If the file cannot be read or contains no valid
    ///   ephemeris data
    ///
    /// # Errors
    /// * `Error::NoEphemeris` - If no valid ephemeris data was found in the
    ///   file
    /// * Other errors if the file cannot be read or parsed
    pub fn navigation_file(
        mut self, navigation_file: Option<PathBuf>,
    ) -> Result<Self, Error> {
        // Read ephemeris
        if let Some(file) = navigation_file {
            let (count, iono_utc, ephemerides) = read_navigation_data(&file)
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

    /// Sets whether to override ephemeris time with the simulation start time.
    ///
    /// When enabled, this option adjusts the ephemeris data to match the
    /// simulation start time, allowing the use of ephemeris data that would
    /// otherwise be out of range. This is useful for testing with specific
    /// ephemeris data at arbitrary times.
    ///
    /// # Arguments
    /// * `time_override` - Optional boolean flag to enable time override
    ///   (default: false)
    ///
    /// # Returns
    /// * `Self` - Builder with time override setting
    pub fn time_override(mut self, time_override: Option<bool>) -> Self {
        self.time_override = time_override;
        self
    }

    /// Sets the simulation start time.
    ///
    /// This method sets the GPS time at which the simulation will start.
    /// The time can be specified as a string in the format "YYYY-MM-DD
    /// HH:MM:SS" or as the special value "now" to use the current system
    /// time.
    ///
    /// # Arguments
    /// * `time` - Optional string representing the start time or "now"
    ///
    /// # Returns
    /// * `Ok(Self)` - Builder with start time set
    /// * `Err(Error)` - If the time string cannot be parsed
    ///
    /// # Errors
    /// * Returns an error if the time string format is invalid
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

    /// Sets the simulation duration in seconds.
    ///
    /// This method specifies how long the simulation should run.
    /// For static positioning, this determines how many samples to generate.
    /// For dynamic positioning, this is limited by the number of positions
    /// available in the user motion file.
    ///
    /// # Arguments
    /// * `duration` - Optional simulation duration in seconds
    ///
    /// # Returns
    /// * `Self` - Builder with duration set
    pub fn duration(mut self, duration: Option<f64>) -> Self {
        self.duration = duration;
        self
    }

    /// Controls whether ionospheric correction is disabled.
    ///
    /// The ionospheric layer affects GPS signal propagation, causing delays.
    /// This option allows disabling the ionospheric correction model for
    /// testing or when simulating ideal conditions.
    ///
    /// # Arguments
    /// * `disable` - Optional boolean flag to disable ionospheric correction
    ///   (default: true)
    ///
    /// # Returns
    /// * `Self` - Builder with ionospheric correction setting
    pub fn ionospheric_disable(mut self, disable: Option<bool>) -> Self {
        self.ionospheric_disable = disable;
        self
    }

    /// Sets leap second parameters for UTC-GPS time conversion.
    ///
    /// GPS time and UTC time differ by a number of leap seconds. This method
    /// allows setting the leap second parameters for accurate time conversion.
    ///
    /// # Arguments
    /// * `leap` - Optional vector containing [week number, day number, delta
    ///   time in seconds]
    ///   - week number: GPS week number when the leap second becomes effective
    ///   - day number: Day of week (1-7, where 1 is Sunday) when the leap
    ///     second becomes effective
    ///   - delta time: Current difference between GPS time and UTC in seconds
    ///
    /// # Returns
    /// * `Self` - Builder with leap second parameters set
    pub fn leap(mut self, leap: Option<Vec<i32>>) -> Self {
        self.leap = leap;
        self
    }

    /// Sets the I/Q sample data format for the output file.
    ///
    /// This method specifies the bit depth for the I/Q samples in the output
    /// file. Different bit depths offer trade-offs between file size and
    /// signal quality.
    ///
    /// # Arguments
    /// * `data_format` - Optional bit depth (1, 8, or 16)
    ///   - 1: 1-bit I/Q samples (smallest file size, lowest quality)
    ///   - 8: 8-bit I/Q samples (medium file size and quality)
    ///   - 16: 16-bit I/Q samples (largest file size, highest quality)
    ///
    /// # Returns
    /// * `Ok(Self)` - Builder with data format set
    /// * `Err(Error)` - If an invalid bit depth is specified
    ///
    /// # Errors
    /// * Returns an error if the data format is not 1, 8, or 16 bits
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

    /// Sets the output file path for the generated I/Q samples.
    ///
    /// This method specifies where the generated GPS signal I/Q samples will be
    /// saved. The file format is binary with the structure determined by
    /// the `data_format` setting.
    ///
    /// # Arguments
    /// * `file` - Optional path to the output file
    ///
    /// # Returns
    /// * `Self` - Builder with output file path set
    pub fn output_file(mut self, file: Option<PathBuf>) -> Self {
        self.output_file = file;
        self
    }

    /// Sets the sampling frequency for the generated I/Q samples.
    ///
    /// This method specifies the sampling rate in Hz for the generated GPS
    /// signal. Higher sampling rates provide more detail but result in
    /// larger output files. The default is 2.6 MHz (2,600,000 Hz).
    ///
    /// # Arguments
    /// * `frequency` - Optional sampling frequency in Hz (must be at least 1
    ///   MHz)
    ///
    /// # Returns
    /// * `Ok(Self)` - Builder with sampling frequency set
    /// * `Err(Error)` - If the frequency is invalid
    ///
    /// # Errors
    /// * Returns an error if the frequency is less than 1 MHz
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

    /// Sets a static location in ECEF (Earth-Centered, Earth-Fixed)
    /// coordinates.
    ///
    /// This method sets a fixed receiver position using ECEF coordinates.
    /// When this option is used, the simulation will use static positioning
    /// mode.
    ///
    /// # Arguments
    /// * `location` - Optional vector containing [X, Y, Z] coordinates in
    ///   meters
    ///
    /// # Returns
    /// * `Ok(Self)` - Builder with static location set
    /// * `Err(Error)` - If another positioning method was already set
    ///
    /// # Errors
    /// * Returns an error if another positioning method was already set
    ///   (duplicate position)
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

    /// Sets a static location in LLH (Latitude, Longitude, Height) coordinates.
    ///
    /// This method sets a fixed receiver position using geodetic coordinates.
    /// The coordinates are automatically converted from degrees to radians and
    /// then to ECEF. When this option is used, the simulation will use
    /// static positioning mode.
    ///
    /// # Arguments
    /// * `location` - Optional vector containing [latitude, longitude,
    ///   altitude] in degrees and meters
    ///
    /// # Returns
    /// * `Ok(Self)` - Builder with static location set
    /// * `Err(Error)` - If another positioning method was already set
    ///
    /// # Errors
    /// * Returns an error if another positioning method was already set
    ///   (duplicate position)
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

    /// Controls whether to enable verbose output during simulation.
    ///
    /// When enabled, this option causes the simulator to output detailed
    /// information about satellite visibility, signal strength, and other
    /// parameters during the simulation. This is useful for debugging and
    /// understanding the simulation process.
    ///
    /// # Arguments
    /// * `verbose` - Optional boolean flag to enable verbose output (default:
    ///   false)
    ///
    /// # Returns
    /// * `Self` - Builder with verbose setting
    pub fn verbose(mut self, verbose: Option<bool>) -> Self {
        self.verbose = verbose;
        self
    }

    /// Sets a fixed gain value to override path loss calculations.
    ///
    /// Normally, the simulator calculates signal strength based on satellite
    /// distance (path loss). This method allows setting a fixed gain value
    /// for all satellites, which can be useful for testing or when
    /// simulating ideal conditions.
    ///
    /// # Arguments
    /// * `loss` - Optional fixed gain value in dB
    ///
    /// # Returns
    /// * `Self` - Builder with fixed gain value set
    pub fn path_loss(mut self, loss: Option<i32>) -> Self {
        self.path_loss = loss;
        self
    }

    /// Sets a user motion file in ECEF coordinates for dynamic positioning.
    ///
    /// This method loads a file containing user motion data in Earth-Centered,
    /// Earth-Fixed (ECEF) coordinate format. The file should contain
    /// position data for each time step of the simulation. When this option
    /// is used, the simulation will use dynamic positioning mode.
    ///
    /// # Arguments
    /// * `file` - Optional path to a user motion file in ECEF format
    ///
    /// # Returns
    /// * `Ok(Self)` - Builder with user motion data loaded
    /// * `Err(Error)` - If the file cannot be read or if another positioning
    ///   method was already set
    ///
    /// # Errors
    /// * Returns an error if another positioning method was already set
    ///   (duplicate position)
    /// * Returns parsing errors if the file cannot be read or contains invalid
    ///   data
    pub fn user_motion_file(
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

    /// Sets a user motion file in LLH coordinates for dynamic positioning.
    ///
    /// This method loads a file containing user motion data in Latitude,
    /// Longitude, Height (LLH) coordinate format. The file should contain
    /// position data for each time step of the simulation.
    /// The LLH coordinates will be automatically converted to ECEF coordinates
    /// for internal use. When this option is used, the simulation will use
    /// dynamic positioning mode.
    ///
    /// # Arguments
    /// * `file` - Optional path to a user motion file in LLH format
    ///
    /// # Returns
    /// * `Ok(Self)` - Builder with user motion data loaded and converted to
    ///   ECEF
    /// * `Err(Error)` - If the file cannot be read or if another positioning
    ///   method was already set
    ///
    /// # Errors
    /// * Returns an error if another positioning method was already set
    ///   (duplicate position)
    /// * Returns parsing errors if the file cannot be read or contains invalid
    ///   data
    pub fn user_motion_llh_file(
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

    /// Sets a NMEA GGA format file for dynamic positioning.
    ///
    /// This method loads a file containing position data in NMEA GGA sentence
    /// format. NMEA GGA sentences contain position information including
    /// latitude, longitude, and altitude. The NMEA data will be
    /// automatically converted to ECEF coordinates for internal use.
    /// When this option is used, the simulation will use dynamic positioning
    /// mode.
    ///
    /// # Arguments
    /// * `file` - Optional path to a file containing NMEA GGA sentences
    ///
    /// # Returns
    /// * `Ok(Self)` - Builder with NMEA GGA data loaded and converted to ECEF
    /// * `Err(Error)` - If the file cannot be read or if another positioning
    ///   method was already set
    ///
    /// # Errors
    /// * Returns an error if another positioning method was already set
    ///   (duplicate position)
    /// * Returns parsing errors if the file cannot be read or contains invalid
    ///   NMEA data
    pub fn user_motion_nmea_gga_file(
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

    /// Sets the time step between simulation updates.
    ///
    /// This method specifies the time interval in seconds between position
    /// updates in the simulation. The default is 0.1 seconds (10 Hz update
    /// rate). Smaller values provide more frequent updates but increase
    /// computation time.
    ///
    /// # Arguments
    /// * `rate` - Optional time step in seconds
    ///
    /// # Returns
    /// * `Self` - Builder with sample rate set
    pub fn sample_rate(mut self, rate: Option<f64>) -> Self {
        self.sample_rate = rate;
        self
    }

    /// Builds the `SignalGenerator` with the configured settings.
    ///
    /// This method finalizes the builder pattern, creating a `SignalGenerator`
    /// instance with all the settings that have been configured. It
    /// performs validation of the settings and sets appropriate defaults
    /// for any unspecified options.
    ///
    /// # Returns
    /// * `Ok(SignalGenerator)` - A fully configured signal generator ready for
    ///   simulation
    /// * `Err(Error)` - If the configuration is invalid or incomplete
    ///
    /// # Errors
    /// * `Error::navigation_not_set()` - If no navigation file was provided
    /// * `Error::invalid_gps_day()` - If an invalid GPS day was specified
    /// * `Error::invalid_gps_week()` - If an invalid GPS week was specified
    /// * `Error::invalid_delta_leap_second()` - If an invalid leap second delta
    ///   was specified
    /// * `Error::wrong_positions()` - If the positions vector is empty
    /// * `Error::invalid_duration()` - If a negative duration was specified
    /// * `Error::invalid_start_time()` - If the start time is outside the
    ///   ephemeris range
    /// * `Error::no_current_ephemerides()` - If no valid ephemeris is available
    ///   for the start time
    /// * `Error::data_format_not_set()` - If no data format was specified
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
            simulation_step_count: user_motion_count,
            receiver_gps_time,
            antenna_gains,
            antenna_pattern,
            mode,
            elevation_mask: 0.0, // Default elevation mask
            sample_frequency,
            sample_rate,
            data_format,
            fixed_gain: self.path_loss,
            output_file: self.output_file,
            verbose: false,
            ..Default::default()
        };
        Ok(generator)
    }
}
