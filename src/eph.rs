use crate::{datetime_t, gpstime_t};
//  Structure representing ephemeris of a single satellite
#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct ephem_t {
    // < Valid Flag
    pub vflg: i32,
    pub t: datetime_t,
    // < Time of Clock
    pub toc: gpstime_t,
    // < Time of Ephemeris
    pub toe: gpstime_t,
    // < Issue of Data, Clock
    pub iodc: i32,
    // < Isuse of Data, Ephemeris
    pub iode: i32,
    // < Delta-N (radians/sec)
    pub deltan: f64,
    // < Cuc (radians)
    pub cuc: f64,
    // < Cus (radians)
    pub cus: f64,
    // < Correction to inclination cos (radians)
    pub cic: f64,
    // < Correction to inclination sin (radians)
    pub cis: f64,
    // < Correction to radius cos (meters)
    pub crc: f64,
    // < Correction to radius sin (meters)
    pub crs: f64,
    // < e Eccentricity
    pub ecc: f64,
    // < sqrt(A) (sqrt(m))
    pub sqrta: f64,
    // < Mean anamoly (radians)
    pub m0: f64,
    // < Longitude of the ascending node (radians)
    pub omg0: f64,
    // < Inclination (radians)
    pub inc0: f64,
    pub aop: f64,
    // < Omega dot (radians/s)
    pub omgdot: f64,
    // < IDOT (radians/s)
    pub idot: f64,
    // < Clock offset (seconds)
    pub af0: f64,
    // < rate (sec/sec)
    pub af1: f64,
    // < acceleration (sec/sec^2)
    pub af2: f64,
    // < Group delay L2 bias
    pub tgd: f64,
    pub svhlth: i32,
    // Working variables follow
    pub codeL2: i32,
    // < Mean motion (Average angular velocity)
    pub n: f64,
    // < sqrt(1-e^2)
    pub sq1e2: f64,
    // < Semi-major axis
    pub A: f64,
    // < OmegaDot-OmegaEdot
    pub omgkdot: f64,
}
