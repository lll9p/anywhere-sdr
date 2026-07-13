use std::f64::consts::PI;

use constants::R2D;

use crate::{Error, traits::LocationMath};

/// Validated WGS-84 geodetic coordinates with canonical radian angles.
///
/// Raw values must enter through [`Location::try_from_degrees`] or
/// [`Location::try_from_radians`].
///
/// ```
/// use geometry::{Ecef, Location};
///
/// # fn main() -> Result<(), geometry::Error> {
/// let tokyo = Location::try_from_degrees(35.681_298, 139.766_247, 10.0)?;
/// assert!(
///     (tokyo.latitude_radians() - 35.681_298_f64.to_radians()).abs() < 1e-12
/// );
///
/// let ecef = Ecef::from(&tokyo);
/// let round_trip = Location::try_from(&ecef)?;
/// assert!((round_trip.longitude_degrees() - 139.766_247).abs() < 1e-8);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct Location {
    /// Geodetic latitude in radians.
    latitude_radians: f64,
    /// Geodetic longitude in radians.
    longitude_radians: f64,
    /// Height above the WGS-84 ellipsoid in meters.
    height_meters: f64,
}
impl Location {
    /// Constructs validated geodetic coordinates from radians and meters.
    pub fn try_from_radians(
        latitude_radians: f64, longitude_radians: f64, height_meters: f64,
    ) -> Result<Self, Error> {
        if !latitude_radians.is_finite()
            || !longitude_radians.is_finite()
            || !height_meters.is_finite()
            || !(-PI / 2.0..=PI / 2.0).contains(&latitude_radians)
            || !(-PI..=PI).contains(&longitude_radians)
        {
            return Err(Error::invalid_coordinates(
                latitude_radians,
                longitude_radians,
                height_meters,
            ));
        }

        Ok(Self {
            latitude_radians,
            longitude_radians,
            height_meters,
        })
    }

    /// Constructs validated geodetic coordinates from degrees and meters.
    pub fn try_from_degrees(
        latitude_degrees: f64, longitude_degrees: f64, height_meters: f64,
    ) -> Result<Self, Error> {
        if !latitude_degrees.is_finite()
            || !longitude_degrees.is_finite()
            || !height_meters.is_finite()
            || !(-90.0..=90.0).contains(&latitude_degrees)
            || !(-180.0..=180.0).contains(&longitude_degrees)
        {
            return Err(Error::invalid_coordinates(
                latitude_degrees / R2D,
                longitude_degrees / R2D,
                height_meters,
            ));
        }
        // Preserve the established GPS/C conversion ratio at degree-based
        // input boundaries while keeping the stored representation explicit.
        Self::try_from_radians(
            latitude_degrees / R2D,
            longitude_degrees / R2D,
            height_meters,
        )
    }

    /// Returns latitude in radians.
    pub fn latitude_radians(&self) -> f64 {
        self.latitude_radians
    }

    /// Returns longitude in radians.
    pub fn longitude_radians(&self) -> f64 {
        self.longitude_radians
    }

    /// Returns latitude in degrees.
    pub fn latitude_degrees(&self) -> f64 {
        self.latitude_radians.to_degrees()
    }

    /// Returns longitude in degrees.
    pub fn longitude_degrees(&self) -> f64 {
        self.longitude_radians.to_degrees()
    }

    /// Returns height above the WGS-84 ellipsoid in meters.
    pub fn height_meters(&self) -> f64 {
        self.height_meters
    }

    /// Computes Local Tangent Plane (ENU) rotation matrix
    /// Returns 3x3 rotation matrix `[[e_x, n_x, u_x], ...]`
    /// where:
    /// - e: East-axis components
    /// - n: North-axis components
    /// - u: Up-axis components
    pub fn ltcmat(&self) -> [[f64; 3]; 3] {
        let (slat, clat) = self.latitude_radians.sin_cos();
        let (slon, clon) = self.longitude_radians.sin_cos();
        [
            [-slat * clon, -slat * slon, clat], // East components
            [-slon, clon, 0.0],                 // North components
            [clat * clon, clat * slon, slat],   // Up components
        ]
    }

