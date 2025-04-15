use crate::{
    constants::*,
    datetime::{DateTime, GpsTime},
    eph::Ephemeris,
    ionoutc::IonoUtc,
    read_nmea_gga::read_nmea_gga,
    read_rinex::read_rinex_nav_all,
    read_user_motion::{read_user_motion, read_user_motion_llh},
    table::{ANT_PAT_DB, COS_TABLE512, SIN_TABLE512},
};
use std::{
    io::Write,
    path::{Path, PathBuf},
    time::Instant,
};

#[derive(Copy, Clone, Default)]
// #[repr(C)]
pub struct Range {
    pub g: GpsTime,
    // pseudorange
    pub range: f64,
    pub rate: f64,
    // geometric distance
    pub d: f64,
    pub azel: [f64; 2],
    pub iono_delay: f64,
}

//  Structure representing a Channel
#[allow(non_snake_case)]
// #[repr(C)]
#[derive(Copy, Clone)]
pub struct Channel {
    //< PRN Number
    pub prn: i32,
    //< C/A Sequence
    pub ca: [i32; CA_SEQ_LEN],
    //< Carrier frequency
    pub f_carr: f64,
    //< Code frequency
    pub f_code: f64,
    /* #ifdef FLOAT_CARR_PHASE
        double carr_phase;
    #endif */
    //< Carrier phase
    pub carr_phase: u32,
    //< Carrier phasestep
    pub carr_phasestep: i32,
    //< Code phase
    pub code_phase: f64,
    // < GPS time at start
    pub g0: GpsTime,
    // < current subframe
    pub sbf: [[u32; N_DWRD_SBF]; 5],
    // < Data words of sub-frame
    pub dwrd: [u32; N_DWRD],
    // < initial word
    pub iword: i32,
    // < initial bit
    pub ibit: i32,
    // < initial code
    pub icode: i32,
    // < current data bit
    pub dataBit: i32,
    // < current C/A code
    pub codeCA: i32,
    pub azel: [f64; 2],
    pub rho0: Range,
}
#[derive(Clone)]
pub struct Params {
    pub xyz: [[f64; 3]; USER_MOTION_SIZE],
    pub llh: [f64; 3],
    pub ionoutc: IonoUtc,
    pub navfile: PathBuf,
    pub nmea_gga: bool,
    pub um_llh: bool,
    pub umfile: Option<PathBuf>,
    pub timeoverwrite: bool,
    pub static_location_mode: bool,
    pub outfile: PathBuf,
    pub samp_freq: f64,
    pub data_format: i32,
    pub t0: DateTime,
    pub g0: GpsTime,
    pub duration: f64,
    pub fixed_gain: i32,
    pub path_loss_enable: bool,
    pub verb: bool,
}
impl Default for Params {
    fn default() -> Self {
        let g0 = GpsTime {
            week: -1,
            ..Default::default()
        };
        Self {
            xyz: [[0.0; 3]; USER_MOTION_SIZE],
            llh: [0.0; 3],
            ionoutc: IonoUtc::default(),
            navfile: PathBuf::new(),
            nmea_gga: false,
            um_llh: false,
            umfile: None,
            timeoverwrite: false,
            static_location_mode: false,
            outfile: PathBuf::from("gpssim.bin"),
            samp_freq: 2_600_000_f64,
            data_format: 16i32,
            t0: DateTime::default(),
            g0,
            duration: USER_MOTION_SIZE as f64 / 10.0f64,
            fixed_gain: 128,
            path_loss_enable: true,
            verb: false,
        }
    }
}
impl Params {
    fn parse_datetime(
        value: &str,
    ) -> Result<jiff::civil::DateTime, jiff::Error> {
        let time: jiff::civil::DateTime = value.parse()?;
        Ok(time)
    }
    #[allow(
        clippy::too_many_lines,
        clippy::impossible_comparisons,
        clippy::too_many_arguments
    )]
    pub fn new(
        ephemerides: &Path, user_motion_ecef: &Option<PathBuf>,
        user_motion_llh: &Option<PathBuf>, nmea_gga: &Option<PathBuf>,
        location_ecef: Option<Vec<f64>>, location: Option<Vec<f64>>,
        leap: &Option<Vec<i32>>, time: &Option<String>,
        time_override: &Option<String>, duration: &Option<usize>,
        output: &Option<PathBuf>, frequency: usize, bits: usize,
        ionospheric_disable: bool, path_loss: &Option<i32>, verbose: bool,
    ) -> Self {
        let mut params = Params::default();
        params.g0.week = -1; // Invalid start time

        params.navfile = ephemerides.to_path_buf();
        if user_motion_ecef.is_some() {
            params.nmea_gga = false;
            params.um_llh = false;
            params.umfile.clone_from(user_motion_ecef);
        } else if user_motion_llh.is_some() {
            params.um_llh = true;
            params.umfile.clone_from(user_motion_llh);
        } else if nmea_gga.is_some() {
            params.nmea_gga = true;
            params.umfile.clone_from(nmea_gga);
        }

        // Static ECEF coordinates input mode
        if let Some(location) = location_ecef {
            params.static_location_mode = true;
            params.xyz[0][0] = location[0];
            params.xyz[0][1] = location[1];
            params.xyz[0][2] = location[2];
        }
        if let Some(location) = location {
            params.static_location_mode = true;
            params.llh[0] = location[0];
            params.llh[1] = location[1];
            params.llh[2] = location[2];
            params.llh[0] /= R2D;
            params.llh[1] /= R2D;
            llh2xyz(&params.llh, &mut params.xyz[0]);
        }
        params.outfile = PathBuf::from("gpssim.bin");
        if let Some(out) = output {
            params.outfile.clone_from(out);
        }
        assert!(frequency >= 1_000_000, "ERROR: Invalid sampling frequency.");
        params.samp_freq = frequency as f64;
        params.data_format = bits as i32;
        assert!(
            params.data_format == 1
                || params.data_format == 8
                || params.data_format == 16,
            "ERROR: Invalid I/Q data format."
        );
        if let Some(leap) = leap {
            // enable custom Leap Event
            params.ionoutc.leapen = 1;
            params.ionoutc.wnlsf = leap[0];
            params.ionoutc.dn = leap[1];
            params.ionoutc.dtlsf = leap[2];
            assert!(
                params.ionoutc.dn < 1 && params.ionoutc.dn > 7,
                "ERROR: Invalid GPS day number"
            );
            assert!(params.ionoutc.wnlsf < 0, "ERROR: Invalid GPS week number");
            assert!(
                params.ionoutc.dtlsf < -128 && params.ionoutc.dtlsf > 127,
                "ERROR: Invalid delta leap second"
            );
        }
        if let Some(time) = time_override {
            params.timeoverwrite = true;
            if time == "now" {
                let now = jiff::Timestamp::now().in_tz("UTC").unwrap();
                params.t0.y = i32::from(now.year());
                params.t0.m = i32::from(now.month());
                params.t0.d = i32::from(now.day());
                params.t0.hh = i32::from(now.hour());
                params.t0.mm = i32::from(now.minute());
                params.t0.sec = f64::from(now.second());
                date2gps(&params.t0, &mut params.g0);
            } else {
                let time = Self::parse_datetime(time).unwrap();
                params.t0.y = i32::from(time.year());
                params.t0.m = i32::from(time.month());
                params.t0.d = i32::from(time.day());
                params.t0.hh = i32::from(time.hour());
                params.t0.mm = i32::from(time.minute());
                params.t0.sec = f64::from(time.second());
                date2gps(&params.t0, &mut params.g0);
            }
        }
        if let Some(time) = time {
            let time = Self::parse_datetime(time).unwrap();

            params.t0.y = i32::from(time.year());
            params.t0.m = i32::from(time.month());
            params.t0.d = i32::from(time.day());
            params.t0.hh = i32::from(time.hour());
            params.t0.mm = i32::from(time.minute());
            params.t0.sec = f64::from(time.second());

            date2gps(&params.t0, &mut params.g0);
        }
        if let Some(duration) = duration {
            params.duration = *duration as f64;
        }
        // Disable ionospheric correction
        params.ionoutc.enable = !ionospheric_disable;
        if let Some(fixed_gain) = path_loss {
            params.fixed_gain = *fixed_gain;
            params.path_loss_enable = false;
        }
        assert!(
            (1..=128).contains(&params.fixed_gain),
            "ERROR: Fixed gain must be between 1 and 128."
        );
        params.verb = verbose;
        params
    }
}

pub fn sub_vect(y: &mut [f64; 3], x1: &[f64; 3], x2: &[f64; 3]) {
    y[0] = x1[0] - x2[0];
    y[1] = x1[1] - x2[1];
    y[2] = x1[2] - x2[2];
}

