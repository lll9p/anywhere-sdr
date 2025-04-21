//! <http://www.movable-type.co.uk/scripts/latlong.html>
use crate::constants::*;
pub trait LocationMath {
    fn norm(&self) -> f64;
    fn dot_prod(&self, _rhs: &Self) -> f64;
    fn precise(&self, rhs: &Self, eps: f64) -> bool;
}

/// LLH format
#[derive(Debug, Clone, Copy, Default)]
pub struct Location {
    pub latitude: f64,
    pub longitude: f64,
    pub height: f64,
}
impl Location {
    pub fn new(latitude: f64, longitude: f64, height: f64) -> Self {
        Self {
            latitude,
            longitude,
            height,
        }
    }

    pub fn to_rad(&self) -> Self {
        Self {
            latitude: self.latitude.to_radians(),
            longitude: self.longitude.to_radians(),
            height: self.height,
        }
    }

    pub fn ltcmat(&self) -> [[f64; 3]; 3] {
        let (slat, clat) = self.latitude.sin_cos();
        let (slon, clon) = self.longitude.sin_cos();
        [[-slat * clon, -slat * slon, clat], [-slon, clon, 0.0], [
            clat * clon,
            clat * slon,
            slat,
        ]]
    }

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

    pub fn measure(&self, other: &Self) -> f64 {
        const R: f64 = 6378.137; // 地球半径，单位为千米
        let d_lat = (other.latitude - self.latitude).to_radians();
        let d_lon = (other.longitude - self.longitude).to_radians();

        let a = (d_lat / 2.0).sin().powi(2)
            + self.latitude.to_radians().cos()
                * other.latitude.to_radians().cos()
                * (d_lon / 2.0).sin().powi(2);

        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
        let d = R * c;

        d * 1000.0 // 返回单位为米
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

    fn precise(&self, rhs: &Self, eps: f64) -> bool {
        (self.latitude - rhs.latitude).abs() <= eps
            && (self.longitude - rhs.longitude).abs() <= eps
            && (self.height - rhs.height).abs() <= eps
    }
}
impl From<&Ecef> for Location {
    fn from(ecef: &Ecef) -> Self {
        let a: f64 = WGS84_RADIUS;
        let eps: f64 = 1.0e-3;
        let e: f64 = WGS84_ECCENTRICITY;
        let e2: f64 = e.powi(2);

        let mut dz: f64;
        let mut zdz: f64;
        let mut nh: f64;
        let mut slat: f64;
        let mut n: f64;
        let mut dz_new: f64;
        if ecef.norm() < eps {
            // Invalid ECEF vector
            return Self {
                latitude: 0.,
                longitude: 0.,
                height: -a,
            };
        }
        let x = ecef.x;
        let y = ecef.y;
        let z = ecef.z;
        let rho2: f64 = x * x + y * y;
        dz = e2 * z;
        loop {
            zdz = z + dz;
            nh = (rho2 + zdz * zdz).sqrt();
            slat = zdz / nh;
            n = a / (1.0 - e2 * slat * slat).sqrt();
            dz_new = n * e2 * slat;
            if (dz - dz_new).abs() < eps {
                break;
            }

            dz = dz_new;
        }
        let llh0 = zdz.atan2(rho2.sqrt());
        let llh1 = y.atan2(x);
        let llh2 = nh - n;

        Self {
            latitude: llh0,
            longitude: llh1,
            height: llh2,
        }
    }
}
impl From<&[f64; 3]> for Location {
    fn from(value: &[f64; 3]) -> Self {
        Self {
            latitude: value[0],
            longitude: value[1],
            height: value[2],
        }
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
/// ECEF format
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

    fn precise(&self, rhs: &Self, eps: f64) -> bool {
        (self.x - rhs.x).abs() <= eps
            && (self.y - rhs.y).abs() <= eps
            && (self.z - rhs.z).abs() <= eps
    }
}
impl From<&Location> for Ecef {
    fn from(loc: &Location) -> Self {
        let a: f64 = WGS84_RADIUS;
        let e: f64 = WGS84_ECCENTRICITY;
        let e2: f64 = e * e;

        let clat: f64 = loc.latitude.cos();
        let slat: f64 = loc.latitude.sin();
        let clon: f64 = loc.longitude.cos();
        let slon: f64 = loc.longitude.sin();
        let d: f64 = e * slat;

        let n: f64 = a / (1. - d.powi(2)).sqrt();
        let nph: f64 = n + loc.height;

        let tmp: f64 = nph * clat;
        let x = tmp * clon;
        let y = tmp * slon;
        let z = ((1. - e2) * n + loc.height) * slat;
        Self { x, y, z }
    }
}
impl From<&[f64; 3]> for Ecef {
    fn from(value: &[f64; 3]) -> Self {
        Self {
            x: value[0],
            y: value[1],
            z: value[2],
        }
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

/// North-East-Up format
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
impl From<&Ecef> for Neu {
    fn from(value: &Ecef) -> Self {
        let ltcmat = Location::from(value).ltcmat();
        Self::from_ecef(value, ltcmat)
    }
}
impl From<&[f64; 3]> for Neu {
    fn from(value: &[f64; 3]) -> Self {
        Self {
            north: value[0],
            east: value[1],
            up: value[2],
        }
    }
}
impl LocationMath for Neu {
    fn norm(&self) -> f64 {
        (self.north.powi(2) + self.east.powi(2) + self.up.powi(2)).sqrt()
    }

    fn dot_prod(&self, rhs: &Self) -> f64 {
        self.north * rhs.north + self.east * rhs.east + self.up * rhs.up
    }

    fn precise(&self, rhs: &Self, eps: f64) -> bool {
        (self.north - rhs.north).abs() <= eps
            && (self.east - rhs.east).abs() <= eps
            && (self.up - rhs.up).abs() <= eps
    }
}

/// bearing + Elevation
#[derive(Debug, Clone, Copy, Default)]
pub struct Azel {
    pub az: f64,
    pub el: f64,
}
impl From<&[f64; 2]> for Azel {
    fn from(value: &[f64; 2]) -> Self {
        Self {
            az: value[0],
            el: value[1],
        }
    }
}

impl From<&Neu> for Azel {
    fn from(neu: &Neu) -> Self {
        let mut az = neu.east.atan2(neu.north);
        if az < 0.0 {
            az += 2.0 * PI;
        }

        let ne: f64 = (neu.north * neu.north + neu.east * neu.east).sqrt();
        let el = neu.up.atan2(ne);
        Self { az, el }
    }
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
mod test {
    #![allow(dead_code, unused)]
    use super::*;
    const LLH: [f64; 3] = [35.274_143_229, 137.014_853_084, 99.998];
    const XYZ: [f64; 3] = [-3_813_477.954, 3_554_276.552, 3_662_785.237];
    const EPS: f64 = 1e-28;
    #[test]
    fn test_geometry_location2efef() {
        let mut xyz = [0.0; 3];
        let xyz = [
            f64::from_bits(13_923_324_196_484_912_872),
            f64::from_bits(4_706_606_011_222_523_641),
            f64::from_bits(13_929_576_035_448_519_812),
        ];
        // xyz = [-1676694.4690794293, 4515052.0724484855, -4167476.3179927487]
        let location = Location::from(&LLH);
        let ecef = Ecef::from(&location);
        let ecef_from_xyz = Ecef::from(&xyz);
        println!("Ecef fro old: {ecef_from_xyz:?}");
        println!("Ecef from new: {ecef:?}");
        assert!(
            ecef.precise(&ecef_from_xyz, EPS),
            "Not equal! {:#?}",
            ecef - &ecef_from_xyz
        );
    }
    #[test]
    fn test_geometry_ecef2location() {
        let llh = [
            f64::from_bits(4_603_720_481_224_739_772),
            f64::from_bits(4_612_567_283_934_169_376),
            f64::from_bits(4_636_737_350_692_634_624),
        ];
        // xyz = [0.6156477194111782, 2.391360502574699, 100.00084324367344]
        let ecef = Ecef::from(&XYZ);
        let location = Location::from(&ecef);
        let location_from_llh = Location::from(&llh);
        println!("Location from old: {location_from_llh:?}");
        println!("Location from new: {location:?}");
        assert!(
            location.precise(&location_from_llh, EPS),
            "Not equal! {:#?}",
            location - location_from_llh
        );
    }
    #[test]
    fn test_geometry_ltcmat() {
        let tmat = [
            [
                f64::from_bits(4_597_406_541_513_150_448),
                f64::from_bits(13_827_093_489_331_282_323),
                f64::from_bits(13_828_338_932_311_870_902),
            ],
            [
                f64::from_bits(4_606_618_994_045_393_047),
                f64::from_bits(4_599_942_923_006_032_601),
                f64::from_bits(0_000_000_000_000_000_000),
            ],
            [
                f64::from_bits(13_821_772_391_745_090_068),
                f64::from_bits(4_604_542_057_699_443_572),
                f64::from_bits(13_827_463_570_854_459_512),
            ],
        ];

        let location = Location::from(&LLH);
        let tmat_from_location = location.ltcmat();
        for (vec0, vec1) in tmat.iter().zip(&tmat_from_location) {
            for (i0, i1) in vec0.iter().zip(vec1) {
                assert!((i0 - i1).abs() <= EPS, "Not equal!");
            }
        }
    }
    #[test]
    fn test_geometry_ecef2neu() {
        let tmat = Location::from(&LLH).ltcmat();
        let neu = [
            f64::from_bits(13_931_381_818_169_503_716),
            f64::from_bits(13_925_646_393_143_350_753),
            f64::from_bits(4_697_507_633_163_841_812),
        ];
        let ecef = Ecef::from(&XYZ);
        let neu = Neu::from(&neu);
        let neu_from_ecef = Neu::from_ecef(&ecef, tmat);
        println!("Neu from old: {neu:?}");
        println!("Neu from new: {neu_from_ecef:?}");
        assert!(neu.precise(&neu_from_ecef, EPS), "Not equal!",);
    }
    #[test]
    fn test_geometry_neu2azel() {
        let tmat = Location::from(&LLH).ltcmat();
        let neu = [
            f64::from_bits(13_931_381_818_169_503_716),
            f64::from_bits(13_925_646_393_143_350_753),
            f64::from_bits(4_697_507_633_163_841_812),
        ];
        let azel = [
            f64::from_bits(4_615_116_355_893_774_375),
            f64::from_bits(4_595_463_099_674_307_653),
        ];
        let neu = Neu::from(&neu);
        let azel = Azel::from(&azel);
        let azel_new = Azel::from(&neu);
        println!("Azel from old: {azel:?}");
        println!("Azel from new: {azel_new:?}");
        assert!(
            (azel.az - azel_new.az).abs() <= EPS
                && (azel.el - azel_new.el).abs() <= EPS,
            "Not equal!"
        );
    }
}
