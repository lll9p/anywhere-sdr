#![allow(
    dead_code,
    mutable_transmutes,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut,
    static_mut_refs,
    clippy::missing_safety_doc
)]
#![feature(extern_types)]
unsafe extern "C" {
    pub type _IO_wide_data;
    pub type _IO_codecvt;
    pub type _IO_marker;
    // static mut stdout: *mut FILE;

    fn fclose(__stream: *mut FILE) -> i32;
    fn fflush(__stream: *mut FILE) -> i32;
    fn fopen(_: *const libc::c_char, _: *const libc::c_char) -> *mut FILE;

    fn sscanf(_: *const libc::c_char, _: *const libc::c_char, _: ...) -> i32;
    fn fgets(__s: *mut libc::c_char, __n: i32, __stream: *mut FILE) -> *mut libc::c_char;
    fn fwrite(_: *const libc::c_void, _: u32, _: u32, _: *mut FILE) -> u32;
    fn atof(__nptr: *const libc::c_char) -> f64;
    fn atoi(__nptr: *const libc::c_char) -> i32;
    fn calloc(_: u32, _: u32) -> *mut libc::c_void;
    fn free(_: *mut libc::c_void);
    fn exit(_: i32) -> !;
    fn strcpy(_: *mut libc::c_char, _: *const libc::c_char) -> *mut libc::c_char;
    fn strncpy(_: *mut libc::c_char, _: *const libc::c_char, _: u32) -> *mut libc::c_char;
    fn strcmp(_: *const libc::c_char, _: *const libc::c_char) -> i32;
    fn strncmp(_: *const libc::c_char, _: *const libc::c_char, _: u32) -> i32;
    fn strchr(_: *const libc::c_char, _: i32) -> *mut libc::c_char;
    fn strtok(_: *mut libc::c_char, _: *const libc::c_char) -> *mut libc::c_char;
    fn clock() -> clock_t;
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
mod table;
mod utils;

use constants::{PI, USER_MOTION_SIZE};
use datetime::{datetime_t, gpstime_t, tm};
use eph::ephem_t;
use getopt::{getopt, optarg, optind};
use ionoutc::ionoutc_t;
use read_nmea_gga::readNmeaGGA;
use read_rinex::readRinexNavAll;
use table::{ant_pat_db, cosTable512, sinTable512};
use utils::*;

// type size_t = u32;
// type __off_t = libc::c_long;
// type __off64_t = libc::c_long;
// type __clock_t = libc::c_long;
// type __time_t = libc::c_long;
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
pub static mut allocatedSat: [i32; 32] = [0; 32];

pub static mut xyz: [[f64; 3]; USER_MOTION_SIZE] = [[0.; 3]; USER_MOTION_SIZE];

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
    let mut delay: [usize; 32] = [
        5, 6, 7, 8, 17, 18, 139, 140, 141, 251, 252, 254, 255, 256, 257, 258, 469, 470, 471, 472,
        473, 474, 509, 512, 513, 514, 515, 516, 859, 860, 861, 862,
    ];
    let mut g1: [i32; 1023] = [0; 1023];
    let mut g2: [i32; 1023] = [0; 1023];
    let mut r1: [i32; 10] = [0; 10];
    let mut r2: [i32; 10] = [0; 10];
    let mut c1: i32 = 0;
    let mut c2: i32 = 0;
    // let mut i: i32 = 0;
    let mut j = 0;
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
        j = 9;
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
    j = 1023 - delay[(prn - 1) as usize];
    while i < 1023 {
        ca[i] = (1_i32 - g1[i] * g2[j % 1023]) / 2_i32;
        i += 1;
        j += 1;
    }
}