pub fn norm_vect(x: &[f64; 3]) -> f64 {
    (x[0] * x[0] + x[1] * x[1] + x[2] * x[2]).sqrt()
}

pub fn dot_prod(x1: &[f64; 3], x2: &[f64; 3]) -> f64 {
    x1[0] * x2[0] + x1[1] * x2[1] + x1[2] * x2[2]
}
/// !generate the C/A code sequence for a given Satellite Vehicle PRN
///  \param[in] prn PRN number of the Satellite Vehicle
///  \param[out] ca Caller-allocated integer array of 1023 bytes
pub fn codegen(ca: &mut [i32; CA_SEQ_LEN], prn: i32) {
    let delay: [usize; 32] = [
        5, 6, 7, 8, 17, 18, 139, 140, 141, 251, 252, 254, 255, 256, 257, 258,
        469, 470, 471, 472, 473, 474, 509, 512, 513, 514, 515, 516, 859, 860,
        861, 862,
    ];
    let mut g1: [i32; CA_SEQ_LEN] = [0; CA_SEQ_LEN];
    let mut g2: [i32; CA_SEQ_LEN] = [0; CA_SEQ_LEN];
    let mut r1: [i32; N_DWRD_SBF] = [0; N_DWRD_SBF];
    let mut r2: [i32; N_DWRD_SBF] = [0; N_DWRD_SBF];
    let mut c1: i32;
    let mut c2: i32;
    if !(1..=32).contains(&prn) {
        return;
    }
    for i in 0..N_DWRD_SBF {
        r2[i] = -1;
        r1[i] = r2[i];
    }
    for i in 0..CA_SEQ_LEN {
        g1[i] = r1[9];
        g2[i] = r2[9];
        c1 = r1[2] * r1[9];
        c2 = r2[1] * r2[2] * r2[5] * r2[7] * r2[8] * r2[9];
        for j in (1..10).rev() {
            r1[j] = r1[j - 1];
            r2[j] = r2[j - 1];
        }
        r1[0] = c1;
        r2[0] = c2;
    }
    let mut j = CA_SEQ_LEN - delay[(prn - 1) as usize];
    for i in 0..CA_SEQ_LEN {
        ca[i] = (1 - g1[i] * g2[j % CA_SEQ_LEN]) / 2;
        j += 1;
    }
}

