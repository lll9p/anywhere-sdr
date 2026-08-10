use std::f64::consts::PI;

use constants::WGS84_RADIUS;

use crate::{Error, Location};

/// Represents a navigation target with bearing and location information.
#[derive(Debug)]
pub struct NavigationTarget {
    /// Step size used by bearing increment/decrement operations, in degrees.
    bearing_step: f64,
    /// Current bearing in degrees.
    bearing: f64,
    /// Current validated geodetic location.
    location: Location,
}

impl Default for NavigationTarget {
    fn default() -> Self {
        Self {
            bearing_step: 1.0,
            bearing: 0.0,
            location: Location::default(),
        }
    }
}

impl NavigationTarget {
    /// Creates a new `NavigationTarget` with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Normalizes a bearing into the range `[0, 360)` degrees.
    fn truncate_bearing(bearing: f64) -> f64 {
        (bearing + 360.0) % 360.0
    }

    /// Increments the current bearing by the bearing step.
    pub fn inc_bearing(&mut self) {
        let bearing = (self.bearing + self.bearing_step) % 360.0;
        self.bearing = Self::truncate_bearing(bearing);
    }

    /// Decrements the current bearing by the bearing step.
    pub fn dec_bearing(&mut self) {
        let bearing = (self.bearing - self.bearing_step) % 360.0;
        self.bearing = Self::truncate_bearing(bearing);
    }

    /// Sets the current location.
    pub fn set_location(&mut self, location: Location) -> &mut Self {
        self.location = location;
        self
    }

    /// Calculates the initial bearing in the range `[0, 360)` degrees.
    pub fn bearing(&self, location: &Location) -> f64 {
        self.location.bearing(location)
    }

    /// Moves the current location along the current bearing.
    ///
    /// The destination longitude is canonicalized to `[-π, π)`.
    pub fn go(&mut self, distance_meters: f64) -> Result<Location, Error> {
        let lat1 = self.location.latitude_radians();
        let lon1 = self.location.longitude_radians();
        let bearing = self.bearing.to_radians();
        let angular_distance = distance_meters / WGS84_RADIUS;
        let lat2 = (lat1.sin() * angular_distance.cos()
            + lat1.cos() * angular_distance.sin() * bearing.cos())
        .asin();
        let lon2 = lon1
            + (bearing.sin() * angular_distance.sin() * lat1.cos())
                .atan2(angular_distance.cos() - lat1.sin() * lat2.sin());
        let lon2 = (lon2 + PI).rem_euclid(2.0 * PI) - PI;
        let new_location = Location::try_from_radians(
            lat2,
            lon2,
            self.location.height_meters(),
        )?;
        self.location = new_location;
        Ok(new_location)
    }
}