    /// Calculates initial bearing between two points using:
    /// θ = atan2(sinΔλ·cosφ2, cosφ1·sinφ2 − sinφ1·cosφ2·cosΔλ)
    /// Returns bearing in degrees (0°-360°)
    pub fn bearing(&self, other: &Self) -> f64 {
        let lat1 = self.latitude_radians;
        let lon1 = self.longitude_radians;
        let lat2 = other.latitude_radians;
        let lon2 = other.longitude_radians;
        let y = (lon2 - lon1).sin() * lat2.cos();
        let x = (lat1.cos()) * (lat2.sin())
            - (lat1.sin()) * (lat2.cos()) * (lon2 - lon1).cos();
        let brng = y.atan2(x).to_degrees();
        (brng + 360.0) % 360.0
    }

    /// Calculates great-circle distance using Haversine formula:
    /// a = sin²(Δφ/2) + cosφ1·cosφ2·sin²(Δλ/2)
    /// c = 2·atan2(√a, √(1−a))
    /// d = R·c
    /// Returns distance in meters
    pub fn measure(&self, other: &Self) -> f64 {
        const R: f64 = 6378.137; // Earth radius in kilometers
        let d_lat = other.latitude_radians - self.latitude_radians;
        let d_lon = other.longitude_radians - self.longitude_radians;

        let a = (d_lat / 2.0).sin().powi(2)
            + self.latitude_radians.cos()
                * other.latitude_radians.cos()
                * (d_lon / 2.0).sin().powi(2);

        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
        let d = R * c;

        d * 1000.0 // Convert from kilometers to meters
    }
}
impl LocationMath for Location {
    fn norm(&self) -> f64 {
        (self.latitude_radians.powi(2)
            + self.longitude_radians.powi(2)
            + self.height_meters.powi(2))
        .sqrt()
    }

    fn dot_prod(&self, rhs: &Self) -> f64 {
        self.latitude_radians * rhs.latitude_radians
            + self.longitude_radians * rhs.longitude_radians
            + self.height_meters * rhs.height_meters
    }

    #[cfg(test)]
    fn precise(&self, rhs: &Self, eps: f64) -> bool {
        (self.latitude_radians - rhs.latitude_radians).abs() <= eps
            && (self.longitude_radians - rhs.longitude_radians).abs() <= eps
            && (self.height_meters - rhs.height_meters).abs() <= eps
    }
}
impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{:.6} rad, {:.6} rad, {:.3} m]",
            self.latitude_radians, self.longitude_radians, self.height_meters
        )
    }
}
/// Earth-Centered Earth-Fixed (ECEF) Cartesian coordinates
/// - X: Through equator at 0° longitude
/// - Y: Through equator at 90° east
/// - Z: Through north pole
#[derive(Debug, Clone, Copy, Default)]
pub struct Ecef {
    /// X coordinate in meters (through equator at 0° longitude)
    pub x: f64,
    /// Y coordinate in meters (through equator at 90° east)
    pub y: f64,
    /// Z coordinate in meters (through north pole)
    pub z: f64,
}
impl Ecef {
    /// Creates a new ECEF coordinate with the specified values.
    ///
    /// # Arguments
    /// * `x` - X coordinate in meters
    /// * `y` - Y coordinate in meters
    /// * `z` - Z coordinate in meters
    ///
    /// # Returns
    /// A new ECEF coordinate
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
}
impl LocationMath for Ecef {
    fn norm(&self) -> f64 {
        (self.x.powi(2) + self.y.powi(2) + self.z.powi(2)).sqrt()
    }

    fn dot_prod(&self, rhs: &Self) -> f64 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    #[cfg(test)]
    fn precise(&self, rhs: &Self, eps: f64) -> bool {
        (self.x - rhs.x).abs() <= eps
            && (self.y - rhs.y).abs() <= eps
            && (self.z - rhs.z).abs() <= eps
    }
}
impl std::ops::Sub<&Self> for Ecef {
    type Output = Self;

    fn sub(self, rhs: &Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}
impl std::ops::SubAssign for Ecef {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
        self.z -= rhs.z;
    }
}
impl std::ops::Mul<f64> for Ecef {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }
}

