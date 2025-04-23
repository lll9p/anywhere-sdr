use constants::{OMEGA_EARTH, R2D, SECONDS_IN_HALF_WEEK, SECONDS_IN_WEEK};
use geometry::{Azel, Ecef, Location, Neu};

use crate::datetime::{DateTime, GpsTime};
///  Structure representing ephemeris of a single satellite
#[allow(non_snake_case)]
// #[repr(C)]
#[derive(Debug, Default)]
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
        &self, time: &GpsTime,
    ) -> ([f64; 3], [f64; 3], [f64; 2]) {
        // 时间归一化处理（处理周秒翻转）
        let normalize_time = |current_time: f64, reference_time: f64| {
            let mut time_diff = current_time - reference_time;
            if time_diff > SECONDS_IN_HALF_WEEK {
                time_diff -= SECONDS_IN_WEEK;
            } else if time_diff < -SECONDS_IN_HALF_WEEK {
                time_diff += SECONDS_IN_WEEK;
            }
            time_diff
        };
        // 计算相对于星历参考时间的归一化时间
        let tk = normalize_time(time.sec, self.toe.sec);

        // 1. 计算偏近点角Ek（开普勒方程迭代求解）
        let mk = self.m0 + self.n * tk;
        let (mut ek, mut ek_prev) = (mk, mk + 1.0);
        let mut one_minusecos_e = 0.0; // Suppress the uninitialized warning.
        while (ek - ek_prev).abs() > 1.0e-14 {
            ek_prev = ek;
            one_minusecos_e = 1.0 - self.ecc * ek_prev.cos();
            ek += (mk - ek_prev + self.ecc * (ek_prev.sin())) / one_minusecos_e;
        }

        // 2. 计算轨道参数
        let (sek, cek) = ek.sin_cos();
        let ekdot = self.n / one_minusecos_e;
        let relativistic = -4.442_807_633E-10 * self.ecc * self.sqrta * sek;
        let pk = (self.sq1e2 * sek).atan2(cek - self.ecc) + self.aop;
        let pkdot = self.sq1e2 * ekdot / one_minusecos_e;

        // 3. 计算纬度参数
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

        // 4. 位置和速度计算
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
        // 5. 时钟校正计算
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

    #[inline]
    pub fn check_visibility(
        &self, time: &GpsTime, xyz: &Ecef, elv_mask: f64,
    ) -> Option<(Azel, bool)> {
        if !self.vflg {
            return None; // Invalid
        }
        let (pos, _vel, _clk) = self.compute_satellite_state(time);
        let llh = Location::from(xyz);
        let los = Ecef::from(&pos) - xyz;
        let neu = Neu::from_ecef(&los, llh.ltcmat());
        let azel = Azel::from(&neu);
        if azel.el * R2D <= elv_mask {
            return Some((azel, false)); // Invisible
        }
        Some((azel, true)) // Visible
    }
}
