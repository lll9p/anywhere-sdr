use constants::{MAX_CHAN, MAX_SAT, SECONDS_IN_HOUR, STATIC_MAX_DURATION};
use geometry::{Ecef, Location};

use super::SignalGeneratorBuilder;
use crate::{
    Error,
    datetime::GpsTime,
    generator::{
        MotionMode,
        signal_generator::SignalGenerator,
        timeline::{
            planned_interval_count, validate_duration, validate_update_step,
        },
    },
};

impl SignalGeneratorBuilder {
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
            if leap.len() < 3 {
                return Err(Error::invalid_leap_second_parameters());
            }
            ionoutc.leapen = 1;
            ionoutc.wnlsf = leap[0];
            ionoutc.day_number = leap[1];
            ionoutc.dtlsf = leap[2];
            if !(1..=7).contains(&ionoutc.day_number) {
                return Err(Error::invalid_gps_day());
            }
            if ionoutc.wnlsf < 0 {
                return Err(Error::invalid_gps_week());
            }
            if !(-128..=127).contains(&ionoutc.dtlsf) {
                return Err(Error::invalid_delta_leap_second());
            }
        }
        // positions
        let positions = if let Some(positions) = self.positions {
            if positions.len() == 1
                && !matches!(self.mode, Some(MotionMode::UserControl))
            {
                self.mode = Some(MotionMode::Static);
            } else if positions.is_empty() {
                return Err(Error::wrong_positions());
            }
            positions
        } else {
            // Default static location; Tokyo
            self.mode = Some(MotionMode::Static);
            let llh =
                Location::try_from_degrees(35.681_298, 139.766_247, 10.0)?;
            let xyz = Ecef::from(&llh);
            // let mut xyz = [0.0, 0.0, 0.0];
            // llh2xyz(&llh, &mut xyz);
            vec![xyz]
        };
        // sample_rate, default is 0.1/10HZ
        let sample_rate = self.sample_rate.unwrap_or(0.1);
        validate_update_step(sample_rate)?;
        // mode
        let mode = self.mode.unwrap_or(MotionMode::Static);
        validate_duration(self.duration)?;
        if matches!(mode, MotionMode::Static)
            && self
                .duration
                .is_some_and(|duration| duration > STATIC_MAX_DURATION as f64)
        {
            return Err(Error::invalid_duration());
        }
        let available_dynamic_intervals = positions
            .len()
            .checked_sub(1)
            .ok_or_else(Error::wrong_positions)?;
        let simulation_step_count = match mode {
            MotionMode::Static => self
                .duration
                .map(|duration| {
                    planned_interval_count(duration, sample_rate, None)
                })
                .transpose()?
                .unwrap_or(0),
            MotionMode::Dynamic => self
                .duration
                .map(|duration| {
                    planned_interval_count(
                        duration,
                        sample_rate,
                        Some(available_dynamic_intervals),
                    )
                })
                .transpose()?
                .unwrap_or(available_dynamic_intervals),
            MotionMode::UserControl => 0,
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
                // Ephemeris time override logic (-T flag):
                // This logic shifts the ephemerides' TOC/TOE to match the
                // simulation start time.
                //
                // CRITICAL DIFFERENCE vs OLD RUST IMPLEMENTATION:
                // Previously, the Rust version would greedily select the first
                // ephemeris set when time_override was enabled,
                // ignoring the validity of the time window. The
                // C version, however, correctly searches for the *most
                // relevant* ephemeris set by checking if the
                // (adjusted) TOC falls within +/- 2 hours of the simulation
                // time.
                //
                // Correct behavior (C-aligned):
                // 1. Adjust ALL ephemeris sets by shifting their TOC/TOE.
                // 2. Later in the code (see "Select the current set of
                //    ephemerides"), STRICTLY select the ephemeris set where
                //    |TOC - SimTime| < 2 hours.
                //
                // This ensures that even with a time override, we use the
                // ephemeris parameters that are physically most
                // relevant to the target orbital position (e.g. choosing
                // "Monday's" ephemeris for a Monday simulation, even if we
                // shifted the year).

                // Round to nearest 2-hour boundary (7200 seconds)
                // This matches the C version's behavior exactly: gtmp.sec =
                // (double)(((int)(g0.sec)) / 7200) * 7200.0;
                let mut gtmp = GpsTime {
                    week: gps_time_0.week,
                    sec: f64::from((gps_time_0.sec as i32) / 7200) * 7200.0,
                };
                // Overwrite the UTC reference week number
                let dsec = gtmp.diff_secs(&gpstime_min);
                // In C version, this is setting ionoutc.wnt
                // Make sure we're setting the correct field in Rust version
                ionoutc.week_number = gtmp.week;
                ionoutc.tot = gtmp.sec as i32;
                // Iono/UTC parameters may no longer valid
                //ionoutc.vflg = FALSE;
                for sv in 0..MAX_SAT {
                    for i_eph in ephemerides.iter_mut().take(count) {
                        if i_eph[sv].vflg {
                            gtmp = i_eph[sv].toc.add_secs(dsec);
                            let time_of_clock = gtmp.to_gps_calendar()?;
                            i_eph[sv].toc = gtmp;
                            i_eph[sv].time_of_clock = time_of_clock;
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

        // If no valid ephemerides found and time_override is true, use the
        // first set
        if valid_ephemerides_index.is_none() && time_override && count > 0 {
            valid_ephemerides_index = Some(0);
        }

        let Some(valid_ephemerides_index) = valid_ephemerides_index else {
            return Err(Error::no_current_ephemerides());
        };
        // Set ionospheric correction based on the disable flag
        // In gpssim.c, when -i flag is used, ionoutc.enable is set to FALSE
        // So when ionospheric_disable is true, ionoutc.enable should be false
        ionoutc.enable = !self.ionospheric_disable.unwrap_or(false);
        let Some(data_format) = self.data_format else {
            return Err(Error::data_format_not_set());
        };

        let generator = SignalGenerator {
            ephemerides,
            valid_ephemerides_index,
            ionoutc,
            positions,
            simulation_step_count,
            duration_seconds: self.duration,
            receiver_gps_time,
            antenna_gains,
            antenna_pattern,
            mode,
            runtime_motion_control: self.runtime_motion_control,
            elevation_mask_degrees: 0.0, // Default elevation mask
            sample_frequency,
            sample_rate,
            data_format,
            fixed_gain: self.path_loss,
            output_file: self.output_file,
            verbose: self.verbose.unwrap_or(false),
            ..Default::default()
        };
        Ok(generator)
    }
}
