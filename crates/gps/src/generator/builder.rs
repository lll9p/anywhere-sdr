use std::path::PathBuf;

use constants::{EPHEM_ARRAY_SIZE, MAX_SAT};
use geometry::Ecef;

use crate::{
    Error,
    datetime::{GpsCalendarDateTime, GpsTime, UtcDateTime},
    ephemeris::Ephemeris,
    generator::{
        RuntimeMotionControl,
        utils::{MotionMode, read_navigation_data},
    },
    io::DataFormat,
    ionoutc::IonoUtc,
};

/// Builder finalization and generator construction.
mod build;
/// Position, motion, and update-step builder settings.
mod positions;
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
    /// Path to the output file for I/Q samples
    output_file: Option<PathBuf>,
    /// Ephemeris data, ionospheric parameters, and UTC parameters
    ephemerides_data: Option<EphemerisRelatedData>,
    /// Leap second parameters [week, day, `delta_t`]
    leap: Option<Vec<i32>>,
    /// Receiver positions (static or dynamic)
    positions: Option<Vec<Ecef>>,
    /// Sample rate for position updates in seconds
    sample_rate: Option<f64>,
    /// Motion mode (static or dynamic)
    mode: Option<MotionMode>,
    /// Optional runtime motion controller (`UserControl` mode)
    runtime_motion_control: Option<RuntimeMotionControl>,
    /// Simulation duration in seconds
    duration: Option<f64>,
    /// Sampling frequency in Hz
    frequency: Option<f64>,
    /// Whether to override ephemeris time with simulation start time
    time_override: Option<bool>,
    /// GPS time at which the simulation starts
    receiver_gps_time: Option<GpsTime>,
    /// I/Q sample data format (1, 8, or 16 bits)
    data_format: Option<DataFormat>,
    /// Fixed gain value to override path loss calculations
    path_loss: Option<i32>,
    /// Whether to disable ionospheric delay modeling
    ionospheric_disable: Option<bool>,
    /// Whether to enable verbose output
    verbose: Option<bool>,
}
impl SignalGeneratorBuilder {
    /// Parses a timestamp with an explicit UTC offset.
    fn parse_utc_timestamp(value: &str) -> Result<jiff::Timestamp, Error> {
        value
            .parse()
            .map_err(|error: jiff::Error| Error::InvalidCalendarDate {
                scale: "UTC",
                reason: error.to_string(),
            })
    }

    /// Parses UTC while preserving an explicit leap-second label.
    fn parse_utc(value: &str) -> Result<UtcDateTime, Error> {
        if !value.contains(":60") {
            let timestamp = Self::parse_utc_timestamp(value)?;
            return UtcDateTime::from_timestamp(&timestamp);
        }
        Self::parse_utc_leap_label(value)
    }

    /// Parses the documented UTC leap-label form without POSIX normalization.
    fn parse_utc_leap_label(value: &str) -> Result<UtcDateTime, Error> {
        let invalid = || Error::InvalidCalendarDate {
            scale: "UTC",
            reason: "leap-second labels must use \
                     YYYY-MM-DDTHH:MM:60[.fraction]Z"
                .into(),
        };
        let value = value.strip_suffix('Z').ok_or_else(&invalid)?;
        let (date, time) = value.split_once('T').ok_or_else(&invalid)?;
        if time.contains('T') {
            return Err(invalid());
        }

        let mut date_parts = date.split('-');
        let year = date_parts
            .next()
            .ok_or_else(&invalid)?
            .parse::<i32>()
            .map_err(|_| invalid())?;
        let month = date_parts
            .next()
            .ok_or_else(&invalid)?
            .parse::<i32>()
            .map_err(|_| invalid())?;
        let day = date_parts
            .next()
            .ok_or_else(&invalid)?
            .parse::<i32>()
            .map_err(|_| invalid())?;
        if date_parts.next().is_some() {
            return Err(invalid());
        }

        let mut time_parts = time.split(':');
        let hour = time_parts
            .next()
            .ok_or_else(&invalid)?
            .parse::<i32>()
            .map_err(|_| invalid())?;
        let minute = time_parts
            .next()
            .ok_or_else(&invalid)?
            .parse::<i32>()
            .map_err(|_| invalid())?;
        let second_text = time_parts.next().ok_or_else(&invalid)?;
        if time_parts.next().is_some() {
            return Err(invalid());
        }
        let valid_second = second_text == "60"
            || second_text.strip_prefix("60.").is_some_and(|fraction| {
                !fraction.is_empty()
                    && fraction.bytes().all(|byte| byte.is_ascii_digit())
            });
        if !valid_second {
            return Err(invalid());
        }
        let second = second_text.parse::<f64>().map_err(|_| invalid())?;
        UtcDateTime::new(year, month, day, hour, minute, second)
    }

