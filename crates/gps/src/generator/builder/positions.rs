use std::path::PathBuf;

use constants::R2D;
use geometry::{Ecef, Location};
use parsing::{read_nmea_gga, read_user_motion, read_user_motion_llh};

use super::SignalGeneratorBuilder;
use crate::{
    Error,
    generator::{MotionMode, RuntimeMotionControl},
};

impl SignalGeneratorBuilder {
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

    /// Enables runtime motion control mode.
    ///
    /// When set, the generator runs in [`MotionMode::UserControl`] and obtains
    /// receiver positions at runtime from a [`RuntimeMotionControl`].
    ///
    /// This is mutually exclusive with `location(_ecef)` and motion file
    /// inputs.
    pub fn runtime_motion_control(
        mut self, control: Option<RuntimeMotionControl>,
    ) -> Result<Self, Error> {
        let Some(control) = control else {
            return Ok(self);
        };
        if self.positions.is_some() {
            return Err(Error::duplicate_position());
        }

        self.mode = Some(MotionMode::UserControl);
        self.positions = Some(vec![control.snapshot().position_ecef]);
        self.runtime_motion_control = Some(control);
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
}
