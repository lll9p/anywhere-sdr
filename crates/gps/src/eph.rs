use crate::{
    constants::*,
    datetime::{DateTime, GpsTime},
    utils::{ecef2neu, ltcmat, neu2azel, sub_vect, xyz2llh},
};
///  Structure representing ephemeris of a single satellite
#[allow(non_snake_case)]
// #[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct Ephemeris {
    /// Valid Flag
    pub vflg: bool,
    pub t: DateTime,
    /// Time of Clock
    pub toc: GpsTime,
    /// Time of Ephemeris
    pub toe: GpsTime,
    /// Issue of Data, Clock
    pub iodc: i32,
    /// Isuse of Data, Ephemeris
    pub iode: i32,
    /// Delta-N (radians/sec)
    pub deltan: f64,
    /// Cuc (radians)
    pub cuc: f64,
    /// Cus (radians)
    pub cus: f64,
    /// Correction to inclination cos (radians)
    pub cic: f64,
    /// Correction to inclination sin (radians)
    pub cis: f64,
    /// Correction to radius cos (meters)
    pub crc: f64,
    /// Correction to radius sin (meters)
    pub crs: f64,
    /// e Eccentricity
    pub ecc: f64,
    /// sqrt(A) (sqrt(m))
    pub sqrta: f64,
    /// Mean anamoly (radians)
    pub m0: f64,
    /// Longitude of the ascending node (radians)
    pub omg0: f64,
    /// Inclination (radians)
    pub inc0: f64,
    pub aop: f64,
    /// Omega dot (radians/s)
    pub omgdot: f64,
    /// IDOT (radians/s)
    pub idot: f64,
    /// Clock offset (seconds)
    pub af0: f64,
    /// rate (sec/sec)
    pub af1: f64,
    /// acceleration (sec/sec^2)
    pub af2: f64,
    /// Group delay L2 bias
    pub tgd: f64,
    pub svhlth: i32,
    /// Working variables follow
    pub codeL2: i32,
    /// Mean motion (Average angular velocity)
    pub n: f64,
    /// sqrt(1-e^2)
    pub sq1e2: f64,
    /// Semi-major axis
    pub A: f64,
    /// OmegaDot-OmegaEdot
    pub omgkdot: f64,
}
impl Ephemeris {
    /// \brief Compute Satellite position, velocity and clock at given time
    ///
    /// Computing Satellite Velocity using the Broadcast Ephemeris
    /// <http://www.ngs.noaa.gov/gps-toolbox/bc_velo.htm>
    /// \param[in] eph Ephemeris data of the satellite
    /// \param[in] g GPS time at which position is to be computed
    /// \param[out] pos Computed position (vector)
    /// \param[out] vel Computed velocity (vector)
    /// \param[clk] clk Computed clock
    #[inline]
    pub fn compute_satellite_state(
        &self,
        time: &GpsTime, /* , pos: &mut [f64; 3], vel: &mut [f64; 3],
                        * clk: &mut [f64; 2], */
    ) -> ([f64; 3], [f64; 3], [f64; 2]) {
        let mut tk = time.sec - self.toe.sec;
        if tk > SECONDS_IN_HALF_WEEK {
            tk -= SECONDS_IN_WEEK;
        } else if tk < -SECONDS_IN_HALF_WEEK {
            tk += SECONDS_IN_WEEK;
        }
        let mk = self.m0 + self.n * tk;
        let mut ek = mk;
        let mut ekold = ek + 1.0;
        let mut one_minusecos_e = 0.0; // Suppress the uninitialized warning.
        while (ek - ekold).abs() > 1.0e-14 {
            ekold = ek;
            one_minusecos_e = 1.0 - self.ecc * ekold.cos();
            ek += (mk - ekold + self.ecc * (ekold.sin())) / one_minusecos_e;
        }
        let sek = ek.sin();
        let cek = ek.cos();
        let ekdot = self.n / one_minusecos_e;
        let relativistic = -4.442_807_633E-10 * self.ecc * self.sqrta * sek;
        let pk = (self.sq1e2 * sek).atan2(cek - self.ecc) + self.aop;
        let pkdot = self.sq1e2 * ekdot / one_minusecos_e;
        let s2pk = (2.0 * pk).sin();
        let c2pk = (2.0 * pk).cos();
        let uk = pk + self.cus * s2pk + self.cuc * c2pk;
        let suk = uk.sin();
        let cuk = uk.cos();
        let ukdot = pkdot * (1.0 + 2.0 * (self.cus * c2pk - self.cuc * s2pk));
        let rk = self.A * one_minusecos_e + self.crc * c2pk + self.crs * s2pk;
        let rkdot = self.A * self.ecc * sek * ekdot
            + 2.0 * pkdot * (self.crs * c2pk - self.crc * s2pk);
        let ik = self.inc0 + self.idot * tk + self.cic * c2pk + self.cis * s2pk;
        let sik = ik.sin();
        let cik = ik.cos();
        let ikdot =
            self.idot + 2.0 * pkdot * (self.cis * c2pk - self.cic * s2pk);
        let xpk = rk * cuk;
        let ypk = rk * suk;
        let xpkdot = rkdot * cuk - ypk * ukdot;
        let ypkdot = rkdot * suk + xpk * ukdot;
        let ok = self.omg0 + tk * self.omgkdot - OMEGA_EARTH * self.toe.sec;
        let sok = ok.sin();
        let cok = ok.cos();
        let pos = [
            xpk * cok - ypk * cik * sok,
            xpk * sok + ypk * cik * cok,
            ypk * sik,
        ];
        // pos[0] = xpk * cok - ypk * cik * sok;
        // pos[1] = xpk * sok + ypk * cik * cok;
        // pos[2] = ypk * sik;
        let tmp = ypkdot * cik - ypk * sik * ikdot;
        let vel = [
            -self.omgkdot * pos[1] + xpkdot * cok - tmp * sok,
            self.omgkdot * pos[0] + xpkdot * sok + tmp * cok,
            ypk * cik * ikdot + ypkdot * sik,
        ];
        // vel[0] = -eph.omgkdot * pos[1] + xpkdot * cok - tmp * sok;
        // vel[1] = eph.omgkdot * pos[0] + xpkdot * sok + tmp * cok;
        // vel[2] = ypk * cik * ikdot + ypkdot * sik;
        let mut tk = time.sec - self.toc.sec;
        if tk > SECONDS_IN_HALF_WEEK {
            tk -= SECONDS_IN_WEEK;
        } else if tk < -SECONDS_IN_HALF_WEEK {
            tk += SECONDS_IN_WEEK;
        }
        let clk = [
            self.af0 + tk * (self.af1 + tk * self.af2) + relativistic
                - self.tgd,
            self.af1 + 2.0 * tk * self.af2,
        ];
        (pos, vel, clk)
        // clk[0] = eph.af0 + tk * (eph.af1 + tk * eph.af2) + relativistic -
        // eph.tgd; clk[1] = eph.af1 + 2.0 * tk * eph.af2;
    }

    #[inline]
    pub fn check_visibility(
        &self, time: &GpsTime, xyz: &[f64; 3], elv_mask: f64,
    ) -> Option<([f64; 2], bool)> {
        if !self.vflg {
            return None; // Invalid
        }
        let mut llh: [f64; 3] = [0.; 3];
        let mut neu: [f64; 3] = [0.; 3];
        // let mut pos: [f64; 3] = [0.; 3];
        // let mut vel: [f64; 3] = [0.; 3];
        // modified from [f64;3] to [f64;2]
        // let mut clk: [f64; 2] = [0.; 2];
        let mut los: [f64; 3] = [0.; 3];
        let mut tmat: [[f64; 3]; 3] = [[0.; 3]; 3];
        xyz2llh(xyz, &mut llh);
        ltcmat(&llh, &mut tmat);
        let (pos, _vel, _clk) = self.compute_satellite_state(time);
        sub_vect(&mut los, &pos, xyz);
        ecef2neu(&los, &tmat, &mut neu);
        let mut azel: [f64; 2] = [0.0; 2];
        neu2azel(&mut azel, &neu);
        if azel[1] * R2D <= elv_mask {
            return Some((azel, false)); // Invisible
        }
        Some((azel, true)) // Visible
    }
}