/// North-East-Up (NEU) local tangent plane coordinates
/// - North: Local north direction
/// - East: Local east direction
/// - Up: Local vertical direction
#[derive(Debug, Clone, Copy, Default)]
pub struct Neu {
    /// North component in meters
    pub north: f64,
    /// East component in meters
    pub east: f64,
    /// Up (vertical) component in meters
    pub up: f64,
}
impl Neu {
    /// Converts ECEF coordinates to NEU coordinates using a local tangent plane
    /// rotation matrix.
    ///
    /// # Arguments
    /// * `ecef` - ECEF coordinates to convert
    /// * `ltcmat` - Local tangent plane rotation matrix from a reference
    ///   location
    ///
    /// # Returns
    /// NEU coordinates relative to the reference location
    pub fn from_ecef(ecef: &Ecef, ltcmat: [[f64; 3]; 3]) -> Self {
        let north = ltcmat[0][0] * ecef.x
            + ltcmat[0][1] * ecef.y
            + ltcmat[0][2] * ecef.z;
        let east = ltcmat[1][0] * ecef.x
            + ltcmat[1][1] * ecef.y
            + ltcmat[1][2] * ecef.z;
        let up = ltcmat[2][0] * ecef.x
            + ltcmat[2][1] * ecef.y
            + ltcmat[2][2] * ecef.z;
        Self { north, east, up }
    }
}
impl LocationMath for Neu {
    fn norm(&self) -> f64 {
        (self.north.powi(2) + self.east.powi(2) + self.up.powi(2)).sqrt()
    }

    fn dot_prod(&self, rhs: &Self) -> f64 {
        self.north * rhs.north + self.east * rhs.east + self.up * rhs.up
    }

    #[cfg(test)]
    fn precise(&self, rhs: &Self, eps: f64) -> bool {
        (self.north - rhs.north).abs() <= eps
            && (self.east - rhs.east).abs() <= eps
            && (self.up - rhs.up).abs() <= eps
    }
}

/// Validated azimuth/elevation angles with canonical radian storage.
///
/// ```
/// use geometry::Azel;
///
/// # fn main() -> Result<(), geometry::Error> {
/// let direction = Azel::try_from_degrees(90.0, 30.0)?;
/// assert!(
///     (direction.azimuth_radians() - std::f64::consts::FRAC_PI_2).abs()
///         < 1e-12
/// );
/// assert!((direction.elevation_degrees() - 30.0).abs() < 1e-12);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct Azel {
    /// Clockwise azimuth from north in radians.
    azimuth_radians: f64,
    /// Elevation above the local horizon in radians.
    elevation_radians: f64,
}

impl Azel {
    /// Constructs validated azimuth/elevation angles from radians.
    pub fn try_from_radians(
        azimuth_radians: f64, elevation_radians: f64,
    ) -> Result<Self, Error> {
        if !azimuth_radians.is_finite()
            || !elevation_radians.is_finite()
            || !(0.0..2.0 * PI).contains(&azimuth_radians)
            || !(-PI / 2.0..=PI / 2.0).contains(&elevation_radians)
        {
            return Err(Error::invalid_azel(
                azimuth_radians,
                elevation_radians,
            ));
        }

        Ok(Self {
            azimuth_radians,
            elevation_radians,
        })
    }

    /// Constructs validated azimuth/elevation angles from degrees.
    pub fn try_from_degrees(
        azimuth_degrees: f64, elevation_degrees: f64,
    ) -> Result<Self, Error> {
        if !azimuth_degrees.is_finite()
            || !elevation_degrees.is_finite()
            || !(0.0..360.0).contains(&azimuth_degrees)
            || !(-90.0..=90.0).contains(&elevation_degrees)
        {
            return Err(Error::invalid_azel(
                azimuth_degrees / R2D,
                elevation_degrees / R2D,
            ));
        }
        // Keep degree boundaries byte-compatible with existing GPS output.
        Self::try_from_radians(azimuth_degrees / R2D, elevation_degrees / R2D)
    }

    /// Returns azimuth in radians.
    pub fn azimuth_radians(&self) -> f64 {
        self.azimuth_radians
    }

    /// Returns elevation in radians.
    pub fn elevation_radians(&self) -> f64 {
        self.elevation_radians
    }

    /// Returns azimuth in degrees.
    pub fn azimuth_degrees(&self) -> f64 {
        self.azimuth_radians * R2D
    }

    /// Returns elevation in degrees.
    pub fn elevation_degrees(&self) -> f64 {
        self.elevation_radians * R2D
    }
}
