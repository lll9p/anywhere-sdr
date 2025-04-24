use constants::{OMEGA_EARTH, R2D, SECONDS_IN_HALF_WEEK, SECONDS_IN_WEEK};
use geometry::{Azel, Ecef, Location, Neu};

use crate::datetime::{DateTime, GpsTime};

/// Represents the broadcast ephemeris data for a single GPS satellite.
///
/// This structure contains the orbital parameters and clock correction terms
/// that allow a receiver to compute a satellite's position, velocity, and clock
/// offset at any time within the validity period of the ephemeris (typically ±2
/// hours from the Time of Ephemeris).
///
/// The ephemeris parameters follow the Interface Specification IS-GPS-200,
/// which defines the structure of the GPS navigation message. These parameters
/// describe the satellite's orbit using a Keplerian model with perturbation
/// terms.
///
/// The structure contains both the raw broadcast parameters and derived working
/// variables that are pre-computed to improve performance during position
/// calculations.
#[allow(non_snake_case)]
#[derive(Default)]
pub struct Ephemeris {
    /// Flag indicating whether this ephemeris data is valid
    pub vflg: bool,

    /// UTC date and time corresponding to this ephemeris
    pub t: DateTime,

    /// Time of Clock (TOC) - reference time for clock parameters
    pub toc: GpsTime,

    /// Time of Ephemeris (TOE) - reference time for ephemeris parameters
    pub toe: GpsTime,

    /// Issue of Data, Clock - identifies the clock data set
    pub iodc: i32,

    /// Issue of Data, Ephemeris - identifies the ephemeris data set
    pub iode: i32,

    /// Mean motion difference from computed value (radians/sec)
    pub deltan: f64,

    /// Amplitude of cosine harmonic correction to argument of latitude
    /// (radians)
    pub cuc: f64,

    /// Amplitude of sine harmonic correction to argument of latitude (radians)
    pub cus: f64,

    /// Amplitude of cosine harmonic correction to angle of inclination
    /// (radians)
    pub cic: f64,

    /// Amplitude of sine harmonic correction to angle of inclination (radians)
    pub cis: f64,

    /// Amplitude of cosine harmonic correction to orbit radius (meters)
    pub crc: f64,

    /// Amplitude of sine harmonic correction to orbit radius (meters)
    pub crs: f64,

    /// Eccentricity of the satellite orbit (dimensionless)
    pub ecc: f64,

    /// Square root of the semi-major axis (sqrt(meters))
    pub sqrta: f64,

    /// Mean anomaly at reference time (radians)
    pub m0: f64,

    /// Longitude of ascending node at weekly epoch (radians)
    pub omg0: f64,

    /// Inclination angle at reference time (radians)
    pub inc0: f64,

    /// Argument of perigee (radians)
    pub aop: f64,

    /// Rate of right ascension (radians/sec)
    pub omgdot: f64,

    /// Rate of inclination angle (radians/sec)
    pub idot: f64,

    /// Satellite clock bias (seconds)
    pub af0: f64,

    /// Satellite clock drift (seconds/second)
    pub af1: f64,

    /// Satellite clock drift rate (seconds/second²)
    pub af2: f64,

    /// Group delay differential between L1 and L2 (seconds)
    pub tgd: f64,

    /// Satellite health status
    pub svhlth: i32,

    /// Code on L2 channel
    pub codeL2: i32,

    /// --- Derived working variables ---

    /// Mean motion - average angular velocity (radians/second)
    pub n: f64,

    /// Square root of (1 - eccentricity²)
    pub sq1e2: f64,

    /// Semi-major axis of the orbit (meters)
    pub A: f64,

