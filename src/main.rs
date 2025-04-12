#![allow(
    dead_code,
    mutable_transmutes,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    clippy::missing_safety_doc
)]
#![feature(extern_types)]
unsafe extern "C" {
    pub type _IO_wide_data;
    pub type _IO_codecvt;
    pub type _IO_marker;

    fn fclose(__stream: *mut FILE) -> i32;
    fn fopen(_: *const libc::c_char, _: *const libc::c_char) -> *mut FILE;

    fn sscanf(_: *const libc::c_char, _: *const libc::c_char, _: ...) -> i32;
    fn fgets(__s: *mut libc::c_char, __n: i32, __stream: *mut FILE) -> *mut libc::c_char;
    fn atof(__nptr: *const libc::c_char) -> f64;
    fn atoi(__nptr: *const libc::c_char) -> i32;
    fn strcpy(_: *mut libc::c_char, _: *const libc::c_char) -> *mut libc::c_char;
    fn strncpy(_: *mut libc::c_char, _: *const libc::c_char, _: u32) -> *mut libc::c_char;
    fn strcmp(_: *const libc::c_char, _: *const libc::c_char) -> i32;
    fn strncmp(_: *const libc::c_char, _: *const libc::c_char, _: u32) -> i32;
    fn strchr(_: *const libc::c_char, _: i32) -> *mut libc::c_char;
    fn strtok(_: *mut libc::c_char, _: *const libc::c_char) -> *mut libc::c_char;
    fn time(__timer: *mut time_t) -> time_t;
    fn gmtime(__timer: *const time_t) -> *mut tm;
}

mod constants;
mod datetime;
mod eph;
mod getopt;
mod ionoutc;
mod read_nmea_gga;
mod read_rinex;
mod read_user_motion;
mod table;
mod utils;

use std::io::Write;

