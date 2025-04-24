use constants::*;

use crate::traits::LocationMath;

/// Geodetic coordinates in Latitude-Longitude-Height (LLH) system
/// - Latitude: Degrees north/south (-90° to 90°)
/// - Longitude: Degrees east/west (-180° to 180°)
/// - Height: Meters above WGS84 ellipsoid
#[derive(Debug, Clone, Copy, Default)]
pub struct Location {
    pub latitude: f64,
    pub longitude: f64,
    pub height: f64,
}
impl Location {
    /// Constructs new LLH coordinates with angular degrees
    pub fn new(latitude: f64, longitude: f64, height: f64) -> Self {
        Self {
            latitude,
            longitude,
            height,
        }
    }

    /// Converts angular degrees to radians for calculations
    pub fn to_rad(&self) -> Self {
        Self {
            latitude: self.latitude.to_radians(),
            longitude: self.longitude.to_radians(),
            height: self.height,
        }
    }

    /// Computes Local Tangent Plane (ENU) rotation matrix
    /// Returns 3x3 rotation matrix `[[e_x, n_x, u_x], ...]`
    /// where:
    /// - e: East-axis components
    /// - n: North-axis components
    /// - u: Up-axis components
    pub fn ltcmat(&self) -> [[f64; 3]; 3] {
        let (slat, clat) = self.latitude.sin_cos();
        let (slon, clon) = self.longitude.sin_cos();
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
        let lat1 = self.latitude.to_radians();
        let lon1 = self.longitude.to_radians();
        let lat2 = other.latitude.to_radians();
        let lon2 = other.longitude.to_radians();
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
        let d_lat = (other.latitude - self.latitude).to_radians();
        let d_lon = (other.longitude - self.longitude).to_radians();

        let a = (d_lat / 2.0).sin().powi(2)
            + self.latitude.to_radians().cos()
                * other.latitude.to_radians().cos()
                * (d_lon / 2.0).sin().powi(2);

        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
        let d = R * c;

        d * 1000.0 // Convert from kilometers to meters
    }
}
impl LocationMath for Location {
    fn norm(&self) -> f64 {
        (self.latitude.powi(2) + self.longitude.powi(2) + self.height.powi(2))
            .sqrt()
    }

    fn dot_prod(&self, rhs: &Self) -> f64 {
        self.latitude * rhs.latitude
            + self.longitude * rhs.longitude
            + self.height * rhs.height
    }

    #[cfg(test)]
    fn precise(&self, rhs: &Self, eps: f64) -> bool {
        (self.latitude - rhs.latitude).abs() <= eps
            && (self.longitude - rhs.longitude).abs() <= eps
            && (self.height - rhs.height).abs() <= eps
    }
}
impl std::ops::Sub for Location {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            latitude: self.latitude - rhs.latitude,
            longitude: self.longitude - rhs.longitude,
            height: self.height - rhs.height,
        }
    }
}
impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{:.6}, {:.6}, {:.6}]",
            self.latitude, self.longitude, self.height
        )
    }
}
/// Earth-Centered Earth-Fixed (ECEF) Cartesian coordinates
/// - X: Through equator at 0° longitude
/// - Y: Through equator at 90° east
/// - Z: Through north pole
#[derive(Debug, Clone, Copy, Default)]
pub struct Ecef {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
impl Ecef {
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
    pub north: f64,
    pub east: f64,
    pub up: f64,
}
impl Neu {
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

/// Azimuth-Elevation pair for directional calculations
/// - Azimuth: Clockwise angle from north (0°-360°)
/// - Elevation: Angle above horizon (0°-90°)
#[derive(Debug, Clone, Copy, Default)]
pub struct Azel {
    pub az: f64,
    pub el: f64,
}

#[derive(Debug)]
pub struct NavigationTarget {
    bearing_step: f64,
    bearing: f64,       // 0~360°
    location: Location, // current location
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
    pub fn new() -> Self {
        Self::default()
    }

    fn truncate_bearing(bearing: f64) -> f64 {
        (bearing + 360.0) % 360.0
    }

    pub fn inc_bearing(&mut self) {
        let bearing = (self.bearing + self.bearing_step) % 360.0;
        self.bearing = Self::truncate_bearing(bearing);
    }

    pub fn dec_bearing(&mut self) {
        let bearing = (self.bearing - self.bearing_step) % 360.0;
        self.bearing = Self::truncate_bearing(bearing);
    }

    pub fn set_location(&mut self, location: Location) -> &mut Self {
        self.location = location;
        self
    }

    pub fn bearing(&self, location: &Location) -> f64 {
        let lat1 = self.location.latitude.to_radians();
        let lon1 = self.location.longitude.to_radians();
        let lat2 = location.latitude.to_radians();
        let lon2 = location.longitude.to_radians();
        let y = (lat2 - lat1) * (lat2 + lat1).cos();
        let x = (lon2 - lon1) * (lon2 + lon1).cos();
        y.atan2(x).to_degrees()
    }

    pub fn go(&mut self, distance: f64) -> Location {
        let location_rad = self.location.to_rad();
        let lat1 = location_rad.latitude;
        let lon1 = location_rad.longitude;
        let bearing = self.bearing.to_radians();
        let distance = distance / WGS84_RADIUS;
        let lat2 = (lat1.sin() * distance.cos()
            + lat1.cos() * distance.sin() * bearing.cos())
        .asin();
        let lon2 = lon1
            + (bearing.sin() * distance.sin() * lat1.cos())
                .atan2(distance.cos() - lat1.sin() * lat2.sin());
        let new_location = Location::new(
            lat2.to_degrees(),
            lon2.to_degrees(),
            self.location.height,
        );
        self.location = new_location;
        new_location
    }
}