    /// Corrected rate of right ascension (omgdot - `OMEGA_EARTH`)
    /// (radians/second)
    pub omgkdot: f64,
}
impl Ephemeris {
    /// Computes satellite position, velocity, and clock correction at a given
    /// time.
    ///
    /// This method implements the algorithm described in the GPS Interface
    /// Specification (IS-GPS-200) to calculate a satellite's position and
    /// velocity in Earth-Centered, Earth-Fixed (ECEF) coordinates, as well
    /// as its clock correction terms.
    ///
    /// The algorithm follows these steps:
    /// 1. Calculate time difference from ephemeris reference time (TOE)
    /// 2. Solve Kepler's equation to find the eccentric anomaly
    /// 3. Calculate true anomaly and argument of latitude
    /// 4. Compute orbit radius and inclination with correction terms
    /// 5. Calculate positions in orbital plane
    /// 6. Compute Earth rotation correction
    /// 7. Transform to ECEF coordinates
    /// 8. Calculate satellite clock correction
    ///
    /// Reference: "Computing Satellite Velocity using the Broadcast Ephemeris"
    /// <http://www.ngs.noaa.gov/gps-toolbox/bc_velo.htm>
    ///
    /// # Arguments
    /// * `time` - GPS time at which to compute the satellite state
    ///
    /// # Returns
    /// A tuple containing:
    /// * `[f64; 3]` - Satellite position in ECEF coordinates (X, Y, Z) in
    ///   meters
    /// * `[f64; 3]` - Satellite velocity in ECEF coordinates (Vx, Vy, Vz) in
    ///   meters/second
    /// * `[f64; 2]` - Clock correction terms: [clock bias in seconds, clock
    ///   drift in seconds/second]
    #[inline]
    pub fn compute_satellite_state(
        &self, time: &GpsTime,
    ) -> ([f64; 3], [f64; 3], [f64; 2]) {
        // Time normalization function (handles GPS week rollover)
        let normalize_time = |current_time: f64, reference_time: f64| {
            let mut time_diff = current_time - reference_time;
            if time_diff > SECONDS_IN_HALF_WEEK {
                time_diff -= SECONDS_IN_WEEK;
            } else if time_diff < -SECONDS_IN_HALF_WEEK {
                time_diff += SECONDS_IN_WEEK;
            }
            time_diff
        };
        // Calculate normalized time relative to ephemeris reference time
        let tk = normalize_time(time.sec, self.toe.sec);

        // 1. Calculate eccentric anomaly (Ek) by iteratively solving Kepler's
        //    equation
        let mk = self.m0 + self.n * tk;
        let (mut ek, mut ek_prev) = (mk, mk + 1.0);
        let mut one_minusecos_e = 0.0; // Suppress the uninitialized warning.
        while (ek - ek_prev).abs() > 1.0e-14 {
            ek_prev = ek;
            one_minusecos_e = 1.0 - self.ecc * ek_prev.cos();
            ek += (mk - ek_prev + self.ecc * (ek_prev.sin())) / one_minusecos_e;
        }

        // 2. Calculate orbital parameters
        let (sek, cek) = ek.sin_cos();
        let ekdot = self.n / one_minusecos_e;
        let relativistic = -4.442_807_633E-10 * self.ecc * self.sqrta * sek;
        let pk = (self.sq1e2 * sek).atan2(cek - self.ecc) + self.aop;
        let pkdot = self.sq1e2 * ekdot / one_minusecos_e;

        // 3. Calculate latitude parameters and corrections
        let (s2pk, c2pk) = (2.0 * pk).sin_cos();
        let uk = pk + self.cus * s2pk + self.cuc * c2pk;
        let (suk, cuk) = uk.sin_cos();
        let ukdot = pkdot * (1.0 + 2.0 * (self.cus * c2pk - self.cuc * s2pk));
        let rk = self.A * one_minusecos_e + self.crc * c2pk + self.crs * s2pk;
        let rkdot = self.A * self.ecc * sek * ekdot
            + 2.0 * pkdot * (self.crs * c2pk - self.crc * s2pk);
        let ik = self.inc0 + self.idot * tk + self.cic * c2pk + self.cis * s2pk;
        let (sik, cik) = ik.sin_cos();
        let ikdot =
            self.idot + 2.0 * pkdot * (self.cis * c2pk - self.cic * s2pk);

        // 4. Calculate satellite position and velocity
        let xpk = rk * cuk;
        let ypk = rk * suk;
        let xpkdot = rkdot * cuk - ypk * ukdot;
        let ypkdot = rkdot * suk + xpk * ukdot;
        let ok = self.omg0 + tk * self.omgkdot - OMEGA_EARTH * self.toe.sec;
        let (sok, cok) = ok.sin_cos();
        let pos = [
            xpk * cok - ypk * cik * sok,
            xpk * sok + ypk * cik * cok,
            ypk * sik,
        ];
        let tmp = ypkdot * cik - ypk * sik * ikdot;
        let vel = [
            -self.omgkdot * pos[1] + xpkdot * cok - tmp * sok,
            self.omgkdot * pos[0] + xpkdot * sok + tmp * cok,
            ypk * cik * ikdot + ypkdot * sik,
        ];
        // 5. Calculate satellite clock corrections
        let tk = normalize_time(time.sec, self.toc.sec);
        let clk = [
            self.af0 + tk * (self.af1 + tk * self.af2) + relativistic
                - self.tgd,
            self.af1 + 2.0 * tk * self.af2,
        ];
        (pos, vel, clk)
        // clk[0] = eph.af0 + tk * (eph.af1 + tk * eph.af2) + relativistic -
        // eph.tgd; clk[1] = eph.af1 + 2.0 * tk * eph.af2;
    }

    /// Checks if a satellite is visible from a given receiver position.
    ///
    /// This method determines whether a satellite is visible to a receiver at a
    /// specific location, taking into account the satellite's position and
    /// an elevation mask angle. A satellite is considered visible if:
    /// 1. The ephemeris data is valid
    /// 2. The satellite's elevation angle is above the specified mask angle
    ///
    /// The method calculates the azimuth and elevation angles to the satellite
    /// and compares the elevation with the mask angle to determine
    /// visibility.
    ///
    /// # Arguments
    /// * `time` - GPS time at which to check visibility
    /// * `xyz` - Receiver position in ECEF coordinates
    /// * `elv_mask` - Elevation mask angle in degrees (satellites below this
    ///   angle are considered invisible)
    ///
    /// # Returns
    /// * `None` - If the ephemeris data is invalid
    /// * `Some((azel, false))` - If the satellite is below the elevation mask
    ///   (invisible)
    /// * `Some((azel, true))` - If the satellite is visible, with its azimuth
    ///   and elevation angles
    #[inline]
    pub fn check_visibility(
        &self, time: &GpsTime, xyz: &Ecef, elv_mask: f64,
    ) -> Option<(Azel, bool)> {
        if !self.vflg {
            return None; // Invalid ephemeris
        }

        // Compute satellite position
        let (pos, _vel, _clk) = self.compute_satellite_state(time);

        // Convert receiver position to geodetic coordinates
        let llh = Location::from(xyz);

        // Calculate line-of-sight vector from receiver to satellite
        let los = Ecef::from(&pos) - xyz;

        // Convert to local North-East-Up coordinates
        let neu = Neu::from_ecef(&los, llh.ltcmat());

        // Convert to azimuth and elevation angles
        let azel = Azel::from(&neu);

        // Check if elevation is above the mask angle
        if azel.el * R2D <= elv_mask {
            return Some((azel, false)); // Below elevation mask (invisible)
        }

        Some((azel, true)) // Visible
    }
}