use constants::{PI, USER_MOTION_SIZE};
use datetime::{datetime_t, gpstime_t, tm};
use eph::ephem_t;
use getopt::{loop_through_opts, usage};
use ionoutc::ionoutc_t;
use read_nmea_gga::readNmeaGGA;
use read_rinex::readRinexNavAll;
use read_user_motion::{readUserMotion, readUserMotionLLH};
use std::time::Instant;
use table::{ant_pat_db, cosTable512, sinTable512};
use utils::*;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: i32,
    pub _IO_read_ptr: *mut libc::c_char,
    pub _IO_read_end: *mut libc::c_char,
    pub _IO_read_base: *mut libc::c_char,
    pub _IO_write_base: *mut libc::c_char,
    pub _IO_write_ptr: *mut libc::c_char,
    pub _IO_write_end: *mut libc::c_char,
    pub _IO_buf_base: *mut libc::c_char,
    pub _IO_buf_end: *mut libc::c_char,
    pub _IO_save_base: *mut libc::c_char,
    pub _IO_backup_base: *mut libc::c_char,
    pub _IO_save_end: *mut libc::c_char,
    pub _markers: *mut _IO_marker,
    pub _chain: *mut _IO_FILE,
    pub _fileno: i32,
    pub _flags2: i32,
    pub _old_offset: libc::c_long,
    pub _cur_column: libc::c_ushort,
    pub _vtable_offset: libc::c_schar,
    pub _shortbuf: [libc::c_char; 1],
    pub _lock: *mut libc::c_void,
    pub _offset: libc::c_long,
    pub _codecvt: *mut _IO_codecvt,
    pub _wide_data: *mut _IO_wide_data,
    pub _freeres_list: *mut _IO_FILE,
    pub _freeres_buf: *mut libc::c_void,
    pub __pad5: u32,
    pub _mode: i32,
    pub _unused2: [libc::c_char; 20],
}
pub type _IO_lock_t = ();
pub type FILE = _IO_FILE;
type clock_t = libc::c_long;
type time_t = libc::c_long;

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct range_t {
    pub g: gpstime_t,
    pub range: f64,
    pub rate: f64,
    pub d: f64,
    pub azel: [f64; 2],
    pub iono_delay: f64,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct channel_t {
    pub prn: i32,
    pub ca: [i32; 1023],
    pub f_carr: f64,
    pub f_code: f64,
    pub carr_phase: u32,
    pub carr_phasestep: i32,
    pub code_phase: f64,
    pub g0: gpstime_t,
    pub sbf: [[u32; 10]; 5],
    pub dwrd: [u32; 60],
    pub iword: i32,
    pub ibit: i32,
    pub icode: i32,
    pub dataBit: i32,
    pub codeCA: i32,
    pub azel: [f64; 2],
    pub rho0: range_t,
}

pub fn subVect(y: &mut [f64; 3], x1: &[f64; 3], x2: &[f64; 3]) {
    y[0] = x1[0] - x2[0];
    y[1] = x1[1] - x2[1];
    y[2] = x1[2] - x2[2];
}

pub fn normVect(x: &[f64; 3]) -> f64 {
    sqrt(x[0] * x[0] + x[1] * x[1] + x[2] * x[2])
}

pub fn dotProd(x1: &[f64; 3], x2: &[f64; 3]) -> f64 {
    x1[0] * x2[0] + x1[1] * x2[1] + x1[2] * x2[2]
}

pub fn codegen(ca: &mut [i32; 1023], prn: i32) {
    let delay: [usize; 32] = [
        5, 6, 7, 8, 17, 18, 139, 140, 141, 251, 252, 254, 255, 256, 257, 258, 469, 470, 471, 472,
        473, 474, 509, 512, 513, 514, 515, 516, 859, 860, 861, 862,
    ];
    let mut g1: [i32; 1023] = [0; 1023];
    let mut g2: [i32; 1023] = [0; 1023];
    let mut r1: [i32; 10] = [0; 10];
    let mut r2: [i32; 10] = [0; 10];
    let mut c1: i32;
    let mut c2: i32;
    // let mut i: i32 = 0;
    // let mut j = 0;
    if !(1..=32).contains(&prn) {
        return;
    }
    let mut i = 0;
    while i < 10 {
        r2[i] = -1_i32;
        r1[i] = r2[i];
        i += 1;
    }
    let mut i = 0;
    while i < 1023 {
        g1[i] = r1[9];
        g2[i] = r2[9];
        c1 = r1[2] * r1[9];
        c2 = r2[1] * r2[2] * r2[5] * r2[7] * r2[8] * r2[9];
        let mut j = 9;
        while j > 0 {
            r1[j] = r1[j - 1];
            r2[j] = r2[j - 1];
            j -= 1;
        }
        r1[0] = c1;
        r2[0] = c2;
        i += 1;
    }
    let mut i = 0;
    let mut j = 1023 - delay[(prn - 1) as usize];
    while i < 1023 {
        ca[i] = (1_i32 - g1[i] * g2[j % 1023]) / 2_i32;
        i += 1;
        j += 1;
    }
}

pub fn date2gps(t: &datetime_t, g: &mut gpstime_t) {
    let doy: [i32; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let ye = (t).y - 1980_i32;
    let mut lpdays = ye / 4_i32 + 1_i32;
    if ye % 4_i32 == 0_i32 && (t).m <= 2_i32 {
        lpdays -= 1;
    }
    let de = ye * 365_i32 + doy[((t).m - 1_i32) as usize] + (t).d + lpdays - 6_i32;
    (g).week = de / 7_i32;
    (g).sec = (de % 7_i32) as f64 * 86400.0f64
        + (t).hh as f64 * 3600.0f64
        + (t).mm as f64 * 60.0f64
        + (t).sec;
}

pub fn gps2date(g: &gpstime_t, t: &mut datetime_t) {
    let c: i32 =
        ((7_i32 * (g).week) as f64 + floor((g).sec / 86400.0f64) + 2444245.0f64) as i32 + 1537_i32;
    let d: i32 = ((c as f64 - 122.1f64) / 365.25f64) as i32;
    let e: i32 = 365_i32 * d + d / 4_i32;
    let f: i32 = ((c - e) as f64 / 30.6001f64) as i32;
    (t).d = c - e - (30.6001f64 * f as f64) as i32;
    (t).m = f - 1_i32 - 12_i32 * (f / 14_i32);
    (t).y = d - 4715_i32 - (7_i32 + (t).m) / 10_i32;
    (t).hh = ((g).sec / 3600.0f64) as i32 % 24_i32;
    (t).mm = ((g).sec / 60.0f64) as i32 % 60_i32;
    (t).sec = g.sec - 60.0f64 * floor((g).sec / 60.0f64);
}

pub fn xyz2llh(xyz_0: &[f64; 3], llh: &mut [f64; 3]) {
    let mut zdz: f64;
    let mut nh: f64;
    let mut slat: f64;
    let mut n: f64;
    let mut dz_new: f64;
    let a = 6378137.0f64;
    let e = 0.0818191908426f64;
    let eps = 1.0e-3f64;
    let e2 = e * e;
    if normVect(xyz_0) < eps {
        llh[0] = 0.0f64;
        llh[1] = 0.0f64;
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
        nh = sqrt(rho2 + zdz * zdz);
        slat = zdz / nh;
        n = a / sqrt(1.0f64 - e2 * slat * slat);
        dz_new = n * e2 * slat;
        if fabs(dz - dz_new) < eps {
            break;
        }
        dz = dz_new;
    }
    llh[0] = atan2(zdz, sqrt(rho2));
    llh[1] = atan2(y, x);
    llh[2] = nh - n;
}

pub fn llh2xyz(llh: &[f64; 3], xyz_0: &mut [f64; 3]) {
    let a = 6378137.0f64;
    let e = 0.0818191908426f64;
    let e2 = e * e;
    let clat = cos(llh[0]);
    let slat = sin(llh[0]);
    let clon = cos(llh[1]);
    let slon = sin(llh[1]);
    let d = e * slat;
    let n = a / sqrt(1.0f64 - d * d);
    let nph = n + llh[2];
    let tmp = nph * clat;
    xyz_0[0] = tmp * clon;
    xyz_0[1] = tmp * slon;
    xyz_0[2] = ((1.0f64 - e2) * n + llh[2]) * slat;
}

pub fn ltcmat(llh: &[f64; 3], t: &mut [[f64; 3]; 3]) {
    let slat = sin(llh[0]);
    let clat = cos(llh[0]);
    let slon = sin(llh[1]);
    let clon = cos(llh[1]);
    t[0][0] = -slat * clon;
    t[0][1] = -slat * slon;
    t[0][2] = clat;
    t[1][0] = -slon;
    t[1][1] = clon;
    t[1][2] = 0.0f64;
    t[2][0] = clat * clon;
    t[2][1] = clat * slon;
    t[2][2] = slat;
}

pub fn ecef2neu(xyz_0: &[f64; 3], t: &[[f64; 3]; 3], neu: &mut [f64; 3]) {
    neu[0] = t[0][0] * xyz_0[0] + t[0][1] * xyz_0[1] + t[0][2] * xyz_0[2];
    neu[1] = t[1][0] * xyz_0[0] + t[1][1] * xyz_0[1] + t[1][2] * xyz_0[2];
    neu[2] = t[2][0] * xyz_0[0] + t[2][1] * xyz_0[1] + t[2][2] * xyz_0[2];
}

pub fn neu2azel(azel: &mut [f64; 2], neu: &[f64; 3]) {
    azel[0] = atan2(neu[1], neu[0]);
    if azel[0] < 0.0f64 {
        azel[0] += 2.0f64 * PI;
    }
    let ne = sqrt(neu[0] * neu[0] + neu[1] * neu[1]);
    azel[1] = atan2(neu[2], ne);
}

pub fn satpos(
    eph: &ephem_t,
    g: &gpstime_t,
    pos: &mut [f64; 3],
    vel: &mut [f64; 3],
    clk: &mut [f64; 2],
) {
    let mut tk = g.sec - eph.toe.sec;
    if tk > 302400.0f64 {
        tk -= 604800.0f64;
    } else if tk < -302400.0f64 {
        tk += 604800.0f64;
    }
    let mk = eph.m0 + eph.n * tk;
    let mut ek = mk;
    let mut ekold = ek + 1.0f64;
    let mut OneMinusecosE = 0_i32 as f64;
    while fabs(ek - ekold) > 1.0E-14f64 {
        ekold = ek;
        OneMinusecosE = 1.0f64 - eph.ecc * cos(ekold);
        ek += (mk - ekold + eph.ecc * sin(ekold)) / OneMinusecosE;
    }
    let sek = sin(ek);
    let cek = cos(ek);
    let ekdot = eph.n / OneMinusecosE;
    let relativistic = -4.442807633E-10f64 * eph.ecc * eph.sqrta * sek;
    let pk = atan2(eph.sq1e2 * sek, cek - eph.ecc) + eph.aop;
    let pkdot = eph.sq1e2 * ekdot / OneMinusecosE;
    let s2pk = sin(2.0f64 * pk);
    let c2pk = cos(2.0f64 * pk);
    let uk = pk + eph.cus * s2pk + eph.cuc * c2pk;
    let suk = sin(uk);
    let cuk = cos(uk);
    let ukdot = pkdot * (1.0f64 + 2.0f64 * (eph.cus * c2pk - eph.cuc * s2pk));
    let rk = eph.A * OneMinusecosE + eph.crc * c2pk + eph.crs * s2pk;
    let rkdot = eph.A * eph.ecc * sek * ekdot + 2.0f64 * pkdot * (eph.crs * c2pk - eph.crc * s2pk);
    let ik = eph.inc0 + eph.idot * tk + eph.cic * c2pk + eph.cis * s2pk;
    let sik = sin(ik);
    let cik = cos(ik);
    let ikdot = eph.idot + 2.0f64 * pkdot * (eph.cis * c2pk - eph.cic * s2pk);
    let xpk = rk * cuk;
    let ypk = rk * suk;
    let xpkdot = rkdot * cuk - ypk * ukdot;
    let ypkdot = rkdot * suk + xpk * ukdot;
    let ok = eph.omg0 + tk * eph.omgkdot - 7.2921151467e-5f64 * eph.toe.sec;
    let sok = sin(ok);
    let cok = cos(ok);
    pos[0] = xpk * cok - ypk * cik * sok;
    pos[1] = xpk * sok + ypk * cik * cok;
    pos[2] = ypk * sik;
    let tmp = ypkdot * cik - ypk * sik * ikdot;
    vel[0] = -eph.omgkdot * pos[1] + xpkdot * cok - tmp * sok;
    vel[1] = eph.omgkdot * pos[0] + xpkdot * sok + tmp * cok;
    vel[2] = ypk * cik * ikdot + ypkdot * sik;
    let mut tk = g.sec - eph.toc.sec;
    if tk > 302400.0f64 {
        tk -= 604800.0f64;
    } else if tk < -302400.0f64 {
        tk += 604800.0f64;
    }
    clk[0] = eph.af0 + tk * (eph.af1 + tk * eph.af2) + relativistic - eph.tgd;
    clk[1] = eph.af1 + 2.0f64 * tk * eph.af2;
}

pub fn eph2sbf(eph: ephem_t, ionoutc: &ionoutc_t, sbf: &mut [[u32; 10]; 5]) {
    let ura: u32 = 0_u32;
    let dataId: u32 = 1_u32;
    let sbf4_page25_svId: u32 = 63_u32;
    let sbf5_page25_svId: u32 = 51_u32;
    let wnlsf: u32;
    let dtlsf: u32;
    let dn: u32;
    let sbf4_page18_svId: u32 = 56_u32;
    let wn = 0_u32;
    let toe = (eph.toe.sec / 16.0f64) as u32;
    let toc = (eph.toc.sec / 16.0f64) as u32;
    let iode = eph.iode as u32;
    let iodc = eph.iodc as u32;
    let deltan = (eph.deltan / 1.136_868_377_216_16e-13_f64 / PI) as i32;
    let cuc = (eph.cuc / 1.862645149230957e-9f64) as i32;
    let cus = (eph.cus / 1.862645149230957e-9f64) as i32;
    let cic = (eph.cic / 1.862645149230957e-9f64) as i32;
    let cis = (eph.cis / 1.862645149230957e-9f64) as i32;
    let crc = (eph.crc / 0.03125f64) as i32;
    let crs = (eph.crs / 0.03125f64) as i32;
    let ecc = (eph.ecc / 1.164153218269348e-10f64) as u32;
    let sqrta = (eph.sqrta / 1.907_348_632_812_5e-6_f64) as u32;
    let m0 = (eph.m0 / 4.656612873077393e-10f64 / PI) as i32;
    let omg0 = (eph.omg0 / 4.656612873077393e-10f64 / PI) as i32;
    let inc0 = (eph.inc0 / 4.656612873077393e-10f64 / PI) as i32;
    let aop = (eph.aop / 4.656612873077393e-10f64 / PI) as i32;
    let omgdot = (eph.omgdot / 1.136_868_377_216_16e-13_f64 / PI) as i32;
    let idot = (eph.idot / 1.136_868_377_216_16e-13_f64 / PI) as i32;
    let af0 = (eph.af0 / 4.656612873077393e-10f64) as i32;
    let af1 = (eph.af1 / 1.136_868_377_216_16e-13_f64) as i32;
    let af2 = (eph.af2 / 2.775557561562891e-17f64) as i32;
    let tgd = (eph.tgd / 4.656612873077393e-10f64) as i32;
    let svhlth = eph.svhlth as u32 as i32;
    let codeL2 = eph.codeL2 as u32 as i32;
    let wna = (eph.toe.week % 256_i32) as u32;
    let toa = (eph.toe.sec / 4096.0f64) as u32;
    let alpha0 = round(ionoutc.alpha0 / 9.313_225_746_154_785e-10_f64) as i32;
    let alpha1 = round(ionoutc.alpha1 / 7.450_580_596_923_828e-9_f64) as i32;
    let alpha2 = round(ionoutc.alpha2 / 5.960_464_477_539_063e-8_f64) as i32;
    let alpha3 = round(ionoutc.alpha3 / 5.960_464_477_539_063e-8_f64) as i32;
    let beta0 = round(ionoutc.beta0 / 2048.0f64) as i32;
    let beta1 = round(ionoutc.beta1 / 16384.0f64) as i32;
    let beta2 = round(ionoutc.beta2 / 65536.0f64) as i32;
    let beta3 = round(ionoutc.beta3 / 65536.0f64) as i32;
    let A0 = round(ionoutc.A0 / 9.313_225_746_154_785e-10_f64) as i32;
    let A1 = round(ionoutc.A1 / 8.881_784_197_001_252e-16_f64) as i32;
    let dtls = ionoutc.dtls;
    let tot = (ionoutc.tot / 4096_i32) as u32;
    let wnt = (ionoutc.wnt % 256_i32) as u32;
    if ionoutc.leapen == 1_i32 {
        wnlsf = (ionoutc.wnlsf % 256_i32) as u32;
        dn = ionoutc.dn as u32;
        dtlsf = ionoutc.dtlsf as u32;
    } else {
        wnlsf = (1929_i32 % 256_i32) as u32;
        dn = 7_i32 as u32;
        dtlsf = 18_i32 as u32;
    }
    (sbf[0])[0] = 0x8b0000_u32 << 6_i32;
    (sbf[0])[1] = 0x1_u32 << 8_i32;
    (sbf[0])[2] = (wn & 0x3ff_u32) << 20_i32
        | (codeL2 as u32 & 0x3_u32) << 18_i32
        | (ura & 0xf_u32) << 14_i32
        | (svhlth as u32 & 0x3f_u32) << 8_i32
        | (iodc >> 8_i32 & 0x3_u32) << 6_i32;
    (sbf[0])[3] = 0_u32;
    (sbf[0])[4] = 0_u32;
    (sbf[0])[5] = 0_u32;
    (sbf[0])[6] = (tgd as u32 & 0xff_u32) << 6_i32;
    (sbf[0])[7] = (iodc & 0xff_u32) << 22_i32 | (toc & 0xffff_u32) << 6_i32;
    (sbf[0])[8] = (af2 as u32 & 0xff_u32) << 22_i32 | (af1 as u32 & 0xffff_u32) << 6_i32;
    (sbf[0])[9] = (af0 as u32 & 0x3fffff_u32) << 8_i32;
    (sbf[1])[0] = 0x8b0000_u32 << 6_i32;
    (sbf[1])[1] = 0x2_u32 << 8_i32;
    (sbf[1])[2] = (iode & 0xff_u32) << 22_i32 | (crs as u32 & 0xffff_u32) << 6_i32;
    (sbf[1])[3] =
        (deltan as u32 & 0xffff_u32) << 14_i32 | ((m0 >> 24_i32) as u32 & 0xff_u32) << 6_i32;
    (sbf[1])[4] = (m0 as u32 & 0xffffff_u32) << 6_i32;
    (sbf[1])[5] = (cuc as u32 & 0xffff_u32) << 14_i32 | (ecc >> 24_i32 & 0xff_u32) << 6_i32;
    (sbf[1])[6] = (ecc & 0xffffff_u32) << 6_i32;
    (sbf[1])[7] = (cus as u32 & 0xffff_u32) << 14_i32 | (sqrta >> 24_i32 & 0xff_u32) << 6_i32;
    (sbf[1])[8] = (sqrta & 0xffffff_u32) << 6_i32;
    (sbf[1])[9] = (toe & 0xffff_u32) << 14_i32;
    (sbf[2])[0] = 0x8b0000_u32 << 6_i32;
    (sbf[2])[1] = 0x3_u32 << 8_i32;
    (sbf[2])[2] =
        (cic as u32 & 0xffff_u32) << 14_i32 | ((omg0 >> 24_i32) as u32 & 0xff_u32) << 6_i32;
    (sbf[2])[3] = (omg0 as u32 & 0xffffff_u32) << 6_i32;
    (sbf[2])[4] =
        (cis as u32 & 0xffff_u32) << 14_i32 | ((inc0 >> 24_i32) as u32 & 0xff_u32) << 6_i32;
    (sbf[2])[5] = (inc0 as u32 & 0xffffff_u32) << 6_i32;
    (sbf[2])[6] =
        (crc as u32 & 0xffff_u32) << 14_i32 | ((aop >> 24_i32) as u32 & 0xff_u32) << 6_i32;
    (sbf[2])[7] = (aop as u32 & 0xffffff_u32) << 6_i32;
    (sbf[2])[8] = (omgdot as u32 & 0xffffff_u32) << 6_i32;
    (sbf[2])[9] = (iode & 0xff_u32) << 22_i32 | (idot as u32 & 0x3fff_u32) << 8_i32;
    if ionoutc.vflg == 1_i32 {
        (sbf[3])[0] = 0x8b0000_u32 << 6_i32;
        (sbf[3])[1] = 0x4_u32 << 8_i32;
        (sbf[3])[2] = dataId << 28_i32
            | sbf4_page18_svId << 22_i32
            | (alpha0 as u32 & 0xff_u32) << 14_i32
            | (alpha1 as u32 & 0xff_u32) << 6_i32;
        (sbf[3])[3] = (alpha2 as u32 & 0xff_u32) << 22_i32
            | (alpha3 as u32 & 0xff_u32) << 14_i32
            | (beta0 as u32 & 0xff_u32) << 6_i32;
        (sbf[3])[4] = (beta1 as u32 & 0xff_u32) << 22_i32
            | (beta2 as u32 & 0xff_u32) << 14_i32
            | (beta3 as u32 & 0xff_u32) << 6_i32;
        (sbf[3])[5] = (A1 as u32 & 0xffffff_u32) << 6_i32;
        (sbf[3])[6] = ((A0 >> 8_i32) as u32 & 0xffffff_u32) << 6_i32;
        (sbf[3])[7] = (A0 as u32 & 0xff_u32) << 22_i32
            | (tot & 0xff_u32) << 14_i32
            | (wnt & 0xff_u32) << 6_i32;
        (sbf[3])[8] = (dtls as u32 & 0xff_u32) << 22_i32
            | (wnlsf & 0xff_u32) << 14_i32
            | (dn & 0xff_u32) << 6_i32;
        (sbf[3])[9] = (dtlsf & 0xff_u32) << 22_i32;
    } else {
        (sbf[3])[0] = 0x8b0000_u32 << 6_i32;
        (sbf[3])[1] = 0x4_u32 << 8_i32;
        (sbf[3])[2] = dataId << 28_i32 | sbf4_page25_svId << 22_i32;
        (sbf[3])[3] = 0_u32;
        (sbf[3])[4] = 0_u32;
        (sbf[3])[5] = 0_u32;
        (sbf[3])[6] = 0_u32;
        (sbf[3])[7] = 0_u32;
        (sbf[3])[8] = 0_u32;
        (sbf[3])[9] = 0_u32;
    }
    (sbf[4])[0] = 0x8b0000_u32 << 6_i32;
    (sbf[4])[1] = 0x5_u32 << 8_i32;
    (sbf[4])[2] = dataId << 28_i32
        | sbf5_page25_svId << 22_i32
        | (toa & 0xff_u32) << 14_i32
        | (wna & 0xff_u32) << 6_i32;
    (sbf[4])[3] = 0_u32;
    (sbf[4])[4] = 0_u32;
    (sbf[4])[5] = 0_u32;
    (sbf[4])[6] = 0_u32;
    (sbf[4])[7] = 0_u32;
    (sbf[4])[8] = 0_u32;
    (sbf[4])[9] = 0_u32;
}

pub fn countBits(v: u32) -> u32 {
    let S: [i32; 5] = [1_i32, 2_i32, 4_i32, 8_i32, 16_i32];
    let B: [u32; 5] = [
        0x55555555_i32 as u32,
        0x33333333_i32 as u32,
        0xf0f0f0f_i32 as u32,
        0xff00ff_i32 as u32,
        0xffff_i32 as u32,
    ];
    let mut c = v;
    c = (c >> S[0] & B[0]).wrapping_add(c & B[0]);
    c = (c >> S[1] & B[1]).wrapping_add(c & B[1]);
    c = (c >> S[2] & B[2]).wrapping_add(c & B[2]);
    c = (c >> S[3] & B[3]).wrapping_add(c & B[3]);
    c = (c >> S[4] & B[4]).wrapping_add(c & B[4]);
    c
}

pub fn computeChecksum(source: u32, nib: i32) -> u32 {
    let bmask: [u32; 6] = [
        0x3b1f3480_u32,
        0x1d8f9a40_u32,
        0x2ec7cd00_u32,
        0x1763e680_u32,
        0x2bb1f340_u32,
        0xb7a89c0_u32,
    ];
    let mut D: u32;
    let mut d: u32 = source & 0x3fffffc0_u32;
    let D29: u32 = source >> 31_i32 & 0x1_u32;
    let D30: u32 = source >> 30_i32 & 0x1_u32;
    if nib != 0 {
        if D30
            .wrapping_add(countBits(bmask[4] & d))
            .wrapping_rem(2_i32 as u32)
            != 0
        {
            d ^= 0x1_u32 << 6_i32;
        }
        if D29
            .wrapping_add(countBits(bmask[5] & d))
            .wrapping_rem(2_i32 as u32)
            != 0
        {
            d ^= 0x1_u32 << 7_i32;
        }
    }
    D = d;
    if D30 != 0 {
        D ^= 0x3fffffc0_u32;
    }
    D |= D29
        .wrapping_add(countBits(bmask[0] & d))
        .wrapping_rem(2_i32 as u32)
        << 5_i32;
    D |= D30
        .wrapping_add(countBits(bmask[1] & d))
        .wrapping_rem(2_i32 as u32)
        << 4_i32;
    D |= D29
        .wrapping_add(countBits(bmask[2] & d))
        .wrapping_rem(2_i32 as u32)
        << 3_i32;
    D |= D30
        .wrapping_add(countBits(bmask[3] & d))
        .wrapping_rem(2_i32 as u32)
        << 2_i32;
    D |= D30
        .wrapping_add(countBits(bmask[4] & d))
        .wrapping_rem(2_i32 as u32)
        << 1_i32;
    D |= D29
        .wrapping_add(countBits(bmask[5] & d))
        .wrapping_rem(2_i32 as u32);
    D &= 0x3fffffff_u32;
    D
}

pub fn subGpsTime(g1: gpstime_t, g0: gpstime_t) -> f64 {
    let mut dt = g1.sec - g0.sec;
    dt += (g1.week - g0.week) as f64 * 604800.0f64;
    dt
}

pub fn incGpsTime(g0: gpstime_t, dt: f64) -> gpstime_t {
    let mut g1: gpstime_t = gpstime_t { week: 0, sec: 0. };
    g1.week = g0.week;
    g1.sec = g0.sec + dt;
    g1.sec = round(g1.sec * 1000.0f64) / 1000.0f64;
    while g1.sec >= 604800.0f64 {
        g1.sec -= 604800.0f64;
        g1.week += 1;
    }
    while g1.sec < 0.0f64 {
        g1.sec += 604800.0f64;
        g1.week -= 1;
    }
    g1
}

pub fn ionosphericDelay(
    ionoutc: &ionoutc_t,
    g: &gpstime_t,
    llh: &[f64; 3],
    azel: &[f64; 2],
) -> f64 {
    let iono_delay: f64;
    if ionoutc.enable == 0_i32 {
        return 0.0f64;
    }
    let E = azel[1] / PI;
    let phi_u = llh[0] / PI;
    let lam_u = llh[1] / PI;
    let F = 1.0f64 + 16.0f64 * pow(0.53f64 - E, 3.0f64);
    if ionoutc.vflg == 0_i32 {
        iono_delay = F * 5.0e-9f64 * 2.99792458e8f64;
    } else {
        let mut PER: f64;
        let psi = 0.0137f64 / (E + 0.11f64) - 0.022f64;
        let phi_i = phi_u + psi * cos(azel[0]);
        let phi_i = phi_i.clamp(-0.416f64, 0.416f64);
        let lam_i = lam_u + psi * sin(azel[0]) / cos(phi_i * PI);
        let phi_m = phi_i + 0.064f64 * cos((lam_i - 1.617f64) * PI);
        let phi_m2 = phi_m * phi_m;
        let phi_m3 = phi_m2 * phi_m;
        let mut AMP = ionoutc.alpha0
            + ionoutc.alpha1 * phi_m
            + ionoutc.alpha2 * phi_m2
            + ionoutc.alpha3 * phi_m3;
        if AMP < 0.0f64 {
            AMP = 0.0f64;
        }
        PER =
            ionoutc.beta0 + ionoutc.beta1 * phi_m + ionoutc.beta2 * phi_m2 + ionoutc.beta3 * phi_m3;
        if PER < 72000.0f64 {
            PER = 72000.0f64;
        }
        let mut t = 86400.0f64 / 2.0f64 * lam_i + g.sec;
        while t >= 86400.0f64 {
            t -= 86400.0f64;
        }
        while t < 0_i32 as f64 {
            t += 86400.0f64;
        }
        let X = 2.0f64 * PI * (t - 50400.0f64) / PER;
        if fabs(X) < 1.57f64 {
            let X2 = X * X;
            let X4 = X2 * X2;
            iono_delay =
                F * (5.0e-9f64 + AMP * (1.0f64 - X2 / 2.0f64 + X4 / 24.0f64)) * 2.99792458e8f64;
        } else {
            iono_delay = F * 5.0e-9f64 * 2.99792458e8f64;
        }
    }
    iono_delay
}

pub fn computeRange(
    rho: &mut range_t,
    eph: &ephem_t,
    ionoutc: &mut ionoutc_t,
    g: &gpstime_t,
    xyz_0: &[f64; 3],
) {
    let mut pos: [f64; 3] = [0.; 3];
    let mut vel: [f64; 3] = [0.; 3];
    let mut clk: [f64; 2] = [0.; 2];
    let mut los: [f64; 3] = [0.; 3];
    let mut llh: [f64; 3] = [0.; 3];
    let mut neu: [f64; 3] = [0.; 3];
    let mut tmat: [[f64; 3]; 3] = [[0.; 3]; 3];
    satpos(eph, g, &mut pos, &mut vel, &mut clk);
    subVect(&mut los, &pos, xyz_0);
    let tau = normVect(&los) / 2.99792458e8f64;
    pos[0] -= vel[0] * tau;
    pos[1] -= vel[1] * tau;
    pos[2] -= vel[2] * tau;
    let xrot = pos[0] + pos[1] * 7.2921151467e-5f64 * tau;
    let yrot = pos[1] - pos[0] * 7.2921151467e-5f64 * tau;
    pos[0] = xrot;
    pos[1] = yrot;
    subVect(&mut los, &pos, xyz_0);
    let range = normVect(&los);
    (rho).d = range;
    (rho).range = range - 2.99792458e8f64 * clk[0];
    let rate = dotProd(&vel, &los) / range;
    (rho).rate = rate;
    rho.g = *g;
    xyz2llh(xyz_0, &mut llh);
    ltcmat(&llh, &mut tmat);
    ecef2neu(&los, &tmat, &mut neu);
    neu2azel(&mut (rho).azel, &neu);
    (rho).iono_delay = ionosphericDelay(ionoutc, g, &llh, &(rho).azel);
    (rho).range += (rho).iono_delay;
}

pub fn computeCodePhase(chan: &mut channel_t, rho1: range_t, dt: f64) {
    let rhorate = (rho1.range - chan.rho0.range) / dt;
    chan.f_carr = -rhorate / 0.190293672798365f64;
    chan.f_code = 1.023e6f64 + chan.f_carr * (1.0f64 / 1540.0f64);
    let ms =
        (subGpsTime(chan.rho0.g, chan.g0) + 6.0f64 - chan.rho0.range / 2.99792458e8f64) * 1000.0f64;
    let mut ims = ms as i32;
    chan.code_phase = (ms - ims as f64) * 1023_f64;
    chan.iword = ims / 600_i32;
    ims -= chan.iword * 600_i32;
    chan.ibit = ims / 20_i32;
    ims -= chan.ibit * 20_i32;
    chan.icode = ims;
    chan.codeCA = chan.ca[chan.code_phase as i32 as usize] * 2_i32 - 1_i32;
    chan.dataBit =
        (chan.dwrd[chan.iword as usize] >> (29_i32 - chan.ibit) & 0x1_u32) as i32 * 2_i32 - 1_i32;
    chan.rho0 = rho1;
}

pub fn generateNavMsg(g: &gpstime_t, chan: &mut channel_t, init: i32) -> i32 {
    let mut g0: gpstime_t = gpstime_t { week: 0, sec: 0. };
    let mut sbfwrd: u32;
    let mut prevwrd: u32 = 0;
    let mut nib: i32;
    g0.week = g.week;
    g0.sec = ((g.sec + 0.5f64) as u32).wrapping_div(30) as f64 * 30.0f64;
    chan.g0 = g0;
    let wn = (g0.week % 1024_i32) as u32;
    let mut tow = (g0.sec as u32).wrapping_div(6);
    if init == 1_i32 {
        prevwrd = 0_u32;
        let mut iwrd = 0;
        while iwrd < 10 {
            sbfwrd = chan.sbf[4][iwrd];
            if iwrd == 1 {
                sbfwrd |= (tow & 0x1ffff_u32) << 13_i32;
            }
            sbfwrd |= prevwrd << 30_i32 & 0xc0000000_u32;
            nib = if iwrd == 1 || iwrd == 9 { 1_i32 } else { 0_i32 };
            chan.dwrd[iwrd] = computeChecksum(sbfwrd, nib);
            prevwrd = chan.dwrd[iwrd];
            iwrd += 1;
        }
    } else {
        let mut iwrd = 0;
        while iwrd < 10 {
            chan.dwrd[iwrd] = chan.dwrd[10 * 5 + iwrd];
            prevwrd = chan.dwrd[iwrd];
            iwrd += 1;
        }
    }
    let mut isbf = 0;
    while isbf < 5 {
        tow = tow.wrapping_add(1);
        let mut iwrd = 0;
        while iwrd < 10 {
            sbfwrd = chan.sbf[isbf][iwrd];
            if isbf == 0 && iwrd == 2 {
                sbfwrd |= (wn & 0x3ff_u32) << 20_i32;
            }
            if iwrd == 1 {
                sbfwrd |= (tow & 0x1ffff_u32) << 13_i32;
            }
            sbfwrd |= prevwrd << 30_i32 & 0xc0000000_u32;
            nib = if iwrd == 1 || iwrd == 9 { 1_i32 } else { 0_i32 };
            chan.dwrd[(isbf + 1) * 10 + iwrd] = computeChecksum(sbfwrd, nib);
            prevwrd = chan.dwrd[(isbf + 1) * 10 + iwrd];
            iwrd += 1;
        }
        isbf += 1;
    }
    1_i32
}

pub fn checkSatVisibility(
    eph: ephem_t,
    g: &gpstime_t,
    xyz_0: &[f64; 3],
    elvMask: f64,
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
    if eph.vflg != 1_i32 {
        return -1_i32;
    }
    xyz2llh(xyz_0, &mut llh);
    ltcmat(&llh, &mut tmat);
    satpos(&eph, g, &mut pos, &mut vel, &mut clk);
    subVect(&mut los, &pos, xyz_0);
    ecef2neu(&los, &tmat, &mut neu);
    neu2azel(azel, &neu);
    if azel[1] * 57.2957795131f64 > elvMask {
        return 1_i32;
    }
    0_i32
}

pub fn allocateChannel(
    chan: &mut [channel_t; 16],
    eph: &mut [ephem_t; 32],
    ionoutc: &mut ionoutc_t,
    grx: &gpstime_t,
    xyz_0: &[f64; 3],
    mut _elvMask: f64,
    allocatedSat: &mut [i32; 32],
) -> i32 {
    let mut nsat: i32 = 0_i32;
    let mut azel: [f64; 2] = [0.; 2];
    let mut rho: range_t = range_t {
        g: gpstime_t { week: 0, sec: 0. },
        range: 0.,
        rate: 0.,
        d: 0.,
        azel: [0.; 2],
        iono_delay: 0.,
    };
    let ref_0: [f64; 3] = [0.0f64, 0., 0.];
    // #[allow(unused_variables)]
    // let mut r_ref: f64 = 0.;
    // #[allow(unused_variables)]
    // let mut r_xyz: f64;
    let mut phase_ini: f64;
    let mut sv = 0;
    while sv < 32_i32 {
        if checkSatVisibility(eph[sv as usize], grx, xyz_0, 0.0f64, &mut azel) == 1_i32 {
            nsat += 1;
            if allocatedSat[sv as usize] == -1_i32 {
                let mut i = 0;
                while i < 16 {
                    if chan[i].prn == 0_i32 {
                        chan[i].prn = sv + 1_i32;
                        chan[i].azel[0] = azel[0];
                        chan[i].azel[1] = azel[1];
                        codegen(&mut chan[i].ca, (chan[i]).prn);
                        eph2sbf(eph[sv as usize], ionoutc, &mut chan[i].sbf);
                        generateNavMsg(grx, &mut chan[i], 1_i32);
                        computeRange(&mut rho, &eph[sv as usize], ionoutc, grx, xyz_0);
                        (chan[i]).rho0 = rho;
                        // r_xyz = rho.range;
                        computeRange(&mut rho, &eph[sv as usize], ionoutc, grx, &ref_0);
                        // r_ref = rho.range;
                        phase_ini = 0.0f64;
                        phase_ini -= floor(phase_ini);
                        (chan[i]).carr_phase = (512.0f64 * 65536.0f64 * phase_ini) as u32;
                        break;
                    } else {
                        i += 1;
                    }
                }
                if i < 16 {
                    allocatedSat[sv as usize] = i as i32;
                }
            }
        } else if allocatedSat[sv as usize] >= 0_i32 {
            (chan[allocatedSat[sv as usize] as usize]).prn = 0_i32;
            allocatedSat[sv as usize] = -1_i32;
        }
        sv += 1;
    }
    nsat
}

unsafe fn process(argc: i32, argv: *mut *mut libc::c_char) -> i32 {
    let mut allocatedSat: [i32; 32] = [0; 32];

    let mut xyz: [[f64; 3]; USER_MOTION_SIZE] = [[0.; 3]; USER_MOTION_SIZE];
    unsafe {
        let mut fp_out: Option<std::fs::File> = None;
        let mut eph: [[ephem_t; 32]; 15] = [[ephem_t::default(); 32]; 15];
        let mut g0: gpstime_t = gpstime_t { week: 0, sec: 0. };
        let mut llh: [f64; 3] = [0.; 3];
        let mut chan: [channel_t; 16] = [channel_t {
            prn: 0,
            ca: [0; 1023],
            f_carr: 0.,
            f_code: 0.,
            carr_phase: 0,
            carr_phasestep: 0,
            code_phase: 0.,
            g0: gpstime_t::default(),
            sbf: [[0; 10]; 5],
            dwrd: [0; 60],
            iword: 0,
            ibit: 0,
            icode: 0,
            dataBit: 0,
            codeCA: 0,
            azel: [0.; 2],
            rho0: range_t::default(),
        }; 16];
        let elvmask: f64 = 0.0f64;
        let mut umfile: [libc::c_char; 100] = [0; 100];
        let mut staticLocationMode: i32 = 0_i32;
        let mut nmeaGGA: i32 = 0_i32;
        let mut umLLH: i32 = 0_i32;
        let mut navfile: [libc::c_char; 100] = [0; 100];
        let mut outfile: [libc::c_char; 100] = [0; 100];
        let mut gain: [i32; 16] = [0; 16];
        let mut fixed_gain: i32 = 128_i32;
        let mut ant_pat: [f64; 37] = [0.; 37];
        let mut t0: datetime_t = datetime_t::default();
        let mut tmin: datetime_t = datetime_t::default();
        let mut tmax: datetime_t = datetime_t::default();
        let mut gmin: gpstime_t = gpstime_t { week: 0, sec: 0. };
        let mut gmax: gpstime_t = gpstime_t { week: 0, sec: 0. };
        let mut timeoverwrite: i32 = 0_i32;
        let mut ionoutc: ionoutc_t = ionoutc_t::default();
        let mut path_loss_enable: i32 = 1_i32;
        navfile[0] = 0_i32 as libc::c_char;
        umfile[0] = 0_i32 as libc::c_char;
        strcpy(
            outfile.as_mut_ptr(),
            b"gpssim.bin\0" as *const u8 as *const libc::c_char,
        );
        let mut samp_freq = 2.6e6f64;
        let mut data_format = 16_i32;
        g0.week = -1_i32;
        let iduration = USER_MOTION_SIZE as i32;
        let mut duration = iduration as f64 / 10.0f64;
        let mut verb = 0_i32;
        ionoutc.enable = 1_i32;
        ionoutc.leapen = 0_i32;
        loop_through_opts(
            argc,
            argv,
            &mut navfile,
            &mut umfile,
            &mut nmeaGGA,
            &mut umLLH,
            &mut staticLocationMode,
            &mut xyz,
            &mut llh,
            &mut outfile,
            &mut samp_freq,
            &mut data_format,
            &mut ionoutc,
            &mut timeoverwrite,
            &mut t0,
            &mut g0,
            &mut duration,
            &mut fixed_gain,
            &mut path_loss_enable,
            &mut verb,
        );
        if navfile[0] as i32 == 0_i32 {
            eprintln!("ERROR: GPS ephemeris file is not specified.\n");
            panic!();
        }
        if umfile[0] as i32 == 0_i32 && staticLocationMode == 0 {
            staticLocationMode = 1_i32;
            llh[0] = 35.681298f64 / 57.2957795131f64;
            llh[1] = 139.766247f64 / 57.2957795131f64;
            llh[2] = 10.0f64;
        }
        if duration < 0.0f64
            || duration > USER_MOTION_SIZE as i32 as f64 / 10.0f64 && staticLocationMode == 0
            || duration > 86400_f64 && staticLocationMode != 0
        {
            eprintln!("ERROR: Invalid duration.");
            panic!();
        }
        let iduration = (duration * 10.0f64 + 0.5f64) as i32;
        let mut samp_freq = floor(samp_freq / 10.0f64);
        let iq_buff_size = samp_freq as usize;
        samp_freq *= 10.0f64;
        let delt = 1.0f64 / samp_freq;

        let mut numd: i32;
        if staticLocationMode == 0 {
            if nmeaGGA == 1_i32 {
                numd = readNmeaGGA(xyz.as_mut_ptr(), umfile.as_mut_ptr());
            } else if umLLH == 1_i32 {
                numd = readUserMotionLLH(&mut xyz, umfile.as_mut_ptr());
            } else {
                numd = readUserMotion(&mut xyz, umfile.as_mut_ptr());
            }
            if numd == -1_i32 {
                eprintln!("ERROR: Failed to open user motion / NMEA GGA file.");
                panic!();
            } else if numd == 0_i32 {
                eprintln!("ERROR: Failed to read user motion / NMEA GGA data.");
                panic!();
            }
            if numd > iduration {
                numd = iduration;
            }
            xyz2llh(&xyz[0], &mut llh);
        } else {
            eprintln!("Using static location mode.");
            numd = iduration;
            llh2xyz(&llh, &mut xyz[0]);
        }

        eprintln!("xyz = {}, {}, {}", xyz[0][0], xyz[0][1], xyz[0][2],);

        eprintln!(
            "llh = {}, {}, {}",
            llh[0] * 57.2957795131f64,
            llh[1] * 57.2957795131f64,
            llh[2],
        );
        let neph = readRinexNavAll(&mut eph, &mut ionoutc, navfile.as_mut_ptr());
        if neph == 0 {
            eprintln!("ERROR: No ephemeris available.",);
            panic!();
        } else if neph == usize::MAX {
            eprintln!("ERROR: ephemeris file not found.");
            panic!();
        }
        if verb == 1_i32 && ionoutc.vflg == 1_i32 {
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
        let mut sv = 0;
        while sv < 32 {
            if eph[0][sv as usize].vflg == 1_i32 {
                gmin = eph[0][sv as usize].toc;
                tmin = eph[0][sv as usize].t;
                break;
            } else {
                sv += 1;
            }
        }
        gmax.sec = 0_i32 as f64;
        gmax.week = 0_i32;
        tmax.sec = 0_i32 as f64;
        tmax.mm = 0_i32;
        tmax.hh = 0_i32;
        tmax.d = 0_i32;
        tmax.m = 0_i32;
        tmax.y = 0_i32;
        let mut sv = 0;
        while sv < 32 {
            if eph[neph - 1][sv as usize].vflg == 1_i32 {
                gmax = eph[neph - 1][sv as usize].toc;
                tmax = eph[neph - 1][sv as usize].t;
                break;
            } else {
                sv += 1;
            }
        }
        if g0.week >= 0_i32 {
            if timeoverwrite == 1_i32 {
                let mut gtmp: gpstime_t = gpstime_t::default();
                let mut ttmp: datetime_t = datetime_t::default();
                gtmp.week = g0.week;
                gtmp.sec = (g0.sec as i32 / 7200_i32) as f64 * 7200.0f64;
                let dsec = subGpsTime(gtmp, gmin);
                ionoutc.wnt = gtmp.week;
                ionoutc.tot = gtmp.sec as i32;
                let mut sv = 0;
                while sv < 32 {
                    let mut i = 0;
                    while i < neph {
                        if eph[i][sv as usize].vflg == 1_i32 {
                            gtmp = incGpsTime(eph[i][sv as usize].toc, dsec);
                            gps2date(&gtmp, &mut ttmp);
                            eph[i][sv as usize].toc = gtmp;
                            eph[i][sv as usize].t = ttmp;
                            gtmp = incGpsTime(eph[i][sv as usize].toe, dsec);
                            eph[i][sv as usize].toe = gtmp;
                        }
                        i += 1;
                    }
                    sv += 1;
                }
            } else if subGpsTime(g0, gmin) < 0.0f64 || subGpsTime(gmax, g0) < 0.0f64 {
                eprintln!("ERROR: Invalid start time.");
                eprintln!(
                    "tmin = {:4}/{:02}/{:02},{:02}:{:02}:{:0>2.0} ({}:{:.0})",
                    tmin.y, tmin.m, tmin.d, tmin.hh, tmin.mm, tmin.sec, gmin.week, gmin.sec,
                );
                eprintln!(
                    "tmax = {:4}/{:02}/{:02},{:02}:{:02}:{:0>2.0} ({}:{:.0})",
                    tmax.y, tmax.m, tmax.d, tmax.hh, tmax.mm, tmax.sec, gmax.week, gmax.sec,
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

        eprintln!("Duration = {:.1} [sec]", numd as f64 / 10.0f64);
        let mut ieph = usize::MAX;
        let mut i = 0;
        while i < neph {
            let mut sv = 0;
            while sv < 32 {
                if eph[i][sv as usize].vflg == 1_i32 {
                    let dt = subGpsTime(g0, eph[i][sv as usize].toc);
                    if (-3600.0f64..3600.0f64).contains(&dt) {
                        ieph = i;
                        break;
                    }
                }
                sv += 1;
            }
            if ieph != usize::MAX {
                break;
            }
            // if ieph >= 0 {
            //     break;
            // }
            i += 1;
        }
        if ieph == usize::MAX {
            eprintln!("ERROR: No current set of ephemerides has been found.",);
            panic!();
        }
        let mut iq_buff: Vec<i16> = vec![0i16; 2 * iq_buff_size as usize];
        let mut iq8_buff: Vec<i8> = vec![0i8; 2 * iq_buff_size as usize];
        if data_format == 8_i32 {
            iq8_buff = vec![0i8; 2 * iq_buff_size as usize];
        } else if data_format == 1_i32 {
            iq8_buff = vec![0i8; iq_buff_size as usize / 4];
        }
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
        let out_file = String::from_utf8(outfile.iter().map(|&c| c as u8).collect());
        if let Ok(out_file) = out_file {
            if out_file != "-" {
                let file_name = out_file.trim_end_matches("\0");
                fp_out = std::fs::File::create(file_name).ok();
            } else {
                // use stdout
                unimplemented!()
            }
        }
        let mut i = 0;
        while i < 16 {
            chan[i].prn = 0_i32;
            i += 1;
        }
        let mut sv = 0;
        while sv < 32 {
            allocatedSat[sv as usize] = -1_i32;
            sv += 1;
        }
        let mut grx = incGpsTime(g0, 0.0f64);
        allocateChannel(
            &mut chan,
            &mut eph[ieph],
            &mut ionoutc,
            &grx,
            &xyz[0],
            elvmask,
            &mut allocatedSat,
        );
        let mut i = 0_i32;
        while i < 16_i32 {
            if chan[i as usize].prn > 0_i32 {
                eprintln!(
                    "{:02} {:6.1} {:5.1} {:11.1} {:5.1}",
                    chan[i as usize].prn,
                    chan[i as usize].azel[0] * 57.2957795131f64,
                    chan[i as usize].azel[1] * 57.2957795131f64,
                    chan[i as usize].rho0.d,
                    chan[i as usize].rho0.iono_delay,
                );
            }
            i += 1;
        }
        let mut i = 0_i32;
        while i < 37_i32 {
            ant_pat[i as usize] = pow(10.0f64, -ant_pat_db[i as usize] / 20.0f64);
            i += 1;
        }
        let time_start = Instant::now();
        grx = incGpsTime(grx, 0.1f64);
        let mut iumd = 1;
        while iumd < numd {
            let mut i = 0_i32;
            while i < 16_i32 {
                if chan[i as usize].prn > 0_i32 {
                    let mut rho: range_t = range_t {
                        g: gpstime_t { week: 0, sec: 0. },
                        range: 0.,
                        rate: 0.,
                        d: 0.,
                        azel: [0.; 2],
                        iono_delay: 0.,
                    };
                    let sv = chan[i as usize].prn - 1;
                    if staticLocationMode == 0 {
                        computeRange(
                            &mut rho,
                            &eph[ieph][sv as usize],
                            &mut ionoutc,
                            &grx,
                            &xyz[iumd as usize],
                        );
                    } else {
                        computeRange(
                            &mut rho,
                            &eph[ieph][sv as usize],
                            &mut ionoutc,
                            &grx,
                            &xyz[0],
                        );
                    }
                    chan[i as usize].azel[0] = rho.azel[0];
                    chan[i as usize].azel[1] = rho.azel[1];
                    computeCodePhase(&mut *chan.as_mut_ptr().offset(i as isize), rho, 0.1f64);
                    chan[i as usize].carr_phasestep =
                        round(512.0f64 * 65536.0f64 * chan[i as usize].f_carr * delt) as i32;
                    let path_loss = 20200000.0f64 / rho.d;
                    let ibs = ((90.0f64 - rho.azel[1] * 57.2957795131f64) / 5.0f64) as usize;
                    let ant_gain = ant_pat[ibs];
                    if path_loss_enable == 1_i32 {
                        gain[i as usize] = (path_loss * ant_gain * 128.0f64) as i32;
                    } else {
                        gain[i as usize] = fixed_gain;
                    }
                }
                i += 1;
            }
            let mut isamp = 0;
            while isamp < iq_buff_size {
                let mut i_acc: i32 = 0_i32;
                let mut q_acc: i32 = 0_i32;
                let mut i = 0usize;
                while i < 16 {
                    if chan[i].prn > 0_i32 {
                        let iTable = (chan[i].carr_phase >> 16_i32 & 0x1ff_i32 as u32) as usize;
                        let ip = chan[i].dataBit * chan[i].codeCA * cosTable512[iTable] * gain[i];
                        let qp = chan[i].dataBit * chan[i].codeCA * sinTable512[iTable] * gain[i];
                        i_acc += ip;
                        q_acc += qp;
                        chan[i].code_phase += chan[i].f_code * delt;
                        if chan[i].code_phase >= 1023_f64 {
                            chan[i].code_phase -= 1023_f64;
                            chan[i].icode += 1;
                            if chan[i].icode >= 20_i32 {
                                chan[i].icode = 0_i32;
                                chan[i].ibit += 1;
                                if chan[i].ibit >= 30_i32 {
                                    chan[i].ibit = 0_i32;
                                    chan[i].iword += 1;
                                }
                                chan[i].dataBit = (chan[i].dwrd[chan[i].iword as usize]
                                    >> (29_i32 - chan[i].ibit)
                                    & 0x1_u32)
                                    as i32
                                    * 2_i32
                                    - 1_i32;
                            }
                        }
                        chan[i].codeCA =
                            chan[i].ca[chan[i].code_phase as i32 as usize] * 2_i32 - 1_i32;
                        chan[i].carr_phase =
                            (chan[i].carr_phase).wrapping_add(chan[i].carr_phasestep as u32);
                    }
                    i += 1;
                }
                i_acc = (i_acc + 64_i32) >> 7_i32;
                q_acc = (q_acc + 64_i32) >> 7_i32;
                iq_buff[isamp * 2] = i_acc as i16;
                iq_buff[isamp * 2 + 1] = q_acc as i16;
                isamp += 1;
            }
            if data_format == 1_i32 {
                let mut isamp = 0;
                while isamp < 2 * iq_buff_size {
                    if isamp % 8 == 0 {
                        iq8_buff[isamp / 8] = 0i8;
                    }
                    let fresh1_new = &mut iq8_buff[isamp / 8];

                    *fresh1_new = (*fresh1_new as i32
                        | (if iq_buff[isamp] as i32 > 0_i32 {
                            0x1_i32
                        } else {
                            0_i32
                        }) << (7_i32 - isamp as i32 % 8_i32))
                        as libc::c_schar;

                    isamp += 1;
                }

                if let Some(file) = &mut fp_out {
                    file.write_all(std::slice::from_raw_parts(
                        iq8_buff.as_ptr() as *const u8,
                        (iq_buff_size as i32 / 4_i32) as usize,
                    ))
                    .ok();
                }
            } else if data_format == 8_i32 {
                let mut isamp = 0;
                while isamp < 2 * iq_buff_size {
                    iq8_buff[isamp] =
                        (iq_buff[isamp] as i32 >> 4_i32) as libc::c_schar;
                    isamp += 1;
                }

                if let Some(file) = &mut fp_out {
                    file.write_all(std::slice::from_raw_parts(
                        iq8_buff.as_ptr() as *const u8,
                        (2_i32 * iq_buff_size as i32) as usize,
                    ))
                    .ok();
                }
            } else if let Some(file) = &mut fp_out {
                let byte_slice = std::slice::from_raw_parts(
                    iq_buff.as_ptr() as *const u8,
                    (2_i32 * iq_buff_size as i32 * 2) as usize, // 2 bytes per sample
                );
                file.write_all(byte_slice).ok();
            }
            let igrx = (grx.sec * 10.0f64 + 0.5f64) as i32;
            if igrx % 300_i32 == 0_i32 {
                let mut i = 0_i32;
                while i < 16_i32 {
                    if chan[i as usize].prn > 0_i32 {
                        generateNavMsg(&grx, &mut *chan.as_mut_ptr().offset(i as isize), 0_i32);
                    }
                    i += 1;
                }
                let mut sv = 0;
                while sv < 32 {
                    if eph[ieph + 1][sv].vflg == 1_i32 {
                        let dt = subGpsTime(eph[ieph + 1][sv].toc, grx);
                        if dt < 3600.0f64 {
                            ieph += 1;
                            let mut i = 0;
                            while i < 16 {
                                if chan[i].prn != 0_i32 {
                                    eph2sbf(
                                        eph[ieph][(chan[i].prn - 1_i32) as usize],
                                        &ionoutc,
                                        &mut chan[i].sbf,
                                    );
                                }
                                i += 1;
                            }
                        }
                        break;
                    } else {
                        sv += 1;
                    }
                }
                if staticLocationMode == 0 {
                    allocateChannel(
                        &mut chan,
                        &mut eph[ieph],
                        &mut ionoutc,
                        &grx,
                        &xyz[iumd as usize],
                        elvmask,
                        &mut allocatedSat,
                    );
                } else {
                    allocateChannel(
                        &mut chan,
                        &mut eph[ieph],
                        &mut ionoutc,
                        &grx,
                        &xyz[0],
                        elvmask,
                        &mut allocatedSat,
                    );
                }
                if verb == 1_i32 {
                    eprintln!();
                    let mut i = 0_i32;
                    while i < 16_i32 {
                        if chan[i as usize].prn > 0_i32 {
                            eprintln!(
                                "{:02} {:6.1} {:5.1} {:11.1} {:5.1}",
                                chan[i as usize].prn,
                                chan[i as usize].azel[0] * 57.2957795131f64,
                                chan[i as usize].azel[1] * 57.2957795131f64,
                                chan[i as usize].rho0.d,
                                chan[i as usize].rho0.iono_delay,
                            );
                        }
                        i += 1;
                    }
                }
            }
            grx = incGpsTime(grx, 0.1f64);

            eprint!("\rTime into run = {:4.1}\0", subGpsTime(grx, g0));
            // todo: temporarily disable
            // fflush(stdout);
            iumd += 1;
        }

        eprintln!("\nDone!");
        eprintln!(
            "Process time = {:.1} [sec]",
            time_start.elapsed().as_secs_f32()
        );
        0_i32
    }
}
pub fn main() {
    // let mut args: Vec<String> = std::env::args().collect();
    let mut args: Vec<*mut libc::c_char> = Vec::new();
    for arg in ::std::env::args() {
        args.push(
            (::std::ffi::CString::new(arg))
                .expect("Failed to convert argument into CString.")
                .into_raw(),
        );
    }
    args.push(::core::ptr::null_mut());

    if args.len() - 1 < 3 {
        usage();
        panic!();
    }
    unsafe { ::std::process::exit(process((args.len() - 1) as i32, args.as_mut_ptr())) }
}
