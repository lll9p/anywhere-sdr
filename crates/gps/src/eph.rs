use crate::{
    constants::*,
    datetime::{DateTime, GpsTime},
    ionoutc::IonoUtc,
    utils::{ecef2neu, ltcmat, neu2azel, sub_vect, xyz2llh},
};
///  Structure representing ephemeris of a single satellite
#[allow(non_snake_case)]
// #[repr(C)]
#[derive(Debug, Clone, Default)]
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
        &self, time: &GpsTime, xyz: &[f64; 3], elv_mask: f64,
    ) -> Option<([f64; 2], bool)> {
        if !self.vflg {
            return None; // Invalid
        }
        let mut llh: [f64; 3] = [0.; 3];
        let mut neu: [f64; 3] = [0.; 3];
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

    /// Converts ephemeris and UTC parameters into GPS navigation message
    /// subframes
    ///
    /// Implements the construction of 5 subframes (each containing 10 30-bit
    /// words) according to IS-GPS-200L specifications. Handles page 18
    /// (ionospheric/UTC parameters) in subframe 4 and page 25 (reserved) in
    /// subframe 5.
    ///
    /// # Arguments
    /// * `eph` - Satellite ephemeris containing orbital parameters and clock
    ///   corrections
    /// * `ionoutc` - Ionospheric delay model and UTC time conversion parameters
    /// * `sbf` - Output buffer for 5 subframes, each represented as [u32;
    ///   `N_DWRD_SBF`]
    ///
    /// # Notes
    /// - Subframes 1-3 contain fundamental ephemeris and clock correction data
    /// - Subframe 4 page 18 includes:
    ///   - Ionospheric α/β coefficients (Klochar model parameters)
    ///   - UTC parameters (A0, A1, `ΔtLS`)
    ///   - Leap second transition parameters
    /// - Subframe 5 page 25 is reserved (zero-filled in this implementation)
    /// - All value conversions follow GPS-ICD-defined scaling factors and
    ///   bit-field layouts
    #[allow(clippy::too_many_lines)]
    pub fn generate_navigation_subframes(
        &self, ionoutc: &IonoUtc, sbf: &mut [[u32; N_DWRD_SBF]; 5],
    ) {
        let ura = 0;
        let data_id = 1;
        let sbf4_page25_sv_id = 63;
        let sbf5_page25_sv_id = 51;
        let wnlsf;
        let dtlsf;
        let dn;
        let sbf4_page18_sv_id = 56;

        // FIXED: This has to be the "transmission" week number, not for the
        // ephemeris reference time wn = (unsigned long)(self.toe.week%1024);
        let wn = 0;
        let toe = (self.toe.sec / 16.0) as u32;
        let toc = (self.toc.sec / 16.0) as u32;
        let iode = self.iode as u32;
        let iodc = self.iodc as u32;
        let deltan = (self.deltan / POW2_M43 / PI) as i32;
        let cuc = (self.cuc / POW2_M29) as i32;
        let cus = (self.cus / POW2_M29) as i32;
        let cic = (self.cic / POW2_M29) as i32;
        let cis = (self.cis / POW2_M29) as i32;
        let crc = (self.crc / POW2_M5) as i32;
        let crs = (self.crs / POW2_M5) as i32;
        let ecc = (self.ecc / POW2_M33) as u32;
        let sqrta = (self.sqrta / POW2_M19) as u32;
        let m0 = (self.m0 / POW2_M31 / PI) as i32;
        let omg0 = (self.omg0 / POW2_M31 / PI) as i32;
        let inc0 = (self.inc0 / POW2_M31 / PI) as i32;
        let aop = (self.aop / POW2_M31 / PI) as i32;
        let omgdot = (self.omgdot / POW2_M43 / PI) as i32;
        let idot = (self.idot / POW2_M43 / PI) as i32;
        let af0 = (self.af0 / POW2_M31) as i32;
        let af1 = (self.af1 / POW2_M43) as i32;
        let af2 = (self.af2 / POW2_M55) as i32;
        let tgd = (self.tgd / POW2_M31) as i32;
        let svhlth = self.svhlth as u32 as i32;

        #[allow(non_snake_case)]
        let codeL2 = self.codeL2 as u32 as i32;
        let wna = (self.toe.week % 256) as u32;
        let toa = (self.toe.sec / 4096.0) as u32;
        let alpha0 = (ionoutc.alpha0 / POW2_M30).round() as i32;
        let alpha1 = (ionoutc.alpha1 / POW2_M27).round() as i32;
        let alpha2 = (ionoutc.alpha2 / POW2_M24).round() as i32;
        let alpha3 = (ionoutc.alpha3 / POW2_M24).round() as i32;
        let beta0 = (ionoutc.beta0 / 2048.0).round() as i32;
        let beta1 = (ionoutc.beta1 / 16384.0).round() as i32;
        let beta2 = (ionoutc.beta2 / 65536.0).round() as i32;
        let beta3 = (ionoutc.beta3 / 65536.0).round() as i32;

        #[allow(non_snake_case)]
        let A0 = (ionoutc.A0 / POW2_M30).round() as i32;

        #[allow(non_snake_case)]
        let A1 = (ionoutc.A1 / POW2_M50).round() as i32;
        let dtls = ionoutc.dtls;
        let tot = (ionoutc.tot / 4096) as u32;
        let week_number = (ionoutc.week_number % 256) as u32;
        // 2016/12/31 (Sat) -> WNlsf = 1929, DN = 7 (http://navigationservices.agi.com/GNSSWeb/)
        // Days are counted from 1 to 7 (Sunday is 1).
        if ionoutc.leapen == 1 {
            wnlsf = (ionoutc.wnlsf % 256) as u32;
            dn = ionoutc.day_number as u32;
            dtlsf = ionoutc.dtlsf as u32;
        } else {
            wnlsf = (1929 % 256) as u32;
            dn = 7;
            dtlsf = 18;
        }
        // Subframe 1
        sbf[0] = [
            0x008b_0000 << 6,
            0x1 << 8,
            (wn & 0x3ff) << 20
                | (codeL2 as u32 & 0x3) << 18
                | (ura & 0xf) << 14
                | (svhlth as u32 & 0x3f) << 8
                | (iodc >> 8 & 0x3) << 6,
            0,
            0,
            0,
            (tgd as u32 & 0xff) << 6,
            (iodc & 0xff) << 22 | (toc & 0xffff) << 6,
            (af2 as u32 & 0xff) << 22 | (af1 as u32 & 0xffff) << 6,
            (af0 as u32 & 0x003f_ffff) << 8,
        ];
        // Subframe 2
        sbf[1] = [
            0x008b_0000 << 6,
            0x2 << 8,
            (iode & 0xff) << 22 | (crs as u32 & 0xffff) << 6,
            (deltan as u32 & 0xffff) << 14 | ((m0 >> 24) as u32 & 0xff) << 6,
            (m0 as u32 & 0x00ff_ffff) << 6,
            (cuc as u32 & 0xffff) << 14 | (ecc >> 24 & 0xff) << 6,
            (ecc & 0x00ff_ffff) << 6,
            (cus as u32 & 0xffff) << 14 | (sqrta >> 24 & 0xff) << 6,
            (sqrta & 0x00ff_ffff) << 6,
            (toe & 0xffff) << 14,
        ];
        // Subframe 3
        sbf[2] = [
            0x008b_0000 << 6,
            0x3 << 8,
            (cic as u32 & 0xffff) << 14 | ((omg0 >> 24) as u32 & 0xff) << 6,
            (omg0 as u32 & 0x00ff_ffff) << 6,
            (cis as u32 & 0xffff) << 14 | ((inc0 >> 24) as u32 & 0xff) << 6,
            (inc0 as u32 & 0x00ff_ffff) << 6,
            (crc as u32 & 0xffff) << 14 | ((aop >> 24) as u32 & 0xff) << 6,
            (aop as u32 & 0x00ff_ffff) << 6,
            (omgdot as u32 & 0x00ff_ffff) << 6,
            (iode & 0xff) << 22 | (idot as u32 & 0x3fff) << 8,
        ];
        if ionoutc.vflg {
            // Subframe 4, page 18
            sbf[3] = [
                0x008b_0000 << 6,
                0x4 << 8,
                data_id << 28
                    | sbf4_page18_sv_id << 22
                    | (alpha0 as u32 & 0xff) << 14
                    | (alpha1 as u32 & 0xff) << 6,
                (alpha2 as u32 & 0xff) << 22
                    | (alpha3 as u32 & 0xff) << 14
                    | (beta0 as u32 & 0xff) << 6,
                (beta1 as u32 & 0xff) << 22
                    | (beta2 as u32 & 0xff) << 14
                    | (beta3 as u32 & 0xff) << 6,
                (A1 as u32 & 0x00ff_ffff) << 6,
                ((A0 >> 8) as u32 & 0x00ff_ffff) << 6,
                (A0 as u32 & 0xff) << 22
                    | (tot & 0xff) << 14
                    | (week_number & 0xff) << 6,
                (dtls as u32 & 0xff) << 22
                    | (wnlsf & 0xff) << 14
                    | (dn & 0xff) << 6,
                (dtlsf & 0xff) << 22,
            ];
        } else {
            // Subframe 4, page 25
            sbf[3] = [
                0x008b_0000 << 6,
                0x4 << 8,
                data_id << 28 | sbf4_page25_sv_id << 22,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ];
        }
        // Subframe 5, page 25
        sbf[4] = [
            0x008b_0000 << 6,
            0x5 << 8,
            data_id << 28
                | sbf5_page25_sv_id << 22
                | (toa & 0xff) << 14
                | (wna & 0xff) << 6,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ];
    }
}