pub fn date2gps(t: &datetime_t, g: &mut gpstime_t) {
    let mut doy: [i32; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let mut ye: i32 = 0;
    let mut de: i32 = 0;
    let mut lpdays: i32 = 0;
    ye = (t).y - 1980_i32;
    lpdays = ye / 4_i32 + 1_i32;
    if ye % 4_i32 == 0_i32 && (t).m <= 2_i32 {
        lpdays -= 1;
    }
    de = ye * 365_i32 + doy[((t).m - 1_i32) as usize] + (t).d + lpdays - 6_i32;
    (g).week = de / 7_i32;
    (g).sec = (de % 7_i32) as f64 * 86400.0f64
        + (t).hh as f64 * 3600.0f64
        + (t).mm as f64 * 60.0f64
        + (t).sec;
}

pub fn gps2date(g: &gpstime_t, t: &mut datetime_t) {
    let mut c: i32 =
        ((7_i32 * (g).week) as f64 + floor((g).sec / 86400.0f64) + 2444245.0f64) as i32 + 1537_i32;
    let mut d: i32 = ((c as f64 - 122.1f64) / 365.25f64) as i32;
    let mut e: i32 = 365_i32 * d + d / 4_i32;
    let mut f: i32 = ((c - e) as f64 / 30.6001f64) as i32;
    (t).d = c - e - (30.6001f64 * f as f64) as i32;
    (t).m = f - 1_i32 - 12_i32 * (f / 14_i32);
    (t).y = d - 4715_i32 - (7_i32 + (t).m) / 10_i32;
    (t).hh = ((g).sec / 3600.0f64) as i32 % 24_i32;
    (t).mm = ((g).sec / 60.0f64) as i32 % 60_i32;
    (t).sec = g.sec - 60.0f64 * floor((g).sec / 60.0f64);
}

pub fn xyz2llh(xyz_0: &[f64; 3], llh: &mut [f64; 3]) {
    let mut a: f64 = 0.;
    let mut eps: f64 = 0.;
    let mut e: f64 = 0.;
    let mut e2: f64 = 0.;
    let mut x: f64 = 0.;
    let mut y: f64 = 0.;
    let mut z: f64 = 0.;
    let mut rho2: f64 = 0.;
    let mut dz: f64 = 0.;
    let mut zdz: f64 = 0.;
    let mut nh: f64 = 0.;
    let mut slat: f64 = 0.;
    let mut n: f64 = 0.;
    let mut dz_new: f64 = 0.;
    a = 6378137.0f64;
    e = 0.0818191908426f64;
    eps = 1.0e-3f64;
    e2 = e * e;
    if normVect(xyz_0) < eps {
        llh[0] = 0.0f64;
        llh[1] = 0.0f64;
        llh[2] = -a;
        return;
    }
    x = xyz_0[0];
    y = xyz_0[1];
    z = xyz_0[2];
    rho2 = x * x + y * y;
    dz = e2 * z;
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
    let mut n: f64 = 0.;
    let mut a: f64 = 0.;
    let mut e: f64 = 0.;
    let mut e2: f64 = 0.;
    let mut clat: f64 = 0.;
    let mut slat: f64 = 0.;
    let mut clon: f64 = 0.;
    let mut slon: f64 = 0.;
    let mut d: f64 = 0.;
    let mut nph: f64 = 0.;
    let mut tmp: f64 = 0.;
    a = 6378137.0f64;
    e = 0.0818191908426f64;
    e2 = e * e;
    clat = cos(llh[0]);
    slat = sin(llh[0]);
    clon = cos(llh[1]);
    slon = sin(llh[1]);
    d = e * slat;
    n = a / sqrt(1.0f64 - d * d);
    nph = n + llh[2];
    tmp = nph * clat;
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
    let mut ne: f64 = 0.;
    azel[0] = atan2(neu[1], neu[0]);
    if azel[0] < 0.0f64 {
        azel[0] += 2.0f64 * PI;
    }
    ne = sqrt(neu[0] * neu[0] + neu[1] * neu[1]);
    azel[1] = atan2(neu[2], ne);
}

pub fn satpos(
    eph: &ephem_t,
    g: &gpstime_t,
    pos: &mut [f64; 3],
    vel: &mut [f64; 3],
    clk: &mut [f64; 2],
) {
    let mut tk: f64 = 0.;
    let mut mk: f64 = 0.;
    let mut ek: f64 = 0.;
    let mut ekold: f64 = 0.;
    let mut ekdot: f64 = 0.;
    let mut cek: f64 = 0.;
    let mut sek: f64 = 0.;
    let mut pk: f64 = 0.;
    let mut pkdot: f64 = 0.;
    let mut c2pk: f64 = 0.;
    let mut s2pk: f64 = 0.;
    let mut uk: f64 = 0.;
    let mut ukdot: f64 = 0.;
    let mut cuk: f64 = 0.;
    let mut suk: f64 = 0.;
    let mut ok: f64 = 0.;
    let mut sok: f64 = 0.;
    let mut cok: f64 = 0.;
    let mut ik: f64 = 0.;
    let mut ikdot: f64 = 0.;
    let mut sik: f64 = 0.;
    let mut cik: f64 = 0.;
    let mut rk: f64 = 0.;
    let mut rkdot: f64 = 0.;
    let mut xpk: f64 = 0.;
    let mut ypk: f64 = 0.;
    let mut xpkdot: f64 = 0.;
    let mut ypkdot: f64 = 0.;
    let mut relativistic: f64 = 0.;
    let mut OneMinusecosE: f64 = 0.;
    let mut tmp: f64 = 0.;
    tk = g.sec - eph.toe.sec;
    if tk > 302400.0f64 {
        tk -= 604800.0f64;
    } else if tk < -302400.0f64 {
        tk += 604800.0f64;
    }
    mk = eph.m0 + eph.n * tk;
    ek = mk;
    ekold = ek + 1.0f64;
    OneMinusecosE = 0_i32 as f64;
    while fabs(ek - ekold) > 1.0E-14f64 {
        ekold = ek;
        OneMinusecosE = 1.0f64 - eph.ecc * cos(ekold);
        ek += (mk - ekold + eph.ecc * sin(ekold)) / OneMinusecosE;
    }
    sek = sin(ek);
    cek = cos(ek);
    ekdot = eph.n / OneMinusecosE;
    relativistic = -4.442807633E-10f64 * eph.ecc * eph.sqrta * sek;
    pk = atan2(eph.sq1e2 * sek, cek - eph.ecc) + eph.aop;
    pkdot = eph.sq1e2 * ekdot / OneMinusecosE;
    s2pk = sin(2.0f64 * pk);
    c2pk = cos(2.0f64 * pk);
    uk = pk + eph.cus * s2pk + eph.cuc * c2pk;
    suk = sin(uk);
    cuk = cos(uk);
    ukdot = pkdot * (1.0f64 + 2.0f64 * (eph.cus * c2pk - eph.cuc * s2pk));
    rk = eph.A * OneMinusecosE + eph.crc * c2pk + eph.crs * s2pk;
    rkdot = eph.A * eph.ecc * sek * ekdot + 2.0f64 * pkdot * (eph.crs * c2pk - eph.crc * s2pk);
    ik = eph.inc0 + eph.idot * tk + eph.cic * c2pk + eph.cis * s2pk;
    sik = sin(ik);
    cik = cos(ik);
    ikdot = eph.idot + 2.0f64 * pkdot * (eph.cis * c2pk - eph.cic * s2pk);
    xpk = rk * cuk;
    ypk = rk * suk;
    xpkdot = rkdot * cuk - ypk * ukdot;
    ypkdot = rkdot * suk + xpk * ukdot;
    ok = eph.omg0 + tk * eph.omgkdot - 7.2921151467e-5f64 * eph.toe.sec;
    sok = sin(ok);
    cok = cos(ok);
    pos[0] = xpk * cok - ypk * cik * sok;
    pos[1] = xpk * sok + ypk * cik * cok;
    pos[2] = ypk * sik;
    tmp = ypkdot * cik - ypk * sik * ikdot;
    vel[0] = -eph.omgkdot * pos[1] + xpkdot * cok - tmp * sok;
    vel[1] = eph.omgkdot * pos[0] + xpkdot * sok + tmp * cok;
    vel[2] = ypk * cik * ikdot + ypkdot * sik;
    tk = g.sec - eph.toc.sec;
    if tk > 302400.0f64 {
        tk -= 604800.0f64;
    } else if tk < -302400.0f64 {
        tk += 604800.0f64;
    }
    clk[0] = eph.af0 + tk * (eph.af1 + tk * eph.af2) + relativistic - eph.tgd;
    clk[1] = eph.af1 + 2.0f64 * tk * eph.af2;
}

pub fn eph2sbf(eph: ephem_t, ionoutc: ionoutc_t, sbf: &mut [[u32; 10]; 5]) {
    let mut wn: u32 = 0;
    let mut toe: u32 = 0;
    let mut toc: u32 = 0;
    let mut iode: u32 = 0;
    let mut iodc: u32 = 0;
    let mut deltan: i32 = 0;
    let mut cuc: i32 = 0;
    let mut cus: i32 = 0;
    let mut cic: i32 = 0;
    let mut cis: i32 = 0;
    let mut crc: i32 = 0;
    let mut crs: i32 = 0;
    let mut ecc: u32 = 0;
    let mut sqrta: u32 = 0;
    let mut m0: i32 = 0;
    let mut omg0: i32 = 0;
    let mut inc0: i32 = 0;
    let mut aop: i32 = 0;
    let mut omgdot: i32 = 0;
    let mut idot: i32 = 0;
    let mut af0: i32 = 0;
    let mut af1: i32 = 0;
    let mut af2: i32 = 0;
    let mut tgd: i32 = 0;
    let mut svhlth: i32 = 0;
    let mut codeL2: i32 = 0;
    let mut ura: u32 = 0_u32;
    let mut dataId: u32 = 1_u32;
    let mut sbf4_page25_svId: u32 = 63_u32;
    let mut sbf5_page25_svId: u32 = 51_u32;
    let mut wna: u32 = 0;
    let mut toa: u32 = 0;
    let mut alpha0: i32 = 0;
    let mut alpha1: i32 = 0;
    let mut alpha2: i32 = 0;
    let mut alpha3: i32 = 0;
    let mut beta0: i32 = 0;
    let mut beta1: i32 = 0;
    let mut beta2: i32 = 0;
    let mut beta3: i32 = 0;
    let mut A0: i32 = 0;
    let mut A1: i32 = 0;
    let mut dtls: i32 = 0;
    let mut tot: u32 = 0;
    let mut wnt: u32 = 0;
    let mut wnlsf: u32 = 0;
    let mut dtlsf: u32 = 0;
    let mut dn: u32 = 0;
    let mut sbf4_page18_svId: u32 = 56_u32;
    wn = 0_u32;
    toe = (eph.toe.sec / 16.0f64) as u32;
    toc = (eph.toc.sec / 16.0f64) as u32;
    iode = eph.iode as u32;
    iodc = eph.iodc as u32;
    deltan = (eph.deltan / 1.136_868_377_216_16e-13_f64 / PI) as i32;
    cuc = (eph.cuc / 1.862645149230957e-9f64) as i32;
    cus = (eph.cus / 1.862645149230957e-9f64) as i32;
    cic = (eph.cic / 1.862645149230957e-9f64) as i32;
    cis = (eph.cis / 1.862645149230957e-9f64) as i32;
    crc = (eph.crc / 0.03125f64) as i32;
    crs = (eph.crs / 0.03125f64) as i32;
    ecc = (eph.ecc / 1.164153218269348e-10f64) as u32;
    sqrta = (eph.sqrta / 1.907_348_632_812_5e-6_f64) as u32;
    m0 = (eph.m0 / 4.656612873077393e-10f64 / PI) as i32;
    omg0 = (eph.omg0 / 4.656612873077393e-10f64 / PI) as i32;
    inc0 = (eph.inc0 / 4.656612873077393e-10f64 / PI) as i32;
    aop = (eph.aop / 4.656612873077393e-10f64 / PI) as i32;
    omgdot = (eph.omgdot / 1.136_868_377_216_16e-13_f64 / PI) as i32;
    idot = (eph.idot / 1.136_868_377_216_16e-13_f64 / PI) as i32;
    af0 = (eph.af0 / 4.656612873077393e-10f64) as i32;
    af1 = (eph.af1 / 1.136_868_377_216_16e-13_f64) as i32;
    af2 = (eph.af2 / 2.775557561562891e-17f64) as i32;
    tgd = (eph.tgd / 4.656612873077393e-10f64) as i32;
    svhlth = eph.svhlth as u32 as i32;
    codeL2 = eph.codeL2 as u32 as i32;
    wna = (eph.toe.week % 256_i32) as u32;
    toa = (eph.toe.sec / 4096.0f64) as u32;
    alpha0 = round(ionoutc.alpha0 / 9.313_225_746_154_785e-10_f64) as i32;
    alpha1 = round(ionoutc.alpha1 / 7.450_580_596_923_828e-9_f64) as i32;
    alpha2 = round(ionoutc.alpha2 / 5.960_464_477_539_063e-8_f64) as i32;
    alpha3 = round(ionoutc.alpha3 / 5.960_464_477_539_063e-8_f64) as i32;
    beta0 = round(ionoutc.beta0 / 2048.0f64) as i32;
    beta1 = round(ionoutc.beta1 / 16384.0f64) as i32;
    beta2 = round(ionoutc.beta2 / 65536.0f64) as i32;
    beta3 = round(ionoutc.beta3 / 65536.0f64) as i32;
    A0 = round(ionoutc.A0 / 9.313_225_746_154_785e-10_f64) as i32;
    A1 = round(ionoutc.A1 / 8.881_784_197_001_252e-16_f64) as i32;
    dtls = ionoutc.dtls;
    tot = (ionoutc.tot / 4096_i32) as u32;
    wnt = (ionoutc.wnt % 256_i32) as u32;
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
    let mut c: u32 = 0;
    let S: [i32; 5] = [1_i32, 2_i32, 4_i32, 8_i32, 16_i32];
    let B: [u32; 5] = [
        0x55555555_i32 as u32,
        0x33333333_i32 as u32,
        0xf0f0f0f_i32 as u32,
        0xff00ff_i32 as u32,
        0xffff_i32 as u32,
    ];
    c = v;
    c = (c >> S[0] & B[0]).wrapping_add(c & B[0]);
    c = (c >> S[1] & B[1]).wrapping_add(c & B[1]);
    c = (c >> S[2] & B[2]).wrapping_add(c & B[2]);
    c = (c >> S[3] & B[3]).wrapping_add(c & B[3]);
    c = (c >> S[4] & B[4]).wrapping_add(c & B[4]);
    c
}

pub fn computeChecksum(source: u32, nib: i32) -> u32 {
    let mut bmask: [u32; 6] = [
        0x3b1f3480_u32,
        0x1d8f9a40_u32,
        0x2ec7cd00_u32,
        0x1763e680_u32,
        0x2bb1f340_u32,
        0xb7a89c0_u32,
    ];
    let mut D: u32 = 0;
    let mut d: u32 = source & 0x3fffffc0_u32;
    let mut D29: u32 = source >> 31_i32 & 0x1_u32;
    let mut D30: u32 = source >> 30_i32 & 0x1_u32;
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
    let mut dt: f64 = 0.;
    dt = g1.sec - g0.sec;
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
    let mut iono_delay: f64 = 0.0f64;
    let mut E: f64 = 0.;
    let mut phi_u: f64 = 0.;
    let mut lam_u: f64 = 0.;
    let mut F: f64 = 0.;
    if ionoutc.enable == 0_i32 {
        return 0.0f64;
    }
    E = azel[1] / PI;
    phi_u = llh[0] / PI;
    lam_u = llh[1] / PI;
    F = 1.0f64 + 16.0f64 * pow(0.53f64 - E, 3.0f64);
    if ionoutc.vflg == 0_i32 {
        iono_delay = F * 5.0e-9f64 * 2.99792458e8f64;
    } else {
        let mut t: f64 = 0.;
        let mut psi: f64 = 0.;
        let mut phi_i: f64 = 0.;
        let mut lam_i: f64 = 0.;
        let mut phi_m: f64 = 0.;
        let mut phi_m2: f64 = 0.;
        let mut phi_m3: f64 = 0.;
        let mut AMP: f64 = 0.;
        let mut PER: f64 = 0.;
        let mut X: f64 = 0.;
        let mut X2: f64 = 0.;
        let mut X4: f64 = 0.;
        psi = 0.0137f64 / (E + 0.11f64) - 0.022f64;
        phi_i = phi_u + psi * cos(azel[0]);
        phi_i = phi_i.clamp(-0.416f64, 0.416f64);
        lam_i = lam_u + psi * sin(azel[0]) / cos(phi_i * PI);
        phi_m = phi_i + 0.064f64 * cos((lam_i - 1.617f64) * PI);
        phi_m2 = phi_m * phi_m;
        phi_m3 = phi_m2 * phi_m;
        AMP = ionoutc.alpha0
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
        t = 86400.0f64 / 2.0f64 * lam_i + g.sec;
        while t >= 86400.0f64 {
            t -= 86400.0f64;
        }
        while t < 0_i32 as f64 {
            t += 86400.0f64;
        }
        X = 2.0f64 * PI * (t - 50400.0f64) / PER;
        if fabs(X) < 1.57f64 {
            X2 = X * X;
            X4 = X2 * X2;
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
    let mut tau: f64 = 0.;
    let mut range: f64 = 0.;
    let mut rate: f64 = 0.;
    let mut xrot: f64 = 0.;
    let mut yrot: f64 = 0.;
    let mut llh: [f64; 3] = [0.; 3];
    let mut neu: [f64; 3] = [0.; 3];
    let mut tmat: [[f64; 3]; 3] = [[0.; 3]; 3];
    satpos(eph, g, &mut pos, &mut vel, &mut clk);
    subVect(&mut los, &pos, xyz_0);
    tau = normVect(&los) / 2.99792458e8f64;
    pos[0] -= vel[0] * tau;
    pos[1] -= vel[1] * tau;
    pos[2] -= vel[2] * tau;
    xrot = pos[0] + pos[1] * 7.2921151467e-5f64 * tau;
    yrot = pos[1] - pos[0] * 7.2921151467e-5f64 * tau;
    pos[0] = xrot;
    pos[1] = yrot;
    subVect(&mut los, &pos, xyz_0);
    range = normVect(&los);
    (rho).d = range;
    (rho).range = range - 2.99792458e8f64 * clk[0];
    rate = dotProd(&vel, &los) / range;
    (rho).rate = rate;
    rho.g = *g;
    xyz2llh(xyz_0, &mut llh);
    ltcmat(&llh, &mut tmat);
    ecef2neu(&los, &tmat, &mut neu);
    neu2azel(&mut (rho).azel, &neu);
    (rho).iono_delay = ionosphericDelay(ionoutc, g, &llh, &(rho).azel);
    (rho).range += (rho).iono_delay;
}

pub unsafe fn computeCodePhase(mut chan: *mut channel_t, mut rho1: range_t, mut dt: f64) {
    unsafe {
        let mut ms: f64 = 0.;
        let mut ims: i32 = 0;
        let mut rhorate: f64 = 0.;
        rhorate = (rho1.range - (*chan).rho0.range) / dt;
        (*chan).f_carr = -rhorate / 0.190293672798365f64;
        (*chan).f_code = 1.023e6f64 + (*chan).f_carr * (1.0f64 / 1540.0f64);
        ms = (subGpsTime((*chan).rho0.g, (*chan).g0) + 6.0f64
            - (*chan).rho0.range / 2.99792458e8f64)
            * 1000.0f64;
        ims = ms as i32;
        (*chan).code_phase = (ms - ims as f64) * 1023_f64;
        (*chan).iword = ims / 600_i32;
        ims -= (*chan).iword * 600_i32;
        (*chan).ibit = ims / 20_i32;
        ims -= (*chan).ibit * 20_i32;
        (*chan).icode = ims;
        (*chan).codeCA = (*chan).ca[(*chan).code_phase as i32 as usize] * 2_i32 - 1_i32;
        (*chan).dataBit = ((*chan).dwrd[(*chan).iword as usize] >> (29_i32 - (*chan).ibit)
            & 0x1_u32) as i32
            * 2_i32
            - 1_i32;
        (*chan).rho0 = rho1;
    }
}

pub unsafe fn readUserMotion(mut xyz_0: *mut [f64; 3], mut filename: *const libc::c_char) -> i32 {
    unsafe {
        let mut fp: *mut FILE = std::ptr::null_mut::<FILE>();
        let mut numd: i32 = 0;
        let mut str: [libc::c_char; 100] = [0; 100];
        let mut t: f64 = 0.;
        let mut x: f64 = 0.;
        let mut y: f64 = 0.;
        let mut z: f64 = 0.;
        fp = fopen(filename, b"rt\0" as *const u8 as *const libc::c_char);
        if fp.is_null() {
            return -1_i32;
        }
        numd = 0_i32;
        while numd < USER_MOTION_SIZE as i32 {
            if (fgets(str.as_mut_ptr(), 100_i32, fp)).is_null() {
                break;
            }
            if -1_i32
                == sscanf(
                    str.as_mut_ptr(),
                    b"%lf,%lf,%lf,%lf\0" as *const u8 as *const libc::c_char,
                    &mut t as *mut f64,
                    &mut x as *mut f64,
                    &mut y as *mut f64,
                    &mut z as *mut f64,
                )
            {
                break;
            }
            (*xyz_0.offset(numd as isize))[0] = x;
            (*xyz_0.offset(numd as isize))[1] = y;
            (*xyz_0.offset(numd as isize))[2] = z;
            numd += 1;
        }
        fclose(fp);
        numd
    }
}

pub unsafe fn readUserMotionLLH(
    mut xyz_full: &mut [[f64; 3]; USER_MOTION_SIZE],
    mut filename: *const libc::c_char,
) -> i32 {
    unsafe {
        let mut fp: *mut FILE = std::ptr::null_mut::<FILE>();
        let mut numd: i32 = 0;
        let mut t: f64 = 0.;
        let mut llh: [f64; 3] = [0.; 3];
        let mut str: [libc::c_char; 100] = [0; 100];
        fp = fopen(filename, b"rt\0" as *const u8 as *const libc::c_char);
        if fp.is_null() {
            return -1_i32;
        }
        numd = 0_i32;
        while numd < USER_MOTION_SIZE as i32 {
            if (fgets(str.as_mut_ptr(), 100_i32, fp)).is_null() {
                break;
            }
            if -1_i32
                == sscanf(
                    str.as_mut_ptr(),
                    b"%lf,%lf,%lf,%lf\0" as *const u8 as *const libc::c_char,
                    &mut t as *mut f64,
                    &mut *llh.as_mut_ptr().offset(0) as *mut f64,
                    &mut *llh.as_mut_ptr().offset(1) as *mut f64,
                    &mut *llh.as_mut_ptr().offset(2) as *mut f64,
                )
            {
                break;
            }
            if llh[0] > 90.0f64 || llh[0] < -90.0f64 || llh[1] > 180.0f64 || llh[1] < -180.0f64 {
                eprintln!(
                    "ERROR: Invalid file format (time[s], latitude[deg], longitude[deg], height [m].\n"
                );
                numd = 0_i32;
                break;
            } else {
                llh[0] /= 57.2957795131f64;
                llh[1] /= 57.2957795131f64;
                llh2xyz(&llh, &mut xyz_full[numd as usize]);
                numd += 1;
            }
        }
        fclose(fp);
        numd
    }
}

pub unsafe fn generateNavMsg(mut g: gpstime_t, mut chan: *mut channel_t, mut init: i32) -> i32 {
    unsafe {
        let mut iwrd: i32 = 0;
        let mut isbf: i32 = 0;
        let mut g0: gpstime_t = gpstime_t { week: 0, sec: 0. };
        let mut wn: u32 = 0;
        let mut tow: u32 = 0;
        let mut sbfwrd: u32 = 0;
        let mut prevwrd: u32 = 0;
        let mut nib: i32 = 0;
        g0.week = g.week;
        g0.sec = ((g.sec + 0.5f64) as u32).wrapping_div(30_u32) as f64 * 30.0f64;
        (*chan).g0 = g0;
        wn = (g0.week % 1024_i32) as u32;
        tow = (g0.sec as u32).wrapping_div(6_u32);
        if init == 1_i32 {
            prevwrd = 0_u32;
            iwrd = 0_i32;
            while iwrd < 10_i32 {
                sbfwrd = (*chan).sbf[4][iwrd as usize];
                if iwrd == 1_i32 {
                    sbfwrd |= (tow & 0x1ffff_u32) << 13_i32;
                }
                sbfwrd |= prevwrd << 30_i32 & 0xc0000000_u32;
                nib = if iwrd == 1_i32 || iwrd == 9_i32 {
                    1_i32
                } else {
                    0_i32
                };
                (*chan).dwrd[iwrd as usize] = computeChecksum(sbfwrd, nib);
                prevwrd = (*chan).dwrd[iwrd as usize];
                iwrd += 1;
            }
        } else {
            iwrd = 0_i32;
            while iwrd < 10_i32 {
                (*chan).dwrd[iwrd as usize] = (*chan).dwrd[(10_i32 * 5_i32 + iwrd) as usize];
                prevwrd = (*chan).dwrd[iwrd as usize];
                iwrd += 1;
            }
        }
        isbf = 0_i32;
        while isbf < 5_i32 {
            tow = tow.wrapping_add(1);
            iwrd = 0_i32;
            while iwrd < 10_i32 {
                sbfwrd = (*chan).sbf[isbf as usize][iwrd as usize];
                if isbf == 0_i32 && iwrd == 2_i32 {
                    sbfwrd |= (wn & 0x3ff_u32) << 20_i32;
                }
                if iwrd == 1_i32 {
                    sbfwrd |= (tow & 0x1ffff_u32) << 13_i32;
                }
                sbfwrd |= prevwrd << 30_i32 & 0xc0000000_u32;
                nib = if iwrd == 1_i32 || iwrd == 9_i32 {
                    1_i32
                } else {
                    0_i32
                };
                (*chan).dwrd[((isbf + 1_i32) * 10_i32 + iwrd) as usize] =
                    computeChecksum(sbfwrd, nib);
                prevwrd = (*chan).dwrd[((isbf + 1_i32) * 10_i32 + iwrd) as usize];
                iwrd += 1;
            }
            isbf += 1;
        }
        1_i32
    }
}

pub fn checkSatVisibility(
    mut eph: ephem_t,
    mut g: gpstime_t,
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
    satpos(&eph, &g, &mut pos, &mut vel, &mut clk);
    subVect(&mut los, &pos, xyz_0);
    ecef2neu(&los, &tmat, &mut neu);
    neu2azel(azel, &neu);
    if azel[1] * 57.2957795131f64 > elvMask {
        return 1_i32;
    }
    0_i32
}

pub unsafe fn allocateChannel(
    chan: &mut [channel_t; 16],
    eph: &mut [ephem_t; 32],
    mut ionoutc: ionoutc_t,
    mut grx: gpstime_t,
    xyz_0: &[f64; 3],
    mut _elvMask: f64,
) -> i32 {
    unsafe {
        let mut nsat: i32 = 0_i32;
        // let mut i: i32 = 0;
        let mut sv = 0;
        let mut azel: [f64; 2] = [0.; 2];
        let mut rho: range_t = range_t {
            g: gpstime_t { week: 0, sec: 0. },
            range: 0.,
            rate: 0.,
            d: 0.,
            azel: [0.; 2],
            iono_delay: 0.,
        };
        let mut ref_0: [f64; 3] = [0.0f64, 0., 0.];
        #[allow(unused_variables)]
        let mut r_ref: f64 = 0.;
        #[allow(unused_variables)]
        let mut r_xyz: f64 = 0.;
        let mut phase_ini: f64 = 0.;
        sv = 0;
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
                            computeRange(&mut rho, &eph[sv as usize], &mut ionoutc, &grx, xyz_0);
                            (chan[i]).rho0 = rho;
                            r_xyz = rho.range;
                            computeRange(&mut rho, &eph[sv as usize], &mut ionoutc, &grx, &ref_0);
                            r_ref = rho.range;
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
}

pub fn usage() {
    eprintln!(
        "Usage: gps-sdr-sim [options]\nOptions:\n  -e <gps_nav>     RINEX navigation file for GPS ephemerides (required)\n  -u <user_motion> User motion file in ECEF x, y, z format (dynamic mode)\n  -x <user_motion> User motion file in lat, lon, height format (dynamic mode)\n  -g <nmea_gga>    NMEA GGA stream (dynamic mode)\n  -c <location>    ECEF X,Y,Z in meters (static mode) e.g. 3967283.154,1022538.181,4872414.484\n  -l <location>    Lat, lon, height (static mode) e.g. 35.681298,139.766247,10.0\n  -L <wnslf,dn,dtslf> User leap future event in GPS week number, day number, next leap second e.g. 2347,3,19\n  -t <date,time>   Scenario start time YYYY/MM/DD,hh:mm:ss\n  -T <date,time>   Overwrite TOC and TOE to scenario start time\n  -d <duration>    Duration [sec] (dynamic mode max: {}, static mode max: {})\n  -o <output>      I/Q sampling data file (default: gpssim.bin)\n  -s <frequency>   Sampling frequency [Hz] (default: 2600000)\n  -b <iq_bits>     I/Q data format [1/8/16] (default: 16)\n  -i               Disable ionospheric delay for spacecraft scenario\n  -p [fixed_gain]  Disable path loss and hold power level constant\n  -v               Show details about simulated channels\n",
        USER_MOTION_SIZE as f64 / 10.0f64,
        86400,
    );
}
unsafe fn process(mut argc: i32, mut argv: *mut *mut libc::c_char) -> i32 {
    unsafe {
        let mut tstart: clock_t = 0;
        let mut tend: clock_t = 0;
        let mut fp: *mut FILE = std::ptr::null_mut::<FILE>();
        let mut sv: i32 = 0;
        let mut neph: i32 = 0;
        let mut ieph: i32 = 0;
        let mut eph: [[ephem_t; 32]; 15] = [[ephem_t::default(); 32]; 15];
        let mut g0: gpstime_t = gpstime_t { week: 0, sec: 0. };
        let mut llh: [f64; 3] = [0.; 3];
        // let mut i: i32 = 0;
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
        let mut elvmask: f64 = 0.0f64;
        let mut ip: i32 = 0;
        let mut qp: i32 = 0;
        let mut iTable: i32 = 0;
        let mut iq_buff: *mut libc::c_short = std::ptr::null_mut::<libc::c_short>();
        let mut iq8_buff: *mut libc::c_schar = std::ptr::null_mut::<libc::c_schar>();
        let mut grx: gpstime_t = gpstime_t::default();
        let mut delt: f64 = 0.;
        let mut isamp: i32 = 0;
        let mut iumd: i32 = 0;
        let mut numd: i32 = 0;
        let mut umfile: [libc::c_char; 100] = [0; 100];
        let mut staticLocationMode: i32 = 0_i32;
        let mut nmeaGGA: i32 = 0_i32;
        let mut umLLH: i32 = 0_i32;
        let mut navfile: [libc::c_char; 100] = [0; 100];
        let mut outfile: [libc::c_char; 100] = [0; 100];
        let mut samp_freq: f64 = 0.;
        let mut iq_buff_size: i32 = 0;
        let mut data_format: i32 = 0;
        let mut result: i32 = 0;
        let mut gain: [i32; 16] = [0; 16];
        let mut path_loss: f64 = 0.;
        let mut ant_gain: f64 = 0.;
        let mut fixed_gain: i32 = 128_i32;
        let mut ant_pat: [f64; 37] = [0.; 37];
        let mut ibs: i32 = 0;
        let mut t0: datetime_t = datetime_t::default();
        let mut tmin: datetime_t = datetime_t::default();
        let mut tmax: datetime_t = datetime_t::default();
        let mut gmin: gpstime_t = gpstime_t { week: 0, sec: 0. };
        let mut gmax: gpstime_t = gpstime_t { week: 0, sec: 0. };
        let mut dt: f64 = 0.;
        let mut igrx: i32 = 0;
        let mut duration: f64 = 0.;
        let mut iduration: i32 = 0;
        let mut verb: i32 = 0;
        let mut timeoverwrite: i32 = 0_i32;
        let mut ionoutc: ionoutc_t = ionoutc_t::default();
        let mut path_loss_enable: i32 = 1_i32;
        navfile[0] = 0_i32 as libc::c_char;
        umfile[0] = 0_i32 as libc::c_char;
        strcpy(
            outfile.as_mut_ptr(),
            b"gpssim.bin\0" as *const u8 as *const libc::c_char,
        );
        samp_freq = 2.6e6f64;
        data_format = 16_i32;
        g0.week = -1_i32;
        iduration = USER_MOTION_SIZE as i32;
        duration = iduration as f64 / 10.0f64;
        verb = 0_i32;
        ionoutc.enable = 1_i32;
        ionoutc.leapen = 0_i32;
        if argc < 3_i32 {
            usage();
            exit(1_i32);
        }
        loop {
            result = getopt(
                argc,
                argv as *const *mut libc::c_char,
                b"e:u:x:g:c:l:o:s:b:L:T:t:d:ipv\0" as *const u8 as *const libc::c_char,
            );
            if result == -1_i32 {
                break;
            }
            let mut current_block_85: u64;
            match result {
                101 => {
                    strcpy(navfile.as_mut_ptr(), optarg);
                    current_block_85 = 2750570471926810434;
                }
                117 => {
                    strcpy(umfile.as_mut_ptr(), optarg);
                    nmeaGGA = 0_i32;
                    umLLH = 0_i32;
                    current_block_85 = 2750570471926810434;
                }
                120 => {
                    strcpy(umfile.as_mut_ptr(), optarg);
                    umLLH = 1_i32;
                    current_block_85 = 2750570471926810434;
                }
                103 => {
                    strcpy(umfile.as_mut_ptr(), optarg);
                    nmeaGGA = 1_i32;
                    current_block_85 = 2750570471926810434;
                }
                99 => {
                    staticLocationMode = 1_i32;
                    sscanf(
                        optarg,
                        b"%lf,%lf,%lf\0" as *const u8 as *const libc::c_char,
                        &mut *(*xyz.as_mut_ptr().offset(0)).as_mut_ptr().offset(0) as *mut f64,
                        &mut *(*xyz.as_mut_ptr().offset(0)).as_mut_ptr().offset(1) as *mut f64,
                        &mut *(*xyz.as_mut_ptr().offset(0)).as_mut_ptr().offset(2) as *mut f64,
                    );
                    current_block_85 = 2750570471926810434;
                }
                108 => {
                    staticLocationMode = 1_i32;
                    sscanf(
                        optarg,
                        b"%lf,%lf,%lf\0" as *const u8 as *const libc::c_char,
                        &mut *llh.as_mut_ptr().offset(0) as *mut f64,
                        &mut *llh.as_mut_ptr().offset(1) as *mut f64,
                        &mut *llh.as_mut_ptr().offset(2) as *mut f64,
                    );
                    llh[0] /= 57.2957795131f64;
                    llh[1] /= 57.2957795131f64;
                    llh2xyz(&llh, &mut xyz[0]);
                    current_block_85 = 2750570471926810434;
                }
                111 => {
                    strcpy(outfile.as_mut_ptr(), optarg);
                    current_block_85 = 2750570471926810434;
                }
                115 => {
                    samp_freq = atof(optarg);
                    if samp_freq < 1.0e6f64 {
                        eprintln!("ERROR: Invalid sampling frequency.\n");
                        exit(1_i32);
                    }
                    current_block_85 = 2750570471926810434;
                }
                98 => {
                    data_format = atoi(optarg);
                    if data_format != 1_i32 && data_format != 8_i32 && data_format != 16_i32 {
                        eprintln!("ERROR: Invalid I/Q data format.\n");
                        exit(1_i32);
                    }
                    current_block_85 = 2750570471926810434;
                }
                76 => {
                    ionoutc.leapen = 1_i32;
                    sscanf(
                        optarg,
                        b"%d,%d,%d\0" as *const u8 as *const libc::c_char,
                        &mut ionoutc.wnlsf as *mut i32,
                        &mut ionoutc.dn as *mut i32,
                        &mut ionoutc.dtlsf as *mut i32,
                    );
                    // original gps-sdr-sim logical mistake
                    if ionoutc.dn < 1_i32 || ionoutc.dn > 7_i32 {
                        eprintln!("ERROR: Invalid GPS day number");
                        exit(1_i32);
                    }
                    if ionoutc.wnlsf < 0_i32 {
                        eprintln!("ERROR: Invalid GPS week number");
                        exit(1_i32);
                    }
                    // original gps-sdr-sim logical mistake
                    if ionoutc.dtlsf < -128_i32 || ionoutc.dtlsf > 127_i32 {
                        eprintln!("ERROR: Invalid delta leap second");
                        exit(1_i32);
                    }
                    current_block_85 = 2750570471926810434;
                }
                84 => {
                    timeoverwrite = 1_i32;
                    if strncmp(
                        optarg,
                        b"now\0" as *const u8 as *const libc::c_char,
                        3_i32 as u32,
                    ) == 0_i32
                    {
                        let mut timer: time_t = 0;
                        let mut gmt: *mut tm = std::ptr::null_mut::<tm>();
                        time(&mut timer);
                        gmt = gmtime(&timer);
                        t0.y = (*gmt).tm_year + 1900_i32;
                        t0.m = (*gmt).tm_mon + 1_i32;
                        t0.d = (*gmt).tm_mday;
                        t0.hh = (*gmt).tm_hour;
                        t0.mm = (*gmt).tm_min;
                        t0.sec = (*gmt).tm_sec as f64;
                        date2gps(&t0, &mut g0);
                        current_block_85 = 2750570471926810434;
                    } else {
                        current_block_85 = 4676144417340510455;
                    }
                }
                116 => {
                    current_block_85 = 4676144417340510455;
                }
                100 => {
                    duration = atof(optarg);
                    current_block_85 = 2750570471926810434;
                }
                105 => {
                    ionoutc.enable = 0_i32;
                    current_block_85 = 2750570471926810434;
                }
                112 => {
                    if optind < argc
                        && *(*argv.offset(optind as isize)).offset(0) as i32 != '-' as i32
                    {
                        fixed_gain = atoi(*argv.offset(optind as isize));
                        if !(1_i32..=128_i32).contains(&fixed_gain) {
                            eprintln!("ERROR: Fixed gain must be between 1 and 128.\n");
                            exit(1_i32);
                        }
                        optind += 1;
                    }
                    path_loss_enable = 0_i32;
                    current_block_85 = 2750570471926810434;
                }
                118 => {
                    verb = 1_i32;
                    current_block_85 = 2750570471926810434;
                }
                58 | 63 => {
                    usage();
                    exit(1_i32);
                }
                _ => {
                    current_block_85 = 2750570471926810434;
                }
            }
            if current_block_85 == 4676144417340510455 {
                sscanf(
                    optarg,
                    b"%d/%d/%d,%d:%d:%lf\0" as *const u8 as *const libc::c_char,
                    &mut t0.y as *mut i32,
                    &mut t0.m as *mut i32,
                    &mut t0.d as *mut i32,
                    &mut t0.hh as *mut i32,
                    &mut t0.mm as *mut i32,
                    &mut t0.sec as *mut f64,
                );
                if t0.y <= 1980_i32
                    || t0.m < 1_i32
                    || t0.m > 12_i32
                    || t0.d < 1_i32
                    || t0.d > 31_i32
                    || t0.hh < 0_i32
                    || t0.hh > 23_i32
                    || t0.mm < 0_i32
                    || t0.mm > 59_i32
                    || t0.sec < 0.0f64
                    || t0.sec >= 60.0f64
                {
                    eprintln!("ERROR: Invalid date and time.\n");
                    exit(1_i32);
                }
                t0.sec = floor(t0.sec);
                date2gps(&t0, &mut g0);
            }
        }
        if navfile[0] as i32 == 0_i32 {
            eprintln!("ERROR: GPS ephemeris file is not specified.\n");
            exit(1_i32);
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
            exit(1_i32);
        }
        iduration = (duration * 10.0f64 + 0.5f64) as i32;
        samp_freq = floor(samp_freq / 10.0f64);
        iq_buff_size = samp_freq as i32;
        samp_freq *= 10.0f64;
        delt = 1.0f64 / samp_freq;
        if staticLocationMode == 0 {
            if nmeaGGA == 1_i32 {
                numd = readNmeaGGA(xyz.as_mut_ptr(), umfile.as_mut_ptr());
            } else if umLLH == 1_i32 {
                numd = readUserMotionLLH(&mut xyz, umfile.as_mut_ptr());
            } else {
                numd = readUserMotion(xyz.as_mut_ptr(), umfile.as_mut_ptr());
            }
            if numd == -1_i32 {
                eprintln!("ERROR: Failed to open user motion / NMEA GGA file.");
                exit(1_i32);
            } else if numd == 0_i32 {
                eprintln!("ERROR: Failed to read user motion / NMEA GGA data.");
                exit(1_i32);
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
        neph = readRinexNavAll(eph.as_mut_ptr(), &mut ionoutc, navfile.as_mut_ptr());
        if neph == 0_i32 {
            eprintln!("ERROR: No ephemeris available.",);
            exit(1_i32);
        } else if neph == -1_i32 {
            eprintln!("ERROR: ephemeris file not found.");
            exit(1_i32);
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
        sv = 0_i32;
        while sv < 32_i32 {
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
        sv = 0_i32;
        while sv < 32_i32 {
            if eph[(neph - 1_i32) as usize][sv as usize].vflg == 1_i32 {
                gmax = eph[(neph - 1_i32) as usize][sv as usize].toc;
                tmax = eph[(neph - 1_i32) as usize][sv as usize].t;
                break;
            } else {
                sv += 1;
            }
        }
        if g0.week >= 0_i32 {
            if timeoverwrite == 1_i32 {
                let mut gtmp: gpstime_t = gpstime_t::default();
                let mut ttmp: datetime_t = datetime_t::default();
                let mut dsec: f64 = 0.;
                gtmp.week = g0.week;
                gtmp.sec = (g0.sec as i32 / 7200_i32) as f64 * 7200.0f64;
                dsec = subGpsTime(gtmp, gmin);
                ionoutc.wnt = gtmp.week;
                ionoutc.tot = gtmp.sec as i32;
                sv = 0_i32;
                while sv < 32_i32 {
                    let mut i = 0;
                    while i < neph {
                        if eph[i as usize][sv as usize].vflg == 1_i32 {
                            gtmp = incGpsTime(eph[i as usize][sv as usize].toc, dsec);
                            gps2date(&gtmp, &mut ttmp);
                            eph[i as usize][sv as usize].toc = gtmp;
                            eph[i as usize][sv as usize].t = ttmp;
                            gtmp = incGpsTime(eph[i as usize][sv as usize].toe, dsec);
                            eph[i as usize][sv as usize].toe = gtmp;
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
                exit(1_i32);
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
        ieph = -1_i32;
        let mut i = 0;
        while i < neph {
            sv = 0_i32;
            while sv < 32_i32 {
                if eph[i as usize][sv as usize].vflg == 1_i32 {
                    dt = subGpsTime(g0, eph[i as usize][sv as usize].toc);
                    if (-3600.0f64..3600.0f64).contains(&dt) {
                        ieph = i;
                        break;
                    }
                }
                sv += 1;
            }
            if ieph >= 0_i32 {
                break;
            }
            i += 1;
        }
        if ieph == -1_i32 {
            eprintln!("ERROR: No current set of ephemerides has been found.",);
            exit(1_i32);
        }
        iq_buff = calloc((2_i32 * iq_buff_size) as u32, 2_i32 as u32) as *mut libc::c_short;
        if iq_buff.is_null() {
            eprintln!("ERROR: Failed to allocate 16-bit I/Q buffer.");
            exit(1_i32);
        }
        if data_format == 8_i32 {
            iq8_buff = calloc((2_i32 * iq_buff_size) as u32, 1_i32 as u32) as *mut libc::c_schar;
            if iq8_buff.is_null() {
                eprintln!("ERROR: Failed to allocate 8-bit I/Q buffer.");
                exit(1_i32);
            }
        } else if data_format == 1_i32 {
            iq8_buff = calloc((iq_buff_size / 4_i32) as u32, 1_i32 as u32) as *mut libc::c_schar;
            if iq8_buff.is_null() {
                eprintln!("ERROR: Failed to allocate compressed 1-bit I/Q buffer.");
                exit(1_i32);
            }
        }
        if strcmp(
            b"-\0" as *const u8 as *const libc::c_char,
            outfile.as_mut_ptr(),
        ) != 0
        {
            fp = fopen(
                outfile.as_mut_ptr(),
                b"wb\0" as *const u8 as *const libc::c_char,
            );
            if fp.is_null() {
                eprintln!("ERROR: Failed to open output file.");
                exit(1_i32);
            }
        } else {
            // todo: temporarily disable
            // fp = stdout;
        }
        let mut i = 0;
        while i < 16 {
            chan[i].prn = 0_i32;
            i += 1;
        }
        sv = 0_i32;
        while sv < 32_i32 {
            allocatedSat[sv as usize] = -1_i32;
            sv += 1;
        }
        grx = incGpsTime(g0, 0.0f64);
        allocateChannel(
            &mut chan,
            &mut eph[ieph as usize],
            ionoutc,
            grx,
            &xyz[0],
            elvmask,
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
        tstart = clock();
        grx = incGpsTime(grx, 0.1f64);
        iumd = 1_i32;
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
                    sv = chan[i as usize].prn - 1_i32;
                    if staticLocationMode == 0 {
                        computeRange(
                            &mut rho,
                            &eph[ieph as usize][sv as usize],
                            &mut ionoutc,
                            &grx,
                            &xyz[iumd as usize],
                        );
                    } else {
                        computeRange(
                            &mut rho,
                            &eph[ieph as usize][sv as usize],
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
                    path_loss = 20200000.0f64 / rho.d;
                    ibs = ((90.0f64 - rho.azel[1] * 57.2957795131f64) / 5.0f64) as i32;
                    ant_gain = ant_pat[ibs as usize];
                    if path_loss_enable == 1_i32 {
                        gain[i as usize] = (path_loss * ant_gain * 128.0f64) as i32;
                    } else {
                        gain[i as usize] = fixed_gain;
                    }
                }
                i += 1;
            }
            isamp = 0_i32;
            while isamp < iq_buff_size {
                let mut i_acc: i32 = 0_i32;
                let mut q_acc: i32 = 0_i32;
                let mut i = 0usize;
                while i < 16 {
                    if chan[i].prn > 0_i32 {
                        iTable = (chan[i].carr_phase >> 16_i32 & 0x1ff_i32 as u32) as i32;
                        ip = chan[i].dataBit
                            * chan[i].codeCA
                            * cosTable512[iTable as usize]
                            * gain[i];
                        qp = chan[i].dataBit
                            * chan[i].codeCA
                            * sinTable512[iTable as usize]
                            * gain[i];
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
                *iq_buff.offset((isamp * 2_i32) as isize) = i_acc as libc::c_short;
                *iq_buff.offset((isamp * 2_i32 + 1_i32) as isize) = q_acc as libc::c_short;
                isamp += 1;
            }
            if data_format == 1_i32 {
                isamp = 0_i32;
                while isamp < 2_i32 * iq_buff_size {
                    if isamp % 8_i32 == 0_i32 {
                        *iq8_buff.offset((isamp / 8_i32) as isize) = 0_i32 as libc::c_schar;
                    }
                    let fresh1 = &mut (*iq8_buff.offset((isamp / 8_i32) as isize));
                    *fresh1 = (*fresh1 as i32
                        | (if *iq_buff.offset(isamp as isize) as i32 > 0_i32 {
                            0x1_i32
                        } else {
                            0_i32
                        }) << (7_i32 - isamp % 8_i32))
                        as libc::c_schar;
                    isamp += 1;
                }
                fwrite(
                    iq8_buff as *const libc::c_void,
                    1_i32 as u32,
                    (iq_buff_size / 4_i32) as u32,
                    fp,
                );
            } else if data_format == 8_i32 {
                isamp = 0_i32;
                while isamp < 2_i32 * iq_buff_size {
                    *iq8_buff.offset(isamp as isize) =
                        (*iq_buff.offset(isamp as isize) as i32 >> 4_i32) as libc::c_schar;
                    isamp += 1;
                }
                fwrite(
                    iq8_buff as *const libc::c_void,
                    1_i32 as u32,
                    (2_i32 * iq_buff_size) as u32,
                    fp,
                );
            } else {
                fwrite(
                    iq_buff as *const libc::c_void,
                    2_i32 as u32,
                    (2_i32 * iq_buff_size) as u32,
                    fp,
                );
            }
            igrx = (grx.sec * 10.0f64 + 0.5f64) as i32;
            if igrx % 300_i32 == 0_i32 {
                let mut i = 0_i32;
                while i < 16_i32 {
                    if chan[i as usize].prn > 0_i32 {
                        generateNavMsg(grx, &mut *chan.as_mut_ptr().offset(i as isize), 0_i32);
                    }
                    i += 1;
                }
                sv = 0_i32;
                while sv < 32_i32 {
                    if eph[(ieph + 1_i32) as usize][sv as usize].vflg == 1_i32 {
                        dt = subGpsTime(eph[(ieph + 1_i32) as usize][sv as usize].toc, grx);
                        if dt < 3600.0f64 {
                            ieph += 1;
                            let mut i = 0_i32;
                            while i < 16_i32 {
                                if chan[i as usize].prn != 0_i32 {
                                    eph2sbf(
                                        eph[ieph as usize][(chan[i as usize].prn - 1_i32) as usize],
                                        ionoutc,
                                        &mut chan[i as usize].sbf,
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
                        &mut eph[ieph as usize],
                        ionoutc,
                        grx,
                        &xyz[iumd as usize],
                        elvmask,
                    );
                } else {
                    allocateChannel(
                        &mut chan,
                        &mut eph[ieph as usize],
                        ionoutc,
                        grx,
                        &xyz[0],
                        elvmask,
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
        tend = clock();

        eprintln!("\nDone!");
        free(iq_buff as *mut libc::c_void);
        fclose(fp);

        eprintln!(
            "Process time = {:.1} [sec]",
            (tend - tstart) as f64 / 1000000_i32 as clock_t as f64,
        );
        0_i32
    }
}
pub fn main() {
    let mut args: Vec<*mut libc::c_char> = Vec::new();
    for arg in ::std::env::args() {
        args.push(
            (::std::ffi::CString::new(arg))
                .expect("Failed to convert argument into CString.")
                .into_raw(),
        );
    }
    args.push(::core::ptr::null_mut());
    unsafe { ::std::process::exit(process((args.len() - 1) as i32, args.as_mut_ptr())) }
}