    /// Parses a timezone-free GPS calendar label.
    fn parse_gps_calendar(value: &str) -> Result<jiff::civil::DateTime, Error> {
        let clock = value
            .split_once('T')
            .or_else(|| value.split_once(' '))
            .map(|(_, clock)| clock);
        if clock.is_some_and(|clock| {
            clock.bytes().any(|byte| {
                matches!(byte, b'Z' | b'z' | b'+' | b'-' | b'[' | b']')
            })
        }) {
            return Err(Error::InvalidCalendarDate {
                scale: "GPS calendar",
                reason: "timezone-bearing labels are available only through \
                         utc_time()"
                    .into(),
            });
        }
        value
            .parse()
            .map_err(|error: jiff::Error| Error::InvalidCalendarDate {
                scale: "GPS calendar",
                reason: error.to_string(),
            })
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

    /// Sets the simulation start from a UTC timestamp or `now`.
    ///
    /// The timestamp must include an explicit UTC offset, for example
    /// `2026-07-14T00:00:00Z`. Historical GPS-UTC leap offsets are applied.
    pub fn utc_time(mut self, time: Option<String>) -> Result<Self, Error> {
        if let Some(time) = time {
            let utc = if time.eq_ignore_ascii_case("now") {
                UtcDateTime::from_timestamp(&jiff::Timestamp::now())?
            } else {
                Self::parse_utc(&time)?
            };
            self.receiver_gps_time = Some(GpsTime::from_utc(&utc)?);
        }
        Ok(self)
    }

    /// Sets the simulation start from a timezone-free GPS calendar label.
    ///
    /// This API is intended for GPS-system labels such as RINEX epochs and
    /// compatibility fixtures. It never applies a GPS-UTC leap offset.
    pub fn gps_calendar_time(
        mut self, time: Option<String>,
    ) -> Result<Self, Error> {
        if let Some(time) = time {
            if time.eq_ignore_ascii_case("now") {
                return Err(Error::InvalidCalendarDate {
                    scale: "GPS calendar",
                    reason: "now is available only through utc_time()".into(),
                });
            }
            let civil = Self::parse_gps_calendar(&time)?;
            let gps_calendar = GpsCalendarDateTime::from_civil(civil);
            self.receiver_gps_time =
                Some(GpsTime::from_gps_calendar(&gps_calendar)?);
        }
        Ok(self)
    }

    /// Sets the simulation start using UTC semantics.
    #[deprecated(note = "use utc_time() or gps_calendar_time() explicitly")]
    pub fn time(self, time: Option<String>) -> Result<Self, Error> {
        self.utc_time(time)
    }

    /// Sets the simulation duration in seconds.
    ///
    /// This method specifies how long the simulation should run. Duration is
    /// converted to the nearest whole complex sample; exact half-sample ties
    /// round upward, so a positive duration below half a sample emits no data.
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
    ///   When true, ionospheric correction is disabled (ionoutc.enable = false)
    ///   When false, ionospheric correction is enabled (ionoutc.enable = true)
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
        if let Some(leap_values) = &leap {
            // Validate leap second parameters
            if leap_values.len() >= 3 {
                // Ensure the values are valid
                let week_number = leap_values[0];
                let day_number = leap_values[1];
                let delta_time = leap_values[2];

                // Validate according to the same rules as in gpssim.c
                // We'll validate these parameters again in the build method
                // but we do a preliminary check here for early error detection
                if week_number < 0 {
                    tracing::warn!(week_number, "invalid GPS week number");
                }
                if !(1..=7).contains(&day_number) {
                    tracing::warn!(day_number, "invalid GPS day number");
                }
                if !(-128..=127).contains(&delta_time) {
                    tracing::warn!(delta_time, "invalid delta leap second");
                }
            } else {
                tracing::warn!(
                    leap_values_len = leap_values.len(),
                    "leap second parameters must have 3 values: [week, day, \
                     delta]"
                );
            }
        }
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
}