//  Convert a UTC date into a GPS date
pub fn date2gps(t: &DateTime, g: &mut GpsTime) {
    let doy: [i32; 12] =
        [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let ye = (t).y - 1980;

    // Compute the number of leap days since Jan 5/Jan 6, 1980.
    let mut lpdays = ye / 4 + 1;
    if ye % 4 == 0 && (t).m <= 2 {
        lpdays -= 1;
    }

    // Compute the number of days elapsed since Jan 5/Jan 6, 1980.
    let de = ye * 365 + doy[((t).m - 1) as usize] + (t).d + lpdays - 6;

    // Convert time to GPS weeks and seconds.
    (g).week = de / 7;
    (g).sec = f64::from(de % 7) * SECONDS_IN_DAY
        + f64::from((t).hh) * SECONDS_IN_HOUR
        + f64::from((t).mm) * SECONDS_IN_MINUTE
        + (t).sec;
}

// Convert Julian day number to calendar date
pub fn gps2date(g: &GpsTime, t: &mut DateTime) {
    let c = (f64::from(7 * (g).week)
        + ((g).sec / 86400.0).floor()
        + 2_444_245.0) as i32
        + 1537;
    let d = ((f64::from(c) - 122.1f64) / 365.25f64) as i32;
    let e = 365 * d + d / 4;
    let f = (f64::from(c - e) / 30.6001) as i32;
    (t).d = c - e - (30.6001 * f64::from(f)) as i32;
    (t).m = f - 1 - 12 * (f / 14);
    (t).y = d - 4715 - (7 + (t).m) / 10;
    (t).hh = ((g).sec / 3600.0) as i32 % 24;
    (t).mm = ((g).sec / 60.0) as i32 % 60;
    (t).sec = g.sec - 60.0 * ((g).sec / 60.0).floor();
}
///  Convert Earth-centered Earth-fixed (ECEF) into Lat/Long/Height
///  \param[in] xyz Input Array of X, Y and Z ECEF coordinates
///  \param[out] llh Output Array of Latitude, Longitude and Height
pub fn xyz2llh(xyz_0: &[f64; 3], llh: &mut [f64; 3]) {
    let mut zdz: f64;
    let mut nh: f64;
    let mut slat: f64;
    let mut n: f64;
    let mut dz_new: f64;
    let a = WGS84_RADIUS;
    let e = WGS84_ECCENTRICITY;
    let eps = 1.0e-3;
    let e2 = e * e;
    if norm_vect(xyz_0) < eps {
        // Invalid ECEF vector
        llh[0] = 0.0;
        llh[1] = 0.0;
        llh[2] = -a;
        return;
    }
    let x = xyz_0[0];
    let y = xyz_0[1];
    let z = xyz_0[2];
    let rho2 = x * x + y * y;
    let mut dz = e2 * z;
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
    llh[0] = zdz.atan2(rho2.sqrt());
    llh[1] = y.atan2(x);
    llh[2] = nh - n;
}

/// Convert Lat/Long/Height into Earth-centered Earth-fixed (ECEF)
/// \param[in] llh Input Array of Latitude, Longitude and Height
/// \param[out] xyz Output Array of X, Y and Z ECEF coordinates
pub fn llh2xyz(llh: &[f64; 3], xyz_0: &mut [f64; 3]) {
    let a = WGS84_RADIUS;
    let e = WGS84_ECCENTRICITY;
    let e2 = e * e;
    let clat = (llh[0]).cos();
    let slat = (llh[0]).sin();
    let clon = (llh[1]).cos();
    let slon = (llh[1]).sin();
    let d = e * slat;
    let n = a / (1.0 - d * d).sqrt();
    let nph = n + llh[2];
    let tmp = nph * clat;
    xyz_0[0] = tmp * clon;
    xyz_0[1] = tmp * slon;
    xyz_0[2] = ((1.0 - e2) * n + llh[2]) * slat;
}

///  \brief Compute the intermediate matrix for LLH to ECEF
///  \param[in] llh Input position in Latitude-Longitude-Height format
///  \param[out] t Three-by-Three output matrix
pub fn ltcmat(llh: &[f64; 3], t: &mut [[f64; 3]; 3]) {
    let slat = (llh[0]).sin();
    let clat = (llh[0]).cos();
    let slon = (llh[1]).sin();
    let clon = (llh[1]).cos();
    t[0][0] = -slat * clon;
    t[0][1] = -slat * slon;
    t[0][2] = clat;
    t[1][0] = -slon;
    t[1][1] = clon;
    t[1][2] = 0.0;
    t[2][0] = clat * clon;
    t[2][1] = clat * slon;
    t[2][2] = slat;
}

///  \brief Convert Earth-centered Earth-Fixed to ?
/// \param[in] xyz Input position as vector in ECEF format
/// \param[in] t Intermediate matrix computed by \ref ltcmat
/// \param[out] neu Output position as North-East-Up format
pub fn ecef2neu(xyz_0: &[f64; 3], t: &[[f64; 3]; 3], neu: &mut [f64; 3]) {
    neu[0] = t[0][0] * xyz_0[0] + t[0][1] * xyz_0[1] + t[0][2] * xyz_0[2];
    neu[1] = t[1][0] * xyz_0[0] + t[1][1] * xyz_0[1] + t[1][2] * xyz_0[2];
    neu[2] = t[2][0] * xyz_0[0] + t[2][1] * xyz_0[1] + t[2][2] * xyz_0[2];
}

///  \brief Convert North-East-Up to Azimuth + Elevation
/// \param[in] neu Input position in North-East-Up format
/// \param[out] azel Output array of azimuth + elevation as double
///
pub fn neu2azel(azel: &mut [f64; 2], neu: &[f64; 3]) {
    azel[0] = neu[1].atan2(neu[0]);
    if azel[0] < 0.0 {
        azel[0] += 2.0 * PI;
    }
    let ne = (neu[0] * neu[0] + neu[1] * neu[1]).sqrt();
    azel[1] = neu[2].atan2(ne);
}

/// \brief Compute Satellite position, velocity and clock at given time
///
/// Computing Satellite Velocity using the Broadcast Ephemeris
/// <http://www.ngs.noaa.gov/gps-toolbox/bc_velo.htm>
/// \param[in] eph Ephemeris data of the satellite
/// \param[in] g GPS time at which position is to be computed
/// \param[out] pos Computed position (vector)
/// \param[out] vel Computed velocity (vector)
/// \param[clk] clk Computed clock
///
pub fn satpos(
    eph: &Ephemeris, g: &GpsTime, pos: &mut [f64; 3], vel: &mut [f64; 3],
    clk: &mut [f64; 2],
) {
    let mut tk = g.sec - eph.toe.sec;
    if tk > SECONDS_IN_HALF_WEEK {
        tk -= SECONDS_IN_WEEK;
    } else if tk < -SECONDS_IN_HALF_WEEK {
        tk += SECONDS_IN_WEEK;
    }
    let mk = eph.m0 + eph.n * tk;
    let mut ek = mk;
    let mut ekold = ek + 1.0;
    let mut one_minusecos_e = 0.0; // Suppress the uninitialized warning.
    while (ek - ekold).abs() > 1.0e-14 {
        ekold = ek;
        one_minusecos_e = 1.0 - eph.ecc * (ekold).cos();
        ek += (mk - ekold + eph.ecc * (ekold.sin())) / one_minusecos_e;
    }
    let sek = (ek).sin();
    let cek = (ek).cos();
    let ekdot = eph.n / one_minusecos_e;
    let relativistic = -4.442_807_633E-10 * eph.ecc * eph.sqrta * sek;
    let pk = (eph.sq1e2 * sek).atan2(cek - eph.ecc) + eph.aop;
    let pkdot = eph.sq1e2 * ekdot / one_minusecos_e;
    let s2pk = (2.0 * pk).sin();
    let c2pk = (2.0 * pk).cos();
    let uk = pk + eph.cus * s2pk + eph.cuc * c2pk;
    let suk = (uk).sin();
    let cuk = (uk).cos();
    let ukdot = pkdot * (1.0 + 2.0 * (eph.cus * c2pk - eph.cuc * s2pk));
    let rk = eph.A * one_minusecos_e + eph.crc * c2pk + eph.crs * s2pk;
    let rkdot = eph.A * eph.ecc * sek * ekdot
        + 2.0 * pkdot * (eph.crs * c2pk - eph.crc * s2pk);
    let ik = eph.inc0 + eph.idot * tk + eph.cic * c2pk + eph.cis * s2pk;
    let sik = (ik).sin();
    let cik = (ik).cos();
    let ikdot = eph.idot + 2.0 * pkdot * (eph.cis * c2pk - eph.cic * s2pk);
    let xpk = rk * cuk;
    let ypk = rk * suk;
    let xpkdot = rkdot * cuk - ypk * ukdot;
    let ypkdot = rkdot * suk + xpk * ukdot;
    let ok = eph.omg0 + tk * eph.omgkdot - OMEGA_EARTH * eph.toe.sec;
    let sok = (ok).sin();
    let cok = (ok).cos();
    pos[0] = xpk * cok - ypk * cik * sok;
    pos[1] = xpk * sok + ypk * cik * cok;
    pos[2] = ypk * sik;
    let tmp = ypkdot * cik - ypk * sik * ikdot;
    vel[0] = -eph.omgkdot * pos[1] + xpkdot * cok - tmp * sok;
    vel[1] = eph.omgkdot * pos[0] + xpkdot * sok + tmp * cok;
    vel[2] = ypk * cik * ikdot + ypkdot * sik;
    let mut tk = g.sec - eph.toc.sec;
    if tk > SECONDS_IN_HALF_WEEK {
        tk -= SECONDS_IN_WEEK;
    } else if tk < -SECONDS_IN_HALF_WEEK {
        tk += SECONDS_IN_WEEK;
    }
    clk[0] = eph.af0 + tk * (eph.af1 + tk * eph.af2) + relativistic - eph.tgd;
    clk[1] = eph.af1 + 2.0 * tk * eph.af2;
}

/// \brief Compute Subframe from Ephemeris
/// \param[in] eph Ephemeris of given SV
/// \param[out] sbf Array of five sub-frames, 10 long words each
///
#[allow(clippy::too_many_lines)]
pub fn eph2sbf(
    eph: Ephemeris, ionoutc: &IonoUtc, sbf: &mut [[u32; N_DWRD_SBF]; 5],
) {
    let ura = 0;
    let data_id = 1;
    let sbf4_page25_sv_id = 63;
    let sbf5_page25_sv_id = 51;
    let wnlsf;
    let dtlsf;
    let dn;
    let sbf4_page18_sv_id = 56;

    // FIXED: This has to be the "transmission" week number, not for the ephemeris reference time
    //wn = (unsigned long)(eph.toe.week%1024);
    let wn = 0;
    let toe = (eph.toe.sec / 16.0) as u32;
    let toc = (eph.toc.sec / 16.0) as u32;
    let iode = eph.iode as u32;
    let iodc = eph.iodc as u32;
    let deltan = (eph.deltan / POW2_M43 / PI) as i32;
    let cuc = (eph.cuc / POW2_M29) as i32;
    let cus = (eph.cus / POW2_M29) as i32;
    let cic = (eph.cic / POW2_M29) as i32;
    let cis = (eph.cis / POW2_M29) as i32;
    let crc = (eph.crc / POW2_M5) as i32;
    let crs = (eph.crs / POW2_M5) as i32;
    let ecc = (eph.ecc / POW2_M33) as u32;
    let sqrta = (eph.sqrta / POW2_M19) as u32;
    let m0 = (eph.m0 / POW2_M31 / PI) as i32;
    let omg0 = (eph.omg0 / POW2_M31 / PI) as i32;
    let inc0 = (eph.inc0 / POW2_M31 / PI) as i32;
    let aop = (eph.aop / POW2_M31 / PI) as i32;
    let omgdot = (eph.omgdot / POW2_M43 / PI) as i32;
    let idot = (eph.idot / POW2_M43 / PI) as i32;
    let af0 = (eph.af0 / POW2_M31) as i32;
    let af1 = (eph.af1 / POW2_M43) as i32;
    let af2 = (eph.af2 / POW2_M55) as i32;
    let tgd = (eph.tgd / POW2_M31) as i32;
    let svhlth = eph.svhlth as u32 as i32;

    #[allow(non_snake_case)]
    let codeL2 = eph.codeL2 as u32 as i32;
    let wna = (eph.toe.week % 256) as u32;
    let toa = (eph.toe.sec / 4096.0) as u32;
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
    let wnt = (ionoutc.wnt % 256) as u32;
    // 2016/12/31 (Sat) -> WNlsf = 1929, DN = 7 (http://navigationservices.agi.com/GNSSWeb/)
    // Days are counted from 1 to 7 (Sunday is 1).
    if ionoutc.leapen == 1 {
        wnlsf = (ionoutc.wnlsf % 256) as u32;
        dn = ionoutc.dn as u32;
        dtlsf = ionoutc.dtlsf as u32;
    } else {
        wnlsf = (1929 % 256) as u32;
        dn = 7;
        dtlsf = 18;
    }
    // Subframe 1
    (sbf[0])[0] = 0x8b0000 << 6;
    (sbf[0])[1] = 0x1 << 8;
    (sbf[0])[2] = (wn & 0x3ff) << 20
        | (codeL2 as u32 & 0x3) << 18
        | (ura & 0xf) << 14
        | (svhlth as u32 & 0x3f) << 8
        | (iodc >> 8 & 0x3) << 6;
    (sbf[0])[3] = 0;
    (sbf[0])[4] = 0;
    (sbf[0])[5] = 0;
    (sbf[0])[6] = (tgd as u32 & 0xff) << 6;
    (sbf[0])[7] = (iodc & 0xff) << 22 | (toc & 0xffff) << 6;
    (sbf[0])[8] = (af2 as u32 & 0xff) << 22 | (af1 as u32 & 0xffff) << 6;
    (sbf[0])[9] = (af0 as u32 & 0x3fffff) << 8;
    // Subframe 2
    (sbf[1])[0] = 0x8b0000 << 6;
    (sbf[1])[1] = 0x2 << 8;
    (sbf[1])[2] = (iode & 0xff) << 22 | (crs as u32 & 0xffff) << 6;
    (sbf[1])[3] =
        (deltan as u32 & 0xffff) << 14 | ((m0 >> 24) as u32 & 0xff) << 6;
    (sbf[1])[4] = (m0 as u32 & 0xffffff) << 6;
    (sbf[1])[5] = (cuc as u32 & 0xffff) << 14 | (ecc >> 24 & 0xff) << 6;
    (sbf[1])[6] = (ecc & 0xffffff) << 6;
    (sbf[1])[7] = (cus as u32 & 0xffff) << 14 | (sqrta >> 24 & 0xff) << 6;
    (sbf[1])[8] = (sqrta & 0xffffff) << 6;
    (sbf[1])[9] = (toe & 0xffff) << 14;
    // Subframe 3
    (sbf[2])[0] = 0x8b0000 << 6;
    (sbf[2])[1] = 0x3 << 8;
    (sbf[2])[2] =
        (cic as u32 & 0xffff) << 14 | ((omg0 >> 24) as u32 & 0xff) << 6;
    (sbf[2])[3] = (omg0 as u32 & 0xffffff) << 6;
    (sbf[2])[4] =
        (cis as u32 & 0xffff) << 14 | ((inc0 >> 24) as u32 & 0xff) << 6;
    (sbf[2])[5] = (inc0 as u32 & 0xffffff) << 6;
    (sbf[2])[6] =
        (crc as u32 & 0xffff) << 14 | ((aop >> 24) as u32 & 0xff) << 6;
    (sbf[2])[7] = (aop as u32 & 0xffffff) << 6;
    (sbf[2])[8] = (omgdot as u32 & 0xffffff_u32) << 6;
    (sbf[2])[9] = (iode & 0xff) << 22 | (idot as u32 & 0x3fff) << 8;
    if ionoutc.vflg {
        // Subframe 4, page 18
        (sbf[3])[0] = 0x8b0000 << 6;
        (sbf[3])[1] = 0x4 << 8;
        (sbf[3])[2] = data_id << 28
            | sbf4_page18_sv_id << 22
            | (alpha0 as u32 & 0xff) << 14
            | (alpha1 as u32 & 0xff) << 6;
        (sbf[3])[3] = (alpha2 as u32 & 0xff) << 22
            | (alpha3 as u32 & 0xff) << 14
            | (beta0 as u32 & 0xff) << 6;
        (sbf[3])[4] = (beta1 as u32 & 0xff) << 22
            | (beta2 as u32 & 0xff) << 14
            | (beta3 as u32 & 0xff) << 6;
        (sbf[3])[5] = (A1 as u32 & 0xffffff) << 6;
        (sbf[3])[6] = ((A0 >> 8) as u32 & 0xffffff) << 6;
        (sbf[3])[7] =
            (A0 as u32 & 0xff) << 22 | (tot & 0xff) << 14 | (wnt & 0xff) << 6;
        (sbf[3])[8] = (dtls as u32 & 0xff) << 22
            | (wnlsf & 0xff) << 14
            | (dn & 0xff) << 6;
        (sbf[3])[9] = (dtlsf & 0xff) << 22;
    } else {
        // Subframe 4, page 25
        (sbf[3])[0] = 0x8b0000 << 6;
        (sbf[3])[1] = 0x4 << 8;
        (sbf[3])[2] = data_id << 28 | sbf4_page25_sv_id << 22;
        (sbf[3])[3] = 0;
        (sbf[3])[4] = 0;
        (sbf[3])[5] = 0;
        (sbf[3])[6] = 0;
        (sbf[3])[7] = 0;
        (sbf[3])[8] = 0;
        (sbf[3])[9] = 0;
    }
    // Subframe 5, page 25
    (sbf[4])[0] = 0x8b0000 << 6;
    (sbf[4])[1] = 0x5 << 8;
    (sbf[4])[2] = data_id << 28
        | sbf5_page25_sv_id << 22
        | (toa & 0xff) << 14
        | (wna & 0xff) << 6;
    (sbf[4])[3] = 0;
    (sbf[4])[4] = 0;
    (sbf[4])[5] = 0;
    (sbf[4])[6] = 0;
    (sbf[4])[7] = 0;
    (sbf[4])[8] = 0;
    (sbf[4])[9] = 0;
}

///  \brief Compute the Checksum for one given word of a subframe
///  \param[in] source The input data
///  \param[in] nib Does this word contain non-information-bearing bits?
///  \returns Computed Checksum
#[allow(non_snake_case)]
pub fn compute_checksum(source: u32, nib: i32) -> u32 {
    /*
    Bits 31 to 30 = 2 LSBs of the previous transmitted word, D29* and D30*
    Bits 29 to  6 = Source data bits, d1, d2, ..., d24
    Bits  5 to  0 = Empty parity bits
    */

    /*
    Bits 31 to 30 = 2 LSBs of the previous transmitted word, D29* and D30*
    Bits 29 to  6 = Data bits transmitted by the SV, D1, D2, ..., D24
    Bits  5 to  0 = Computed parity bits, D25, D26, ..., D30
    */

    /*
                      1            2           3
    bit    12 3456 7890 1234 5678 9012 3456 7890
    ---    -------------------------------------
    D25    11 1011 0001 1111 0011 0100 1000 0000
    D26    01 1101 1000 1111 1001 1010 0100 0000
    D27    10 1110 1100 0111 1100 1101 0000 0000
    D28    01 0111 0110 0011 1110 0110 1000 0000
    D29    10 1011 1011 0001 1111 0011 0100 0000
    D30    00 1011 0111 1010 1000 1001 1100 0000
    */
    let bmask: [u32; 6] = [
        0x3b1f3480, 0x1d8f9a40, 0x2ec7cd00, 0x1763e680, 0x2bb1f340, 0xb7a89c0,
    ];
    let mut D: u32;
    let mut d: u32 = source & 0x3fffffc0;
    let D29: u32 = source >> 31 & 0x1;
    let D30: u32 = source >> 30 & 0x1;
    if nib != 0 {
        // Non-information bearing bits for word 2 and 10
        /*
        Solve bits 23 and 24 to preserve parity check
        with zeros in bits 29 and 30.
        */
        if D30
            .wrapping_add((bmask[4] & d).count_ones())
            .wrapping_rem(2)
            != 0
        {
            d ^= 0x1 << 6;
        }
        if D29
            .wrapping_add((bmask[5] & d).count_ones())
            .wrapping_rem(2)
            != 0
        {
            d ^= 0x1 << 7;
        }
    }
    D = d;
    if D30 != 0 {
        D ^= 0x3fffffc0;
    }
    D |= D29
        .wrapping_add((bmask[0] & d).count_ones())
        .wrapping_rem(2)
        << 5;
    D |= D30
        .wrapping_add((bmask[1] & d).count_ones())
        .wrapping_rem(2)
        << 4;
    D |= D29
        .wrapping_add((bmask[2] & d).count_ones())
        .wrapping_rem(2)
        << 3;
    D |= D30
        .wrapping_add((bmask[3] & d).count_ones())
        .wrapping_rem(2)
        << 2;
    D |= D30
        .wrapping_add((bmask[4] & d).count_ones())
        .wrapping_rem(2)
        << 1;
    D |= D29
        .wrapping_add((bmask[5] & d).count_ones())
        .wrapping_rem(2);
    D &= 0x3fffffff;

    //D |= (source & 0xC0000000UL); // Add D29* and D30* from source data bits
    D
}

pub fn sub_gps_time(g1: GpsTime, g0: GpsTime) -> f64 {
    let mut dt = g1.sec - g0.sec;
    dt += f64::from(g1.week - g0.week) * SECONDS_IN_WEEK;
    dt
}

pub fn inc_gps_time(g0: GpsTime, dt: f64) -> GpsTime {
    let mut g1: GpsTime = GpsTime { week: 0, sec: 0. };
    g1.week = g0.week;
    g1.sec = g0.sec + dt;
    g1.sec = (g1.sec * 1000.0).round() / 1000.0; // Avoid rounding error
    while g1.sec >= SECONDS_IN_WEEK {
        g1.sec -= SECONDS_IN_WEEK;
        g1.week += 1;
    }
    while g1.sec < 0.0 {
        g1.sec += SECONDS_IN_WEEK;
        g1.week -= 1;
    }
    g1
}

#[allow(non_snake_case)]
pub fn ionospheric_delay(
    ionoutc: &IonoUtc, g: &GpsTime, llh: &[f64; 3], azel: &[f64; 2],
) -> f64 {
    let iono_delay: f64;
    if !ionoutc.enable {
        // No ionospheric delay
        return 0.0;
    }
    let E = azel[1] / PI;
    let phi_u = llh[0] / PI;
    let lam_u = llh[1] / PI;
    let F = 1.0 + 16.0 * (0.53f64 - E).powf(3.0);
    if ionoutc.vflg {
        let mut PER: f64;

        // Earth's central angle between the user position and the earth projection of
        // ionospheric intersection point (semi-circles)
        let psi = 0.0137 / (E + 0.11) - 0.022;

        // Geodetic latitude of the earth projection of the ionospheric intersection point
        // (semi-circles)
        let phi_i = phi_u + psi * (azel[0]).cos();
        let phi_i = phi_i.clamp(-0.416, 0.416);

        // Geodetic longitude of the earth projection of the ionospheric intersection point
        // (semi-circles)
        let lam_i = lam_u + psi * (azel[0]).sin() / (phi_i * PI).cos();
        // Geomagnetic latitude of the earth projection of the ionospheric intersection
        // point (mean ionospheric height assumed 350 km) (semi-circles)
        let phi_m = phi_i + 0.064 * ((lam_i - 1.617) * PI).cos();
        let phi_m2 = phi_m * phi_m;
        let phi_m3 = phi_m2 * phi_m;
        let mut AMP = ionoutc.alpha0
            + ionoutc.alpha1 * phi_m
            + ionoutc.alpha2 * phi_m2
            + ionoutc.alpha3 * phi_m3;
        if AMP < 0.0 {
            AMP = 0.0;
        }
        PER = ionoutc.beta0
            + ionoutc.beta1 * phi_m
            + ionoutc.beta2 * phi_m2
            + ionoutc.beta3 * phi_m3;
        if PER < 72000.0 {
            PER = 72000.0;
        }
        // Local time (sec)
        let mut t = SECONDS_IN_DAY / 2.0 * lam_i + g.sec;
        while t >= SECONDS_IN_DAY {
            t -= SECONDS_IN_DAY;
        }
        while t < 0.0 {
            t += SECONDS_IN_DAY;
        }
        // Phase (radians)
        let X = 2.0 * PI * (t - 50400.0) / PER;
        if (X).abs() < 1.57 {
            let X2 = X * X;
            let X4 = X2 * X2;
            iono_delay = F
                * (5.0e-9 + AMP * (1.0 - X2 / 2.0 + X4 / 24.0))
                * SPEED_OF_LIGHT;
        } else {
            iono_delay = F * 5.0e-9 * SPEED_OF_LIGHT;
        }
    } else {
        iono_delay = F * 5.0e-9 * SPEED_OF_LIGHT;
    }
    iono_delay
}

///  \brief Compute range between a satellite and the receiver
///  \param[out] rho The computed range
///  \param[in] eph Ephemeris data of the satellite
///  \param[in] g GPS time at time of receiving the signal
///  \param[in] xyz position of the receiver
pub fn compute_range(
    rho: &mut Range, eph: &Ephemeris, ionoutc: &mut IonoUtc, g: &GpsTime,
    xyz_0: &[f64; 3],
) {
    let mut pos: [f64; 3] = [0.; 3];
    let mut vel: [f64; 3] = [0.; 3];
    let mut clk: [f64; 2] = [0.; 2];
    let mut los: [f64; 3] = [0.; 3];
    let mut llh: [f64; 3] = [0.; 3];
    let mut neu: [f64; 3] = [0.; 3];
    let mut tmat: [[f64; 3]; 3] = [[0.; 3]; 3];
    // SV position at time of the pseudorange observation.
    satpos(eph, g, &mut pos, &mut vel, &mut clk);
    // Receiver to satellite vector and light-time.
    sub_vect(&mut los, &pos, xyz_0);
    let tau = norm_vect(&los) / SPEED_OF_LIGHT;
    // Extrapolate the satellite position backwards to the transmission time.
    pos[0] -= vel[0] * tau;
    pos[1] -= vel[1] * tau;
    pos[2] -= vel[2] * tau;
    let xrot = pos[0] + pos[1] * OMEGA_EARTH * tau;
    let yrot = pos[1] - pos[0] * OMEGA_EARTH * tau;
    pos[0] = xrot;
    pos[1] = yrot;
    // New observer to satellite vector and satellite range.
    sub_vect(&mut los, &pos, xyz_0);
    let range = norm_vect(&los);
    (rho).d = range;
    // Pseudorange.
    (rho).range = range - SPEED_OF_LIGHT * clk[0];
    // Relative velocity of SV and receiver.
    let rate = dot_prod(&vel, &los) / range;
    // Pseudorange rate.
    (rho).rate = rate; // - SPEED_OF_LIGHT*clk[1];
                       // Time of application.
    rho.g = *g;
    // Azimuth and elevation angles.
    xyz2llh(xyz_0, &mut llh);
    ltcmat(&llh, &mut tmat);
    ecef2neu(&los, &tmat, &mut neu);
    neu2azel(&mut (rho).azel, &neu);
    // Add ionospheric delay
    (rho).iono_delay = ionospheric_delay(ionoutc, g, &llh, &(rho).azel);
    (rho).range += (rho).iono_delay;
}

///  \brief Compute the code phase for a given channel (satellite)
///  \param chan Channel on which we operate (is updated)
///  \param[in] rho1 Current range, after \a dt has expired
///  \param[in dt delta-t (time difference) in seconds
pub fn compute_code_phase(chan: &mut Channel, rho1: Range, dt: f64) {
    // Pseudorange rate.
    let rhorate = (rho1.range - chan.rho0.range) / dt;
    // Carrier and code frequency.
    chan.f_carr = -rhorate / LAMBDA_L1;
    chan.f_code = CODE_FREQ + chan.f_carr * CARR_TO_CODE;
    // Initial code phase and data bit counters.
    let ms = (sub_gps_time(chan.rho0.g, chan.g0) + 6.0
        - chan.rho0.range / SPEED_OF_LIGHT)
        * 1000.0;
    let mut ims = ms as i32;
    chan.code_phase = (ms - f64::from(ims)) * CA_SEQ_LEN as f64; // in chip
    chan.iword = ims / 600; // 1 word = 30 bits = 600 ms
    ims -= chan.iword * 600;
    chan.ibit = ims / 20; // 1 bit = 20 code = 20 ms
    ims -= chan.ibit * 20;
    chan.icode = ims; // 1 code = 1 ms
    chan.codeCA = chan.ca[chan.code_phase as usize] * 2 - 1;
    chan.dataBit =
        (chan.dwrd[chan.iword as usize] >> (29 - chan.ibit) & 0x1) as i32 * 2
            - 1;
    // Save current pseudorange
    chan.rho0 = rho1;
}

pub fn generate_nav_msg(g: &GpsTime, chan: &mut Channel, init: bool) {
    let mut g0: GpsTime = GpsTime { week: 0, sec: 0. };
    let mut sbfwrd: u32;
    let mut prevwrd: u32 = 0;
    let mut nib: i32;
    g0.week = g.week;
    g0.sec = f64::from(((g.sec + 0.5) as u32).wrapping_div(30)) * 30.0; // Align with the full frame length = 30 sec
    chan.g0 = g0; // Data bit reference time

    let wn = (g0.week % 1024) as u32;
    let mut tow = (g0.sec as u32).wrapping_div(6);
    if init {
        // Initialize subframe 5
        prevwrd = 0;
        for iwrd in 0..N_DWRD_SBF {
            sbfwrd = chan.sbf[4][iwrd];
            // Add TOW-count message into HOW
            if iwrd == 1 {
                sbfwrd |= (tow & 0x1ffff) << 13;
            }
            // Compute checksum
            sbfwrd |= prevwrd << 30 & 0xc0000000; // 2 LSBs of the previous transmitted word
            nib = if iwrd == 1 || iwrd == 9 { 1 } else { 0 }; // Non-information bearing bits for word 2 and 10
            chan.dwrd[iwrd] = compute_checksum(sbfwrd, nib);
            prevwrd = chan.dwrd[iwrd];
        }
    } else {
        // Save subframe 5
        for iwrd in 0..N_DWRD_SBF {
            chan.dwrd[iwrd] = chan.dwrd[N_DWRD_SBF * N_SBF + iwrd];
            prevwrd = chan.dwrd[iwrd];
        }
        /*
        // Sanity check
        if (((chan->dwrd[1])&(0x1FFFFUL<<13)) != ((tow&0x1FFFFUL)<<13))
        {
            fprintf(stderr, "\nWARNING: Invalid TOW in subframe 5.\n");
            return(0);
        }
        */
    }
    for isbf in 0..N_SBF {
        tow = tow.wrapping_add(1);

        for iwrd in 0..N_DWRD_SBF {
            sbfwrd = chan.sbf[isbf][iwrd];
            // Add transmission week number to Subframe 1
            if isbf == 0 && iwrd == 2 {
                sbfwrd |= (wn & 0x3ff) << 20;
            }
            // Add TOW-count message into HOW
            if iwrd == 1 {
                sbfwrd |= (tow & 0x1ffff) << 13;
            }
            // Compute checksum
            sbfwrd |= prevwrd << 30 & 0xc0000000; // 2 LSBs of the previous transmitted word
            nib = if iwrd == 1 || iwrd == 9 { 1 } else { 0 }; // Non-information bearing bits for word 2 and 10
            chan.dwrd[(isbf + 1) * N_DWRD_SBF + iwrd] =
                compute_checksum(sbfwrd, nib);
            prevwrd = chan.dwrd[(isbf + 1) * N_DWRD_SBF + iwrd];
        }
    }
}

pub fn check_sat_visibility(
    eph: Ephemeris, g: &GpsTime, xyz_0: &[f64; 3], elv_mask: f64,
    azel: &mut [f64; 2],
) -> i32 {
    let mut llh: [f64; 3] = [0.; 3];
    let mut neu: [f64; 3] = [0.; 3];
    let mut pos: [f64; 3] = [0.; 3];
    let mut vel: [f64; 3] = [0.; 3];
    // modified from [f64;3] to [f64;2]
    let mut clk: [f64; 2] = [0.; 2];
    let mut los: [f64; 3] = [0.; 3];
    let mut tmat: [[f64; 3]; 3] = [[0.; 3]; 3];
    if !eph.vflg {
        return -1; // Invalid
    }
    xyz2llh(xyz_0, &mut llh);
    ltcmat(&llh, &mut tmat);
    satpos(&eph, g, &mut pos, &mut vel, &mut clk);
    sub_vect(&mut los, &pos, xyz_0);
    ecef2neu(&los, &tmat, &mut neu);
    neu2azel(azel, &neu);
    if azel[1] * R2D > elv_mask {
        return 1; // Visible
    }
    0 // Invisible
}

pub fn allocate_channel(
    chan: &mut [Channel; 16], eph: &mut [Ephemeris; 32], ionoutc: &mut IonoUtc,
    grx: &GpsTime, xyz_0: &[f64; 3], _elv_mask: f64,
    allocated_sat: &mut [i32; MAX_SAT],
) -> i32 {
    let mut nsat: i32 = 0;
    let mut azel: [f64; 2] = [0.; 2];
    let mut rho: Range = Range {
        g: GpsTime { week: 0, sec: 0. },
        range: 0.,
        rate: 0.,
        d: 0.,
        azel: [0.; 2],
        iono_delay: 0.,
    };
    let ref_0: [f64; 3] = [0., 0., 0.];
    // #[allow(unused_variables)]
    // let mut r_ref: f64 = 0.;
    // #[allow(unused_variables)]
    // let mut r_xyz: f64;
    let mut phase_ini: f64;
    for sv in 0..MAX_SAT {
        if check_sat_visibility(eph[sv], grx, xyz_0, 0.0, &mut azel) == 1 {
            nsat += 1; // Number of visible satellites
            if allocated_sat[sv] == -1 {
                // Visible but not allocated
                //
                // Allocated new satellite
                let mut i = 0;
                while i < MAX_CHAN {
                    if chan[i].prn == 0 {
                        // Initialize channel
                        chan[i].prn = sv as i32 + 1;
                        chan[i].azel[0] = azel[0];
                        chan[i].azel[1] = azel[1];
                        // C/A code generation
                        codegen(&mut chan[i].ca, (chan[i]).prn);
                        // Generate subframe
                        eph2sbf(eph[sv], ionoutc, &mut chan[i].sbf);
                        // Generate navigation message
                        generate_nav_msg(grx, &mut chan[i], true);
                        // Initialize pseudorange
                        compute_range(&mut rho, &eph[sv], ionoutc, grx, xyz_0);
                        (chan[i]).rho0 = rho;
                        // Initialize carrier phase
                        // r_xyz = rho.range;
                        compute_range(&mut rho, &eph[sv], ionoutc, grx, &ref_0);
                        // r_ref = rho.range;
                        phase_ini = 0.0; // TODO: Must initialize properly
                                         //phase_ini = (2.0*r_ref - r_xyz)/LAMBDA_L1;
                                         // #ifdef FLOAT_CARR_PHASE
                                         //                         chan[i].carr_phase = phase_ini - floor(phase_ini);
                                         // #else
                        phase_ini -= (phase_ini).floor();
                        (chan[i]).carr_phase =
                            (512.0 * 65536.0 * phase_ini) as u32;
                        break;
                    }
                    i += 1;
                }
                // Set satellite allocation channel
                if i < MAX_CHAN {
                    allocated_sat[sv] = i as i32;
                }
            }
        } else if allocated_sat[sv] >= 0 {
            // Not visible but allocated
            // Clear channel
            (chan[allocated_sat[sv] as usize]).prn = 0;
            // Clear satellite allocation flag
            allocated_sat[sv] = -1;
        }
    }
    nsat
}

#[allow(clippy::too_many_lines)]
pub fn process(params: Params) -> i32 {
    let mut allocated_sat: [i32; MAX_SAT] = [0; 32];

    let mut fp_out: Option<std::fs::File>;
    let mut eph: [[Ephemeris; MAX_SAT]; EPHEM_ARRAY_SIZE] =
        [[Ephemeris::default(); MAX_SAT]; EPHEM_ARRAY_SIZE];
    let mut chan: [Channel; 16] = [Channel {
        prn: 0,
        ca: [0; CA_SEQ_LEN],
        f_carr: 0.,
        f_code: 0.,
        carr_phase: 0,
        carr_phasestep: 0,
        code_phase: 0.,
        g0: GpsTime::default(),
        sbf: [[0; 10]; 5],
        dwrd: [0; 60],
        iword: 0,
        ibit: 0,
        icode: 0,
        dataBit: 0,
        codeCA: 0,
        azel: [0.; 2],
        rho0: Range::default(),
    }; 16];
    let elvmask: f64 = 0.0;

    // Default options
    // let mut umfile: [libc::c_char; 100] = [0; 100];
    // let mut navfile: [libc::c_char; 100] = [0; 100];
    // let mut outfile: [libc::c_char; 100] = [0; 100];
    let mut gain: [i32; 16] = [0; 16];
    let mut ant_pat: [f64; 37] = [0.; 37];
    let mut tmin = DateTime::default();
    let mut tmax = DateTime::default();
    let mut gmin = GpsTime::default();
    let mut gmax = GpsTime::default();
    let navfile = params.navfile;
    let umfile = params.umfile;
    let nmea_gga = params.nmea_gga;
    let um_llh = params.um_llh;
    let mut static_location_mode = params.static_location_mode;
    let mut xyz = params.xyz;
    let mut llh = params.llh;
    let outfile = params.outfile;
    let samp_freq = params.samp_freq;
    let data_format = params.data_format;
    let mut ionoutc = params.ionoutc;
    let timeoverwrite = params.timeoverwrite;
    let mut t0 = params.t0;
    let mut g0 = params.g0;
    let duration = params.duration;
    let fixed_gain = params.fixed_gain;
    let path_loss_enable = params.path_loss_enable;
    let verb = params.verb;

    if umfile.is_none() && !static_location_mode {
        // Default static location; Tokyo
        static_location_mode = true;
        llh[0] = 35.681298 / R2D;
        llh[1] = 139.766247 / R2D;
        llh[2] = 10.0f64;
    }
    if duration < 0.0f64
        || duration > USER_MOTION_SIZE as f64 / 10.0 && !static_location_mode
        || duration > STATIC_MAX_DURATION as f64 && static_location_mode
    {
        eprintln!("ERROR: Invalid duration.");
        panic!();
    }
    let iduration = (duration * 10.0 + 0.5) as usize;
    let mut samp_freq = (samp_freq / 10.0).floor();
    let iq_buff_size = samp_freq as usize; // samples per 0.1sec
    samp_freq *= 10.0;
    // let delt = 1.0f64 / samp_freq;
    let delt = samp_freq.recip();

    ////////////////////////////////////////////////////////////
    // Receiver position
    ////////////////////////////////////////////////////////////
    let numd = if static_location_mode {
        // Static geodetic coordinates input mode: "-l"
        // Added by scateu@gmail.com
        eprintln!("Using static location mode.");
        // Set user initial position
        llh2xyz(&llh, &mut xyz[0]);
        // Set simulation duration
        iduration
    } else {
        let umfilex = umfile.clone().unwrap();
        let numd = if nmea_gga {
            read_nmea_gga(&mut xyz, &umfilex)
            // numd = readNmeaGGA(&mut xyz, umfile);
        } else if um_llh {
            read_user_motion_llh(&mut xyz, &umfilex)
            // numd = unsafe { readUserMotionLLH(&mut xyz, umfile) };
        } else {
            read_user_motion(&mut xyz, &umfilex)
            // numd = unsafe { readUserMotion(&mut xyz, umfile) };
        };
        let Ok(mut numd) = numd else {
            panic!("ERROR: Failed to open user motion / NMEA GGA file.");
        };
        assert_ne!(
            numd, 0,
            "ERROR: Failed to read user motion / NMEA GGA data."
        );
        // Set simulation duration
        if numd > iduration {
            numd = iduration;
        }
        // Set user initial position
        xyz2llh(&xyz[0], &mut llh);
        numd
    };

    eprintln!("xyz = {}, {}, {}", xyz[0][0], xyz[0][1], xyz[0][2],);

    eprintln!("llh = {}, {}, {}", llh[0] * R2D, llh[1] * R2D, llh[2],);

    ////////////////////////////////////////////////////////////
    // Read ephemeris
    ////////////////////////////////////////////////////////////
    // let navfile = navfile.to_str().unwrap_or("");
    // let c_string = CString::new(navfile).unwrap();
    // let navff = c_string.into_raw();
    // let neph = readRinexNavAll(&mut eph, &mut ionoutc, navff);
    let Ok(neph) = read_rinex_nav_all(&mut eph, &mut ionoutc, &navfile) else {
        panic!("ERROR: ephemeris file not found or error.");
    };
    assert_ne!(neph, 0, "ERROR: No ephemeris available.");
    if verb && ionoutc.vflg {
        eprintln!(
            "  {:12.3e} {:12.3e} {:12.3e} {:12.3e}",
            ionoutc.alpha0, ionoutc.alpha1, ionoutc.alpha2, ionoutc.alpha3,
        );

        eprintln!(
            "  {:12.3e} {:12.3e} {:12.3e} {:12.3e}",
            ionoutc.beta0, ionoutc.beta1, ionoutc.beta2, ionoutc.beta3,
        );

        eprintln!(
            "   {:19.11e} {:19.11e}  {:9} {:9}",
            ionoutc.A0, ionoutc.A1, ionoutc.tot, ionoutc.wnt,
        );

        eprintln!("{:6}", ionoutc.dtls,);
    }
    for sv in 0..MAX_SAT {
        if eph[0][sv].vflg {
            gmin = eph[0][sv].toc;
            tmin = eph[0][sv].t;
            break;
        }
    }
    // gmax.sec = 0.;
    // gmax.week = 0;
    // tmax.sec = 0.;
    // tmax.mm = 0;
    // tmax.hh = 0;
    // tmax.d = 0;
    // tmax.m = 0;
    // tmax.y = 0;

    for sv in 0..MAX_SAT {
        if eph[neph - 1][sv].vflg {
            gmax = eph[neph - 1][sv].toc;
            tmax = eph[neph - 1][sv].t;
            break;
        }
    }
    if g0.week >= 0 {
        // Scenario start time has been set.
        if timeoverwrite {
            let mut gtmp: GpsTime = GpsTime::default();
            let mut ttmp: DateTime = DateTime::default();
            gtmp.week = g0.week;
            gtmp.sec = f64::from(g0.sec as i32 / 7200) * 7200.0;
            // Overwrite the UTC reference week number
            let dsec = sub_gps_time(gtmp, gmin);
            ionoutc.wnt = gtmp.week;
            ionoutc.tot = gtmp.sec as i32;
            // Iono/UTC parameters may no longer valid
            //ionoutc.vflg = FALSE;
            for sv in 0..MAX_SAT {
                for i_eph in eph.iter_mut().take(neph) {
                    if i_eph[sv].vflg {
                        gtmp = inc_gps_time(i_eph[sv].toc, dsec);
                        gps2date(&gtmp, &mut ttmp);
                        i_eph[sv].toc = gtmp;
                        i_eph[sv].t = ttmp;
                        gtmp = inc_gps_time(i_eph[sv].toe, dsec);
                        i_eph[sv].toe = gtmp;
                    }
                }
            }
        } else if sub_gps_time(g0, gmin) < 0.0
            || sub_gps_time(gmax, g0) < 0.0f64
        {
            eprintln!("ERROR: Invalid start time.");
            eprintln!(
                "tmin = {:4}/{:02}/{:02},{:02}:{:02}:{:0>2.0} ({}:{:.0})",
                tmin.y,
                tmin.m,
                tmin.d,
                tmin.hh,
                tmin.mm,
                tmin.sec,
                gmin.week,
                gmin.sec,
            );
            eprintln!(
                "tmax = {:4}/{:02}/{:02},{:02}:{:02}:{:0>2.0} ({}:{:.0})",
                tmax.y,
                tmax.m,
                tmax.d,
                tmax.hh,
                tmax.mm,
                tmax.sec,
                gmax.week,
                gmax.sec,
            );
            panic!();
        }
    } else {
        g0 = gmin;
        t0 = tmin;
    }

    eprintln!(
        "Start time = {:4}/{:02}/{:02},{:02}:{:02}:{:0>2.0} ({}:{:.0})",
        t0.y, t0.m, t0.d, t0.hh, t0.mm, t0.sec, g0.week, g0.sec,
    );

    eprintln!("Duration = {:.1} [sec]", numd as f64 / 10.0);

    // Select the current set of ephemerides
    let mut ieph = usize::MAX;
    for (i, eph_item) in eph.iter().enumerate().take(neph) {
        for e in eph_item.iter().take(MAX_SAT) {
            if e.vflg {
                let dt = sub_gps_time(g0, e.toc);
                if (-SECONDS_IN_HOUR..SECONDS_IN_HOUR).contains(&dt) {
                    ieph = i;
                    break;
                }
            }
        }
        if ieph != usize::MAX {
            // ieph has been set
            break;
        }
        // if ieph >= 0 {
        //     break;
        // }
    }

    if ieph == usize::MAX {
        eprintln!("ERROR: No current set of ephemerides has been found.",);
        panic!();
    }

    ////////////////////////////////////////////////////////////
    // Baseband signal buffer and output file
    ////////////////////////////////////////////////////////////

    // Allocate I/Q buffer
    let mut iq_buff: Vec<i16> = vec![0; 2 * iq_buff_size];
    let mut iq8_buff: Vec<i8> = vec![0; 2 * iq_buff_size];
    if data_format == SC08 {
        iq8_buff = vec![0; 2 * iq_buff_size];
    } else if data_format == SC01 {
        iq8_buff = vec![0; iq_buff_size / 4]; // byte = {I0, Q0, I1, Q1, I2, Q2, I3, Q3}
    }

    // Open output file
    // "-" can be used as name for stdout
    // if strcmp(
    //     b"-\0" as *const u8 as *const libc::c_char,
    //     outfile.as_mut_ptr(),
    // ) != 0
    // {
    //     fp = fopen(
    //         outfile.as_mut_ptr(),
    //         b"wb\0" as *const u8 as *const libc::c_char,
    //     );
    //     if fp.is_null() {
    //         eprintln!("ERROR: Failed to open output file.");
    //         panic!();
    //     }
    // } else {
    //     // todo: temporarily disable
    //     // fp = stdout;
    // }
    // let out_file = String::from_utf8(outfile.iter().map(|&c| c as u8).collect());
    // if let Ok(out_file) = out_file {
    //     if out_file != "-" {
    //         let file_name = out_file.trim_end_matches("\0");
    fp_out = std::fs::File::create(outfile).ok();
    //     } else {
    //         // use stdout
    //         unimplemented!()
    //     }
    // }

    ////////////////////////////////////////////////////////////
    // Initialize channels
    ////////////////////////////////////////////////////////////

    // Clear all channels
    chan.iter_mut().take(MAX_CHAN).for_each(|ch| ch.prn = 0);
    // Clear satellite allocation flag
    allocated_sat.iter_mut().take(MAX_SAT).for_each(|s| *s = -1);
    // Initial reception time
    let mut grx = inc_gps_time(g0, 0.0);
    // Allocate visible satellites
    allocate_channel(
        &mut chan,
        &mut eph[ieph],
        &mut ionoutc,
        &grx,
        &xyz[0],
        elvmask,
        &mut allocated_sat,
    );
    // for i in 0..MAX_CHAN {
    for ichan in chan.iter().take(MAX_CHAN) {
        if ichan.prn > 0 {
            eprintln!(
                "{:02} {:6.1} {:5.1} {:11.1} {:5.1}",
                ichan.prn,
                ichan.azel[0] * R2D,
                ichan.azel[1] * R2D,
                ichan.rho0.d,
                ichan.rho0.iono_delay,
            );
        }
    }

    ////////////////////////////////////////////////////////////
    // Receiver antenna gain pattern
    ////////////////////////////////////////////////////////////
    for i in 0..37 {
        ant_pat[i] = 10.0f64.powf(-ANT_PAT_DB[i] / 20.0);
    }

    ////////////////////////////////////////////////////////////
    // Generate baseband signals
    ////////////////////////////////////////////////////////////
    let time_start = Instant::now();
    grx = inc_gps_time(grx, 0.1);
    for iumd in 1..numd {
        for i in 0..MAX_CHAN {
            if chan[i].prn > 0 {
                // Refresh code phase and data bit counters
                let mut rho: Range = Range {
                    g: GpsTime::default(),
                    range: 0.,
                    rate: 0.,
                    d: 0.,
                    azel: [0.; 2],
                    iono_delay: 0.,
                };
                let sv = (chan[i].prn - 1) as usize;
                // Current pseudorange
                if static_location_mode {
                    compute_range(
                        &mut rho,
                        &eph[ieph][sv],
                        &mut ionoutc,
                        &grx,
                        &xyz[0],
                    );
                } else {
                    compute_range(
                        &mut rho,
                        &eph[ieph][sv],
                        &mut ionoutc,
                        &grx,
                        &xyz[iumd],
                    );
                }
                // Update code phase and data bit counters
                chan[i].azel[0] = rho.azel[0];
                chan[i].azel[1] = rho.azel[1];
                compute_code_phase(&mut chan[i], rho, 0.1);
                chan[i].carr_phasestep =
                    (512.0 * 65536.0 * chan[i].f_carr * delt).round() as i32;

                // Path loss
                let path_loss = 20200000.0 / rho.d;
                // Receiver antenna gain
                let ibs = ((90.0 - rho.azel[1] * R2D) / 5.0) as usize; // covert elevation to boresight
                let ant_gain = ant_pat[ibs];
                // Signal gain
                if path_loss_enable {
                    gain[i] = (path_loss * ant_gain * 128.0) as i32; // scaled by 2^7
                } else {
                    gain[i] = fixed_gain; // hold the power level constant
                }
            }
        }
        for isamp in 0..iq_buff_size {
            let mut i_acc: i32 = 0;
            let mut q_acc: i32 = 0;
            for i in 0..16 {
                if chan[i].prn > 0 {
                    // #ifdef FLOAT_CARR_PHASE
                    //                     iTable = (int)floor(chan[i].carr_phase*512.0);
                    // #else
                    let i_table = (chan[i].carr_phase >> 16 & 0x1ff) as usize; // 9-bit index
                    let ip = chan[i].dataBit
                        * chan[i].codeCA
                        * COS_TABLE512[i_table]
                        * gain[i];
                    let qp = chan[i].dataBit
                        * chan[i].codeCA
                        * SIN_TABLE512[i_table]
                        * gain[i];
                    // Accumulate for all visible satellites
                    i_acc += ip;
                    q_acc += qp;
                    // Update code phase
                    chan[i].code_phase += chan[i].f_code * delt;
                    if chan[i].code_phase >= CA_SEQ_LEN as f64 {
                        chan[i].code_phase -= CA_SEQ_LEN as f64;
                        chan[i].icode += 1;
                        if chan[i].icode >= 20 {
                            // 20 C/A codes = 1 navigation data bit
                            chan[i].icode = 0;
                            chan[i].ibit += 1;
                            if chan[i].ibit >= 30 {
                                // 30 navigation data bits = 1 word
                                chan[i].ibit = 0;
                                chan[i].iword += 1;

                                /*
                                if (chan[i].iword>=N_DWRD)
                                    fprintf(stderr, "\nWARNING: Subframe word buffer overflow.\n");
                                */
                            }
                            // Set new navigation data bit
                            chan[i].dataBit = (chan[i].dwrd
                                [chan[i].iword as usize]
                                >> (29 - chan[i].ibit)
                                & 0x1)
                                as i32
                                * 2
                                - 1;
                        }
                    }
                    // Set current code chip
                    chan[i].codeCA =
                        chan[i].ca[chan[i].code_phase as i32 as usize] * 2_i32
                            - 1_i32;
                    // Update carrier phase
                    // #ifdef FLOAT_CARR_PHASE
                    //                     chan[i].carr_phase += chan[i].f_carr * delt;
                    //
                    //                     if (chan[i].carr_phase >= 1.0)
                    //                         chan[i].carr_phase -= 1.0;
                    //                     else if (chan[i].carr_phase<0.0)
                    //                         chan[i].carr_phase += 1.0;
                    // #else
                    chan[i].carr_phase = (chan[i].carr_phase)
                        .wrapping_add(chan[i].carr_phasestep as u32);
                }
            }
            // Scaled by 2^7
            i_acc = (i_acc + 64) >> 7;
            q_acc = (q_acc + 64) >> 7;
            // Store I/Q samples into buffer
            iq_buff[isamp * 2] = i_acc as i16;
            iq_buff[isamp * 2 + 1] = q_acc as i16;
        }
        if data_format == SC01 {
            for isamp in 0..2 * iq_buff_size {
                if isamp % 8 == 0 {
                    iq8_buff[isamp / 8] = 0;
                }
                let fresh1_new = &mut iq8_buff[isamp / 8];

                *fresh1_new = (i32::from(*fresh1_new)
                    | (if i32::from(iq_buff[isamp]) > 0 {
                        0x1
                    } else {
                        0
                    }) << (7 - isamp as i32 % 8))
                    as libc::c_schar;
            }

            if let Some(file) = &mut fp_out {
                unsafe {
                    file.write_all(std::slice::from_raw_parts(
                        iq8_buff.as_ptr().cast::<u8>(),
                        iq_buff_size / 4,
                    ))
                    .ok();
                }
            }
        } else if data_format == SC08 {
            for isamp in 0..2 * iq_buff_size {
                iq8_buff[isamp] =
                    (i32::from(iq_buff[isamp]) >> 4) as libc::c_schar;
                // 12-bit bladeRF -> 8-bit HackRF
                //iq8_buff[isamp] = iq_buff[isamp] >> 8; // for PocketSDR
            }

            if let Some(file) = &mut fp_out {
                unsafe {
                    file.write_all(std::slice::from_raw_parts(
                        iq8_buff.as_ptr().cast::<u8>(),
                        2 * iq_buff_size,
                    ))
                    .ok();
                }
            }
        } else if let Some(file) = &mut fp_out {
            // data_format==SC16
            let byte_slice = unsafe {
                std::slice::from_raw_parts(
                    iq_buff.as_ptr().cast::<u8>(),
                    2 * iq_buff_size * 2, // 2 bytes per sample
                )
            };
            file.write_all(byte_slice).ok();
        }
        //
        // Update navigation message and channel allocation every 30 seconds
        //
        let igrx = (grx.sec * 10.0 + 0.5) as i32;
        if igrx % 300 == 0 {
            // Every 30 seconds
            // for i in 0..MAX_CHAN {
            for ichan in chan.iter_mut().take(MAX_CHAN) {
                if ichan.prn > 0 {
                    generate_nav_msg(&grx, ichan, false);
                }
            }
            // Refresh ephemeris and subframes
            // Quick and dirty fix. Need more elegant way.
            for sv in 0..MAX_SAT {
                if eph[ieph + 1][sv].vflg {
                    let dt = sub_gps_time(eph[ieph + 1][sv].toc, grx);
                    if dt < SECONDS_IN_HOUR {
                        ieph += 1;
                        // for i in 0..MAX_CHAN {
                        for ichan in chan.iter_mut().take(MAX_CHAN) {
                            // Generate new subframes if allocated
                            if ichan.prn != 0_i32 {
                                eph2sbf(
                                    eph[ieph][(ichan.prn - 1) as usize],
                                    &ionoutc,
                                    &mut ichan.sbf,
                                );
                            }
                        }
                    }
                    break;
                }
            }
            // Update channel allocation
            if static_location_mode {
                allocate_channel(
                    &mut chan,
                    &mut eph[ieph],
                    &mut ionoutc,
                    &grx,
                    &xyz[0],
                    elvmask,
                    &mut allocated_sat,
                );
            } else {
                allocate_channel(
                    &mut chan,
                    &mut eph[ieph],
                    &mut ionoutc,
                    &grx,
                    &xyz[iumd],
                    elvmask,
                    &mut allocated_sat,
                );
            }
            // Show details about simulated channels
            if verb {
                eprintln!();
                // for i in 0..MAX_CHAN {
                for ichan in chan.iter().take(MAX_CHAN) {
                    if ichan.prn > 0 {
                        eprintln!(
                            "{:02} {:6.1} {:5.1} {:11.1} {:5.1}",
                            ichan.prn,
                            ichan.azel[0] * R2D,
                            ichan.azel[1] * R2D,
                            ichan.rho0.d,
                            ichan.rho0.iono_delay,
                        );
                    }
                }
            }
        }
        // Update receiver time
        grx = inc_gps_time(grx, 0.1);

        // Update time counter
        eprint!("\rTime into run = {:4.1}\0", sub_gps_time(grx, g0));
        // todo: temporarily disable
        // fflush(stdout);
        // iumd += 1;
    }

    eprintln!("\nDone!");
    eprintln!(
        "Process time = {:.1} [sec]",
        time_start.elapsed().as_secs_f32()
    );
    0_i32
}
