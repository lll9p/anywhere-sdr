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
    fn atan2(_: f64, _: f64) -> f64;
    fn cos(_: f64) -> f64;
    fn sin(_: f64) -> f64;
    fn pow(_: f64, _: f64) -> f64;
    fn sqrt(_: f64) -> f64;
    fn fabs(_: f64) -> f64;
    fn floor(_: f64) -> f64;
    fn round(_: f64) -> f64;
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

use constants::{PI, USER_MOTION_SIZE};
use datetime::{datetime_t, gpstime_t, tm};
use eph::ephem_t;
use getopt::{getopt, optarg, optind};
use ionoutc::ionoutc_t;
use read_nmea_gga::readNmeaGGA;
use read_rinex::readRinexNavAll;
use table::{ant_pat_db, cosTable512, sinTable512};

pub type size_t = u32;
pub type __off_t = libc::c_long;
pub type __off64_t = libc::c_long;
pub type __clock_t = libc::c_long;
pub type __time_t = libc::c_long;
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
    pub _old_offset: __off_t,
    pub _cur_column: libc::c_ushort,
    pub _vtable_offset: libc::c_schar,
    pub _shortbuf: [libc::c_char; 1],
    pub _lock: *mut libc::c_void,
    pub _offset: __off64_t,
    pub _codecvt: *mut _IO_codecvt,
    pub _wide_data: *mut _IO_wide_data,
    pub _freeres_list: *mut _IO_FILE,
    pub _freeres_buf: *mut libc::c_void,
    pub __pad5: size_t,
    pub _mode: i32,
    pub _unused2: [libc::c_char; 20],
}
pub type _IO_lock_t = ();
pub type FILE = _IO_FILE;
pub type clock_t = __clock_t;
pub type time_t = __time_t;

#[derive(Copy, Clone)]
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

pub unsafe fn subVect(mut y: *mut f64, mut x1: *const f64, mut x2: *const f64) {
    unsafe {
        *y.offset(0) = *x1.offset(0) - *x2.offset(0);
        *y.offset(1) = *x1.offset(1) - *x2.offset(1);
        *y.offset(2) = *x1.offset(2) - *x2.offset(2);
    }
}

pub unsafe fn normVect(mut x: *const f64) -> f64 {
    unsafe {
        sqrt(
            *x.offset(0) * *x.offset(0) + *x.offset(1) * *x.offset(1) + *x.offset(2) * *x.offset(2),
        )
    }
}

pub unsafe fn dotProd(mut x1: *const f64, mut x2: *const f64) -> f64 {
    unsafe {
        *x1.offset(0) * *x2.offset(0)
            + *x1.offset(1) * *x2.offset(1)
            + *x1.offset(2) * *x2.offset(2)
    }
}

pub unsafe fn codegen(mut ca: *mut i32, mut prn: i32) {
    unsafe {
        let mut delay: [i32; 32] = [
            5_i32, 6_i32, 7_i32, 8_i32, 17_i32, 18_i32, 139_i32, 140_i32, 141_i32, 251_i32,
            252_i32, 254_i32, 255_i32, 256_i32, 257_i32, 258_i32, 469_i32, 470_i32, 471_i32,
            472_i32, 473_i32, 474_i32, 509_i32, 512_i32, 513_i32, 514_i32, 515_i32, 516_i32,
            859_i32, 860_i32, 861_i32, 862_i32,
        ];
        let mut g1: [i32; 1023] = [0; 1023];
        let mut g2: [i32; 1023] = [0; 1023];
        let mut r1: [i32; 10] = [0; 10];
        let mut r2: [i32; 10] = [0; 10];
        let mut c1: i32 = 0;
        let mut c2: i32 = 0;
        let mut i: i32 = 0;
        let mut j: i32 = 0;
        if !(1_i32..=32_i32).contains(&prn) {
            return;
        }
        i = 0_i32;
        while i < 10_i32 {
            r2[i as usize] = -1_i32;
            r1[i as usize] = r2[i as usize];
            i += 1;
        }
        i = 0_i32;
        while i < 1023_i32 {
            g1[i as usize] = r1[9_i32 as usize];
            g2[i as usize] = r2[9_i32 as usize];
            c1 = r1[2_i32 as usize] * r1[9_i32 as usize];
            c2 = r2[1_i32 as usize]
                * r2[2_i32 as usize]
                * r2[5_i32 as usize]
                * r2[7_i32 as usize]
                * r2[8_i32 as usize]
                * r2[9_i32 as usize];
            j = 9_i32;
            while j > 0_i32 {
                r1[j as usize] = r1[(j - 1_i32) as usize];
                r2[j as usize] = r2[(j - 1_i32) as usize];
                j -= 1;
            }
            r1[0_i32 as usize] = c1;
            r2[0_i32 as usize] = c2;
            i += 1;
        }
        i = 0_i32;
        j = 1023_i32 - delay[(prn - 1_i32) as usize];
        while i < 1023_i32 {
            *ca.offset(i as isize) = (1_i32 - g1[i as usize] * g2[(j % 1023_i32) as usize]) / 2_i32;
            i += 1;
            j += 1;
        }
    }
}

pub unsafe fn date2gps(mut t: *const datetime_t, mut g: *mut gpstime_t) {
    unsafe {
        let mut doy: [i32; 12] = [
            0_i32, 31_i32, 59_i32, 90_i32, 120_i32, 151_i32, 181_i32, 212_i32, 243_i32, 273_i32,
            304_i32, 334_i32,
        ];
        let mut ye: i32 = 0;
        let mut de: i32 = 0;
        let mut lpdays: i32 = 0;
        ye = (*t).y - 1980_i32;
        lpdays = ye / 4_i32 + 1_i32;
        if ye % 4_i32 == 0_i32 && (*t).m <= 2_i32 {
            lpdays -= 1;
        }
        de = ye * 365_i32 + doy[((*t).m - 1_i32) as usize] + (*t).d + lpdays - 6_i32;
        (*g).week = de / 7_i32;
        (*g).sec = (de % 7_i32) as f64 * 86400.0f64
            + (*t).hh as f64 * 3600.0f64
            + (*t).mm as f64 * 60.0f64
            + (*t).sec;
    }
}

pub unsafe fn gps2date(mut g: *const gpstime_t, mut t: *mut datetime_t) {
    unsafe {
        let mut c: i32 = ((7_i32 * (*g).week) as f64 + floor((*g).sec / 86400.0f64) + 2444245.0f64)
            as i32
            + 1537_i32;
        let mut d: i32 = ((c as f64 - 122.1f64) / 365.25f64) as i32;
        let mut e: i32 = 365_i32 * d + d / 4_i32;
        let mut f: i32 = ((c - e) as f64 / 30.6001f64) as i32;
        (*t).d = c - e - (30.6001f64 * f as f64) as i32;
        (*t).m = f - 1_i32 - 12_i32 * (f / 14_i32);
        (*t).y = d - 4715_i32 - (7_i32 + (*t).m) / 10_i32;
        (*t).hh = ((*g).sec / 3600.0f64) as i32 % 24_i32;
        (*t).mm = ((*g).sec / 60.0f64) as i32 % 60_i32;
        (*t).sec = (*g).sec - 60.0f64 * floor((*g).sec / 60.0f64);
    }
}

pub unsafe fn xyz2llh(mut xyz_0: *const f64, mut llh: *mut f64) {
    unsafe {
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
            *llh.offset(0) = 0.0f64;
            *llh.offset(1) = 0.0f64;
            *llh.offset(2) = -a;
            return;
        }
        x = *xyz_0.offset(0);
        y = *xyz_0.offset(1);
        z = *xyz_0.offset(2);
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
        *llh.offset(0) = atan2(zdz, sqrt(rho2));
        *llh.offset(1) = atan2(y, x);
        *llh.offset(2) = nh - n;
    }
}

pub unsafe fn llh2xyz(mut llh: *const f64, mut xyz_0: *mut f64) {
    unsafe {
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
        clat = cos(*llh.offset(0));
        slat = sin(*llh.offset(0));
        clon = cos(*llh.offset(1));
        slon = sin(*llh.offset(1));
        d = e * slat;
        n = a / sqrt(1.0f64 - d * d);
        nph = n + *llh.offset(2);
        tmp = nph * clat;
        *xyz_0.offset(0) = tmp * clon;
        *xyz_0.offset(1) = tmp * slon;
        *xyz_0.offset(2) = ((1.0f64 - e2) * n + *llh.offset(2)) * slat;
    }
}

pub unsafe fn ltcmat(mut llh: *const f64, mut t: *mut [f64; 3]) {
    unsafe {
        let mut slat: f64 = 0.;
        let mut clat: f64 = 0.;
        let mut slon: f64 = 0.;
        let mut clon: f64 = 0.;
        slat = sin(*llh.offset(0));
        clat = cos(*llh.offset(0));
        slon = sin(*llh.offset(1));
        clon = cos(*llh.offset(1));
        (*t.offset(0))[0_i32 as usize] = -slat * clon;
        (*t.offset(0))[1_i32 as usize] = -slat * slon;
        (*t.offset(0))[2_i32 as usize] = clat;
        (*t.offset(1))[0_i32 as usize] = -slon;
        (*t.offset(1))[1_i32 as usize] = clon;
        (*t.offset(1))[2_i32 as usize] = 0.0f64;
        (*t.offset(2))[0_i32 as usize] = clat * clon;
        (*t.offset(2))[1_i32 as usize] = clat * slon;
        (*t.offset(2))[2_i32 as usize] = slat;
    }
}

pub unsafe fn ecef2neu(mut xyz_0: *const f64, mut t: *mut [f64; 3], mut neu: *mut f64) {
    unsafe {
        *neu.offset(0) = (*t.offset(0))[0_i32 as usize] * *xyz_0.offset(0)
            + (*t.offset(0))[1_i32 as usize] * *xyz_0.offset(1)
            + (*t.offset(0))[2_i32 as usize] * *xyz_0.offset(2);
        *neu.offset(1) = (*t.offset(1))[0_i32 as usize] * *xyz_0.offset(0)
            + (*t.offset(1))[1_i32 as usize] * *xyz_0.offset(1)
            + (*t.offset(1))[2_i32 as usize] * *xyz_0.offset(2);
        *neu.offset(2) = (*t.offset(2))[0_i32 as usize] * *xyz_0.offset(0)
            + (*t.offset(2))[1_i32 as usize] * *xyz_0.offset(1)
            + (*t.offset(2))[2_i32 as usize] * *xyz_0.offset(2);
    }
}

pub unsafe fn neu2azel(mut azel: *mut f64, mut neu: *const f64) {
    unsafe {
        let mut ne: f64 = 0.;
        *azel.offset(0) = atan2(*neu.offset(1), *neu.offset(0));
        if *azel.offset(0) < 0.0f64 {
            *azel.offset(0) += 2.0f64 * PI;
        }
        ne = sqrt(*neu.offset(0) * *neu.offset(0) + *neu.offset(1) * *neu.offset(1));
        *azel.offset(1) = atan2(*neu.offset(2), ne);
    }
}

pub unsafe fn satpos(
    mut eph: ephem_t,
    mut g: gpstime_t,
    mut pos: *mut f64,
    mut vel: *mut f64,
    mut clk: *mut f64,
) {
    unsafe {
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
        *pos.offset(0) = xpk * cok - ypk * cik * sok;
        *pos.offset(1) = xpk * sok + ypk * cik * cok;
        *pos.offset(2) = ypk * sik;
        tmp = ypkdot * cik - ypk * sik * ikdot;
        *vel.offset(0) = -eph.omgkdot * *pos.offset(1) + xpkdot * cok - tmp * sok;
        *vel.offset(1) = eph.omgkdot * *pos.offset(0) + xpkdot * sok + tmp * cok;
        *vel.offset(2) = ypk * cik * ikdot + ypkdot * sik;
        tk = g.sec - eph.toc.sec;
        if tk > 302400.0f64 {
            tk -= 604800.0f64;
        } else if tk < -302400.0f64 {
            tk += 604800.0f64;
        }
        *clk.offset(0) = eph.af0 + tk * (eph.af1 + tk * eph.af2) + relativistic - eph.tgd;
        *clk.offset(1) = eph.af1 + 2.0f64 * tk * eph.af2;
    }
}

pub unsafe fn eph2sbf(eph: ephem_t, ionoutc: ionoutc_t, mut sbf: *mut [u32; 10]) {
    unsafe {
        let mut wn: u32 = 0;
        let mut toe: u32 = 0;
        let mut toc: u32 = 0;
        let mut iode: u32 = 0;
        let mut iodc: u32 = 0;
        let mut deltan: libc::c_long = 0;
        let mut cuc: libc::c_long = 0;
        let mut cus: libc::c_long = 0;
        let mut cic: libc::c_long = 0;
        let mut cis: libc::c_long = 0;
        let mut crc: libc::c_long = 0;
        let mut crs: libc::c_long = 0;
        let mut ecc: u32 = 0;
        let mut sqrta: u32 = 0;
        let mut m0: libc::c_long = 0;
        let mut omg0: libc::c_long = 0;
        let mut inc0: libc::c_long = 0;
        let mut aop: libc::c_long = 0;
        let mut omgdot: libc::c_long = 0;
        let mut idot: libc::c_long = 0;
        let mut af0: libc::c_long = 0;
        let mut af1: libc::c_long = 0;
        let mut af2: libc::c_long = 0;
        let mut tgd: libc::c_long = 0;
        let mut svhlth: i32 = 0;
        let mut codeL2: i32 = 0;
        let mut ura: u32 = 0_u32;
        let mut dataId: u32 = 1_u32;
        let mut sbf4_page25_svId: u32 = 63_u32;
        let mut sbf5_page25_svId: u32 = 51_u32;
        let mut wna: u32 = 0;
        let mut toa: u32 = 0;
        let mut alpha0: libc::c_long = 0;
        let mut alpha1: libc::c_long = 0;
        let mut alpha2: libc::c_long = 0;
        let mut alpha3: libc::c_long = 0;
        let mut beta0: libc::c_long = 0;
        let mut beta1: libc::c_long = 0;
        let mut beta2: libc::c_long = 0;
        let mut beta3: libc::c_long = 0;
        let mut A0: libc::c_long = 0;
        let mut A1: libc::c_long = 0;
        let mut dtls: libc::c_long = 0;
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
        deltan = (eph.deltan / 1.136_868_377_216_16e-13_f64 / PI) as libc::c_long;
        cuc = (eph.cuc / 1.862645149230957e-9f64) as libc::c_long;
        cus = (eph.cus / 1.862645149230957e-9f64) as libc::c_long;
        cic = (eph.cic / 1.862645149230957e-9f64) as libc::c_long;
        cis = (eph.cis / 1.862645149230957e-9f64) as libc::c_long;
        crc = (eph.crc / 0.03125f64) as libc::c_long;
        crs = (eph.crs / 0.03125f64) as libc::c_long;
        ecc = (eph.ecc / 1.164153218269348e-10f64) as u32;
        sqrta = (eph.sqrta / 1.907_348_632_812_5e-6_f64) as u32;
        m0 = (eph.m0 / 4.656612873077393e-10f64 / PI) as libc::c_long;
        omg0 = (eph.omg0 / 4.656612873077393e-10f64 / PI) as libc::c_long;
        inc0 = (eph.inc0 / 4.656612873077393e-10f64 / PI) as libc::c_long;
        aop = (eph.aop / 4.656612873077393e-10f64 / PI) as libc::c_long;
        omgdot = (eph.omgdot / 1.136_868_377_216_16e-13_f64 / PI) as libc::c_long;
        idot = (eph.idot / 1.136_868_377_216_16e-13_f64 / PI) as libc::c_long;
        af0 = (eph.af0 / 4.656612873077393e-10f64) as libc::c_long;
        af1 = (eph.af1 / 1.136_868_377_216_16e-13_f64) as libc::c_long;
        af2 = (eph.af2 / 2.775557561562891e-17f64) as libc::c_long;
        tgd = (eph.tgd / 4.656612873077393e-10f64) as libc::c_long;
        svhlth = eph.svhlth as u32 as i32;
        codeL2 = eph.codeL2 as u32 as i32;
        wna = (eph.toe.week % 256_i32) as u32;
        toa = (eph.toe.sec / 4096.0f64) as u32;
        alpha0 = round(ionoutc.alpha0 / 9.313_225_746_154_785e-10_f64) as libc::c_long;
        alpha1 = round(ionoutc.alpha1 / 7.450_580_596_923_828e-9_f64) as libc::c_long;
        alpha2 = round(ionoutc.alpha2 / 5.960_464_477_539_063e-8_f64) as libc::c_long;
        alpha3 = round(ionoutc.alpha3 / 5.960_464_477_539_063e-8_f64) as libc::c_long;
        beta0 = round(ionoutc.beta0 / 2048.0f64) as libc::c_long;
        beta1 = round(ionoutc.beta1 / 16384.0f64) as libc::c_long;
        beta2 = round(ionoutc.beta2 / 65536.0f64) as libc::c_long;
        beta3 = round(ionoutc.beta3 / 65536.0f64) as libc::c_long;
        A0 = round(ionoutc.A0 / 9.313_225_746_154_785e-10_f64) as libc::c_long;
        A1 = round(ionoutc.A1 / 8.881_784_197_001_252e-16_f64) as libc::c_long;
        dtls = ionoutc.dtls as libc::c_long;
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
        (*sbf.offset(0))[0_i32 as usize] = 0x8b0000_u32 << 6_i32;
        (*sbf.offset(0))[1_i32 as usize] = 0x1_u32 << 8_i32;
        (*sbf.offset(0))[2_i32 as usize] = (wn & 0x3ff_u32) << 20_i32
            | (codeL2 as u32 & 0x3_u32) << 18_i32
            | (ura & 0xf_u32) << 14_i32
            | (svhlth as u32 & 0x3f_u32) << 8_i32
            | (iodc >> 8_i32 & 0x3_u32) << 6_i32;
        (*sbf.offset(0))[3_i32 as usize] = 0_u32;
        (*sbf.offset(0))[4_i32 as usize] = 0_u32;
        (*sbf.offset(0))[5_i32 as usize] = 0_u32;
        (*sbf.offset(0))[6_i32 as usize] = (tgd as u32 & 0xff_u32) << 6_i32;
        (*sbf.offset(0))[7_i32 as usize] =
            (iodc & 0xff_u32) << 22_i32 | (toc & 0xffff_u32) << 6_i32;
        (*sbf.offset(0))[8_i32 as usize] =
            (af2 as u32 & 0xff_u32) << 22_i32 | (af1 as u32 & 0xffff_u32) << 6_i32;
        (*sbf.offset(0))[9_i32 as usize] = (af0 as u32 & 0x3fffff_u32) << 8_i32;
        (*sbf.offset(1))[0_i32 as usize] = 0x8b0000_u32 << 6_i32;
        (*sbf.offset(1))[1_i32 as usize] = 0x2_u32 << 8_i32;
        (*sbf.offset(1))[2_i32 as usize] =
            (iode & 0xff_u32) << 22_i32 | (crs as u32 & 0xffff_u32) << 6_i32;
        (*sbf.offset(1))[3_i32 as usize] =
            (deltan as u32 & 0xffff_u32) << 14_i32 | ((m0 >> 24_i32) as u32 & 0xff_u32) << 6_i32;
        (*sbf.offset(1))[4_i32 as usize] = (m0 as u32 & 0xffffff_u32) << 6_i32;
        (*sbf.offset(1))[5_i32 as usize] =
            (cuc as u32 & 0xffff_u32) << 14_i32 | (ecc >> 24_i32 & 0xff_u32) << 6_i32;
        (*sbf.offset(1))[6_i32 as usize] = (ecc & 0xffffff_u32) << 6_i32;
        (*sbf.offset(1))[7_i32 as usize] =
            (cus as u32 & 0xffff_u32) << 14_i32 | (sqrta >> 24_i32 & 0xff_u32) << 6_i32;
        (*sbf.offset(1))[8_i32 as usize] = (sqrta & 0xffffff_u32) << 6_i32;
        (*sbf.offset(1))[9_i32 as usize] = (toe & 0xffff_u32) << 14_i32;
        (*sbf.offset(2))[0_i32 as usize] = 0x8b0000_u32 << 6_i32;
        (*sbf.offset(2))[1_i32 as usize] = 0x3_u32 << 8_i32;
        (*sbf.offset(2))[2_i32 as usize] =
            (cic as u32 & 0xffff_u32) << 14_i32 | ((omg0 >> 24_i32) as u32 & 0xff_u32) << 6_i32;
        (*sbf.offset(2))[3_i32 as usize] = (omg0 as u32 & 0xffffff_u32) << 6_i32;
        (*sbf.offset(2))[4_i32 as usize] =
            (cis as u32 & 0xffff_u32) << 14_i32 | ((inc0 >> 24_i32) as u32 & 0xff_u32) << 6_i32;
        (*sbf.offset(2))[5_i32 as usize] = (inc0 as u32 & 0xffffff_u32) << 6_i32;
        (*sbf.offset(2))[6_i32 as usize] =
            (crc as u32 & 0xffff_u32) << 14_i32 | ((aop >> 24_i32) as u32 & 0xff_u32) << 6_i32;
        (*sbf.offset(2))[7_i32 as usize] = (aop as u32 & 0xffffff_u32) << 6_i32;
        (*sbf.offset(2))[8_i32 as usize] = (omgdot as u32 & 0xffffff_u32) << 6_i32;
        (*sbf.offset(2))[9_i32 as usize] =
            (iode & 0xff_u32) << 22_i32 | (idot as u32 & 0x3fff_u32) << 8_i32;
        if ionoutc.vflg == 1_i32 {
            (*sbf.offset(3))[0_i32 as usize] = 0x8b0000_u32 << 6_i32;
            (*sbf.offset(3))[1_i32 as usize] = 0x4_u32 << 8_i32;
            (*sbf.offset(3))[2_i32 as usize] = dataId << 28_i32
                | sbf4_page18_svId << 22_i32
                | (alpha0 as u32 & 0xff_u32) << 14_i32
                | (alpha1 as u32 & 0xff_u32) << 6_i32;
            (*sbf.offset(3))[3_i32 as usize] = (alpha2 as u32 & 0xff_u32) << 22_i32
                | (alpha3 as u32 & 0xff_u32) << 14_i32
                | (beta0 as u32 & 0xff_u32) << 6_i32;
            (*sbf.offset(3))[4_i32 as usize] = (beta1 as u32 & 0xff_u32) << 22_i32
                | (beta2 as u32 & 0xff_u32) << 14_i32
                | (beta3 as u32 & 0xff_u32) << 6_i32;
            (*sbf.offset(3))[5_i32 as usize] = (A1 as u32 & 0xffffff_u32) << 6_i32;
            (*sbf.offset(3))[6_i32 as usize] = ((A0 >> 8_i32) as u32 & 0xffffff_u32) << 6_i32;
            (*sbf.offset(3))[7_i32 as usize] = (A0 as u32 & 0xff_u32) << 22_i32
                | (tot & 0xff_u32) << 14_i32
                | (wnt & 0xff_u32) << 6_i32;
            (*sbf.offset(3))[8_i32 as usize] = (dtls as u32 & 0xff_u32) << 22_i32
                | (wnlsf & 0xff_u32) << 14_i32
                | (dn & 0xff_u32) << 6_i32;
            (*sbf.offset(3))[9_i32 as usize] = (dtlsf & 0xff_u32) << 22_i32;
        } else {
            (*sbf.offset(3))[0_i32 as usize] = 0x8b0000_u32 << 6_i32;
            (*sbf.offset(3))[1_i32 as usize] = 0x4_u32 << 8_i32;
            (*sbf.offset(3))[2_i32 as usize] = dataId << 28_i32 | sbf4_page25_svId << 22_i32;
            (*sbf.offset(3))[3_i32 as usize] = 0_u32;
            (*sbf.offset(3))[4_i32 as usize] = 0_u32;
            (*sbf.offset(3))[5_i32 as usize] = 0_u32;
            (*sbf.offset(3))[6_i32 as usize] = 0_u32;
            (*sbf.offset(3))[7_i32 as usize] = 0_u32;
            (*sbf.offset(3))[8_i32 as usize] = 0_u32;
            (*sbf.offset(3))[9_i32 as usize] = 0_u32;
        }
        (*sbf.offset(4))[0_i32 as usize] = 0x8b0000_u32 << 6_i32;
        (*sbf.offset(4))[1_i32 as usize] = 0x5_u32 << 8_i32;
        (*sbf.offset(4))[2_i32 as usize] = dataId << 28_i32
            | sbf5_page25_svId << 22_i32
            | (toa & 0xff_u32) << 14_i32
            | (wna & 0xff_u32) << 6_i32;
        (*sbf.offset(4))[3_i32 as usize] = 0_u32;
        (*sbf.offset(4))[4_i32 as usize] = 0_u32;
        (*sbf.offset(4))[5_i32 as usize] = 0_u32;
        (*sbf.offset(4))[6_i32 as usize] = 0_u32;
        (*sbf.offset(4))[7_i32 as usize] = 0_u32;
        (*sbf.offset(4))[8_i32 as usize] = 0_u32;
        (*sbf.offset(4))[9_i32 as usize] = 0_u32;
    }
}

pub fn countBits(mut v: u32) -> u32 {
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
    c = (c >> S[0_i32 as usize] & B[0_i32 as usize]).wrapping_add(c & B[0_i32 as usize]);
    c = (c >> S[1_i32 as usize] & B[1_i32 as usize]).wrapping_add(c & B[1_i32 as usize]);
    c = (c >> S[2_i32 as usize] & B[2_i32 as usize]).wrapping_add(c & B[2_i32 as usize]);
    c = (c >> S[3_i32 as usize] & B[3_i32 as usize]).wrapping_add(c & B[3_i32 as usize]);
    c = (c >> S[4_i32 as usize] & B[4_i32 as usize]).wrapping_add(c & B[4_i32 as usize]);
    c
}

pub fn computeChecksum(mut source: u32, mut nib: i32) -> u32 {
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
            .wrapping_add(countBits(bmask[4_i32 as usize] & d))
            .wrapping_rem(2_i32 as u32)
            != 0
        {
            d ^= 0x1_u32 << 6_i32;
        }
        if D29
            .wrapping_add(countBits(bmask[5_i32 as usize] & d))
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
        .wrapping_add(countBits(bmask[0_i32 as usize] & d))
        .wrapping_rem(2_i32 as u32)
        << 5_i32;
    D |= D30
        .wrapping_add(countBits(bmask[1_i32 as usize] & d))
        .wrapping_rem(2_i32 as u32)
        << 4_i32;
    D |= D29
        .wrapping_add(countBits(bmask[2_i32 as usize] & d))
        .wrapping_rem(2_i32 as u32)
        << 3_i32;
    D |= D30
        .wrapping_add(countBits(bmask[3_i32 as usize] & d))
        .wrapping_rem(2_i32 as u32)
        << 2_i32;
    D |= D30
        .wrapping_add(countBits(bmask[4_i32 as usize] & d))
        .wrapping_rem(2_i32 as u32)
        << 1_i32;
    D |= D29
        .wrapping_add(countBits(bmask[5_i32 as usize] & d))
        .wrapping_rem(2_i32 as u32);
    D &= 0x3fffffff_u32;
    D
}

pub fn subGpsTime(mut g1: gpstime_t, mut g0: gpstime_t) -> f64 {
    let mut dt: f64 = 0.;
    dt = g1.sec - g0.sec;
    dt += (g1.week - g0.week) as f64 * 604800.0f64;
    dt
}

pub fn incGpsTime(mut g0: gpstime_t, mut dt: f64) -> gpstime_t {
    unsafe {
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
}

pub unsafe fn ionosphericDelay(
    mut ionoutc: *const ionoutc_t,
    mut g: gpstime_t,
    mut llh: *mut f64,
    mut azel: *mut f64,
) -> f64 {
    unsafe {
        let mut iono_delay: f64 = 0.0f64;
        let mut E: f64 = 0.;
        let mut phi_u: f64 = 0.;
        let mut lam_u: f64 = 0.;
        let mut F: f64 = 0.;
        if (*ionoutc).enable == 0_i32 {
            return 0.0f64;
        }
        E = *azel.offset(1) / PI;
        phi_u = *llh.offset(0) / PI;
        lam_u = *llh.offset(1) / PI;
        F = 1.0f64 + 16.0f64 * pow(0.53f64 - E, 3.0f64);
        if (*ionoutc).vflg == 0_i32 {
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
            phi_i = phi_u + psi * cos(*azel.offset(0));
            phi_i = phi_i.clamp(-0.416f64, 0.416f64);
            lam_i = lam_u + psi * sin(*azel.offset(0)) / cos(phi_i * PI);
            phi_m = phi_i + 0.064f64 * cos((lam_i - 1.617f64) * PI);
            phi_m2 = phi_m * phi_m;
            phi_m3 = phi_m2 * phi_m;
            AMP = (*ionoutc).alpha0
                + (*ionoutc).alpha1 * phi_m
                + (*ionoutc).alpha2 * phi_m2
                + (*ionoutc).alpha3 * phi_m3;
            if AMP < 0.0f64 {
                AMP = 0.0f64;
            }
            PER = (*ionoutc).beta0
                + (*ionoutc).beta1 * phi_m
                + (*ionoutc).beta2 * phi_m2
                + (*ionoutc).beta3 * phi_m3;
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
}

pub unsafe fn computeRange(
    mut rho: *mut range_t,
    mut eph: ephem_t,
    mut ionoutc: *mut ionoutc_t,
    mut g: gpstime_t,
    mut xyz_0: *mut f64,
) {
    unsafe {
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
        satpos(eph, g, pos.as_mut_ptr(), vel.as_mut_ptr(), clk.as_mut_ptr());
        subVect(los.as_mut_ptr(), pos.as_mut_ptr(), xyz_0 as *const f64);
        tau = normVect(los.as_mut_ptr()) / 2.99792458e8f64;
        pos[0_i32 as usize] -= vel[0_i32 as usize] * tau;
        pos[1_i32 as usize] -= vel[1_i32 as usize] * tau;
        pos[2_i32 as usize] -= vel[2_i32 as usize] * tau;
        xrot = pos[0_i32 as usize] + pos[1_i32 as usize] * 7.2921151467e-5f64 * tau;
        yrot = pos[1_i32 as usize] - pos[0_i32 as usize] * 7.2921151467e-5f64 * tau;
        pos[0_i32 as usize] = xrot;
        pos[1_i32 as usize] = yrot;
        subVect(los.as_mut_ptr(), pos.as_mut_ptr(), xyz_0 as *const f64);
        range = normVect(los.as_mut_ptr());
        (*rho).d = range;
        (*rho).range = range - 2.99792458e8f64 * clk[0_i32 as usize];
        rate = dotProd(vel.as_mut_ptr(), los.as_mut_ptr()) / range;
        (*rho).rate = rate;
        (*rho).g = g;
        xyz2llh(xyz_0 as *const f64, llh.as_mut_ptr());
        ltcmat(llh.as_mut_ptr(), tmat.as_mut_ptr());
        ecef2neu(los.as_mut_ptr(), tmat.as_mut_ptr(), neu.as_mut_ptr());
        neu2azel(((*rho).azel).as_mut_ptr(), neu.as_mut_ptr());
        (*rho).iono_delay =
            ionosphericDelay(ionoutc, g, llh.as_mut_ptr(), ((*rho).azel).as_mut_ptr());
        (*rho).range += (*rho).iono_delay;
    }
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
            (*xyz_0.offset(numd as isize))[0_i32 as usize] = x;
            (*xyz_0.offset(numd as isize))[1_i32 as usize] = y;
            (*xyz_0.offset(numd as isize))[2_i32 as usize] = z;
            numd += 1;
        }
        fclose(fp);
        numd
    }
}

pub unsafe fn readUserMotionLLH(
    mut xyz_0: *mut [f64; 3],
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
            if llh[0_i32 as usize] > 90.0f64
                || llh[0_i32 as usize] < -90.0f64
                || llh[1_i32 as usize] > 180.0f64
                || llh[1_i32 as usize] < -180.0f64
            {
                eprintln!(
                    "ERROR: Invalid file format (time[s], latitude[deg], longitude[deg], height [m].\n"
                );
                numd = 0_i32;
                break;
            } else {
                llh[0_i32 as usize] /= 57.2957795131f64;
                llh[1_i32 as usize] /= 57.2957795131f64;
                llh2xyz(
                    llh.as_mut_ptr(),
                    (*xyz_0.offset(numd as isize)).as_mut_ptr(),
                );
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
                sbfwrd = (*chan).sbf[4_i32 as usize][iwrd as usize] as u32;
                if iwrd == 1_i32 {
                    sbfwrd = (sbfwrd as u32 | (tow & 0x1ffff_u32) << 13_i32) as u32;
                }
                sbfwrd = (sbfwrd as u32 | prevwrd << 30_i32 & 0xc0000000_u32) as u32;
                nib = if iwrd == 1_i32 || iwrd == 9_i32 {
                    1_i32
                } else {
                    0_i32
                };
                (*chan).dwrd[iwrd as usize] = computeChecksum(sbfwrd as u32, nib);
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
                sbfwrd = (*chan).sbf[isbf as usize][iwrd as usize] as u32;
                if isbf == 0_i32 && iwrd == 2_i32 {
                    sbfwrd = (sbfwrd as u32 | (wn & 0x3ff_u32) << 20_i32) as u32;
                }
                if iwrd == 1_i32 {
                    sbfwrd = (sbfwrd as u32 | (tow & 0x1ffff_u32) << 13_i32) as u32;
                }
                sbfwrd = (sbfwrd as u32 | prevwrd << 30_i32 & 0xc0000000_u32) as u32;
                nib = if iwrd == 1_i32 || iwrd == 9_i32 {
                    1_i32
                } else {
                    0_i32
                };
                (*chan).dwrd[((isbf + 1_i32) * 10_i32 + iwrd) as usize] =
                    computeChecksum(sbfwrd as u32, nib);
                prevwrd = (*chan).dwrd[((isbf + 1_i32) * 10_i32 + iwrd) as usize];
                iwrd += 1;
            }
            isbf += 1;
        }
        1_i32
    }
}

pub unsafe fn checkSatVisibility(
    mut eph: ephem_t,
    mut g: gpstime_t,
    mut xyz_0: *mut f64,
    mut elvMask: f64,
    mut azel: *mut f64,
) -> i32 {
    unsafe {
        let mut llh: [f64; 3] = [0.; 3];
        let mut neu: [f64; 3] = [0.; 3];
        let mut pos: [f64; 3] = [0.; 3];
        let mut vel: [f64; 3] = [0.; 3];
        let mut clk: [f64; 3] = [0.; 3];
        let mut los: [f64; 3] = [0.; 3];
        let mut tmat: [[f64; 3]; 3] = [[0.; 3]; 3];
        if eph.vflg != 1_i32 {
            return -1_i32;
        }
        xyz2llh(xyz_0, llh.as_mut_ptr());
        ltcmat(llh.as_mut_ptr(), tmat.as_mut_ptr());
        satpos(eph, g, pos.as_mut_ptr(), vel.as_mut_ptr(), clk.as_mut_ptr());
        subVect(los.as_mut_ptr(), pos.as_mut_ptr(), xyz_0);
        ecef2neu(los.as_mut_ptr(), tmat.as_mut_ptr(), neu.as_mut_ptr());
        neu2azel(azel, neu.as_mut_ptr());
        if *azel.offset(1) * 57.2957795131f64 > elvMask {
            return 1_i32;
        }
        0_i32
    }
}

pub unsafe fn allocateChannel(
    mut chan: *mut channel_t,
    mut eph: *mut ephem_t,
    mut ionoutc: ionoutc_t,
    mut grx: gpstime_t,
    mut xyz_0: *mut f64,
    mut _elvMask: f64,
) -> i32 {
    unsafe {
        let mut nsat: i32 = 0_i32;
        let mut i: i32 = 0;
        let mut sv: i32 = 0;
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
        sv = 0_i32;
        while sv < 32_i32 {
            if checkSatVisibility(
                *eph.offset(sv as isize),
                grx,
                xyz_0,
                0.0f64,
                azel.as_mut_ptr(),
            ) == 1_i32
            {
                nsat += 1;
                if allocatedSat[sv as usize] == -1_i32 {
                    i = 0_i32;
                    while i < 16_i32 {
                        if (*chan.offset(i as isize)).prn == 0_i32 {
                            (*chan.offset(i as isize)).prn = sv + 1_i32;
                            (*chan.offset(i as isize)).azel[0_i32 as usize] = azel[0_i32 as usize];
                            (*chan.offset(i as isize)).azel[1_i32 as usize] = azel[1_i32 as usize];
                            codegen(
                                ((*chan.offset(i as isize)).ca).as_mut_ptr(),
                                (*chan.offset(i as isize)).prn,
                            );
                            eph2sbf(
                                *eph.offset(sv as isize),
                                ionoutc,
                                ((*chan.offset(i as isize)).sbf).as_mut_ptr(),
                            );
                            generateNavMsg(grx, &mut *chan.offset(i as isize), 1_i32);
                            computeRange(
                                &mut rho,
                                *eph.offset(sv as isize),
                                &mut ionoutc,
                                grx,
                                xyz_0,
                            );
                            (*chan.offset(i as isize)).rho0 = rho;
                            r_xyz = rho.range;
                            computeRange(
                                &mut rho,
                                *eph.offset(sv as isize),
                                &mut ionoutc,
                                grx,
                                ref_0.as_mut_ptr(),
                            );
                            r_ref = rho.range;
                            phase_ini = 0.0f64;
                            phase_ini -= floor(phase_ini);
                            (*chan.offset(i as isize)).carr_phase =
                                (512.0f64 * 65536.0f64 * phase_ini) as u32;
                            break;
                        } else {
                            i += 1;
                        }
                    }
                    if i < 16_i32 {
                        allocatedSat[sv as usize] = i;
                    }
                }
            } else if allocatedSat[sv as usize] >= 0_i32 {
                (*chan.offset(allocatedSat[sv as usize] as isize)).prn = 0_i32;
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
unsafe fn main_0(mut argc: i32, mut argv: *mut *mut libc::c_char) -> i32 {
    unsafe {
        let mut tstart: clock_t = 0;
        let mut tend: clock_t = 0;
        let mut fp: *mut FILE = std::ptr::null_mut::<FILE>();
        let mut sv: i32 = 0;
        let mut neph: i32 = 0;
        let mut ieph: i32 = 0;
        let mut eph: [[ephem_t; 32]; 15] = [[ephem_t {
            vflg: 0,
            t: datetime_t {
                y: 0,
                m: 0,
                d: 0,
                hh: 0,
                mm: 0,
                sec: 0.,
            },
            toc: gpstime_t { week: 0, sec: 0. },
            toe: gpstime_t { week: 0, sec: 0. },
            iodc: 0,
            iode: 0,
            deltan: 0.,
            cuc: 0.,
            cus: 0.,
            cic: 0.,
            cis: 0.,
            crc: 0.,
            crs: 0.,
            ecc: 0.,
            sqrta: 0.,
            m0: 0.,
            omg0: 0.,
            inc0: 0.,
            aop: 0.,
            omgdot: 0.,
            idot: 0.,
            af0: 0.,
            af1: 0.,
            af2: 0.,
            tgd: 0.,
            svhlth: 0,
            codeL2: 0,
            n: 0.,
            sq1e2: 0.,
            A: 0.,
            omgkdot: 0.,
        }; 32]; 15];
        let mut g0: gpstime_t = gpstime_t { week: 0, sec: 0. };
        let mut llh: [f64; 3] = [0.; 3];
        let mut i: i32 = 0;
        let mut chan: [channel_t; 16] = [channel_t {
            prn: 0,
            ca: [0; 1023],
            f_carr: 0.,
            f_code: 0.,
            carr_phase: 0,
            carr_phasestep: 0,
            code_phase: 0.,
            g0: gpstime_t { week: 0, sec: 0. },
            sbf: [[0; 10]; 5],
            dwrd: [0; 60],
            iword: 0,
            ibit: 0,
            icode: 0,
            dataBit: 0,
            codeCA: 0,
            azel: [0.; 2],
            rho0: range_t {
                g: gpstime_t { week: 0, sec: 0. },
                range: 0.,
                rate: 0.,
                d: 0.,
                azel: [0.; 2],
                iono_delay: 0.,
            },
        }; 16];
        let mut elvmask: f64 = 0.0f64;
        let mut ip: i32 = 0;
        let mut qp: i32 = 0;
        let mut iTable: i32 = 0;
        let mut iq_buff: *mut libc::c_short = std::ptr::null_mut::<libc::c_short>();
        let mut iq8_buff: *mut libc::c_schar = std::ptr::null_mut::<libc::c_schar>();
        let mut grx: gpstime_t = gpstime_t { week: 0, sec: 0. };
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
        let mut t0: datetime_t = datetime_t {
            y: 0,
            m: 0,
            d: 0,
            hh: 0,
            mm: 0,
            sec: 0.,
        };
        let mut tmin: datetime_t = datetime_t {
            y: 0,
            m: 0,
            d: 0,
            hh: 0,
            mm: 0,
            sec: 0.,
        };
        let mut tmax: datetime_t = datetime_t {
            y: 0,
            m: 0,
            d: 0,
            hh: 0,
            mm: 0,
            sec: 0.,
        };
        let mut gmin: gpstime_t = gpstime_t { week: 0, sec: 0. };
        let mut gmax: gpstime_t = gpstime_t { week: 0, sec: 0. };
        let mut dt: f64 = 0.;
        let mut igrx: i32 = 0;
        let mut duration: f64 = 0.;
        let mut iduration: i32 = 0;
        let mut verb: i32 = 0;
        let mut timeoverwrite: i32 = 0_i32;
        let mut ionoutc: ionoutc_t = ionoutc_t {
            enable: 0,
            vflg: 0,
            alpha0: 0.,
            alpha1: 0.,
            alpha2: 0.,
            alpha3: 0.,
            beta0: 0.,
            beta1: 0.,
            beta2: 0.,
            beta3: 0.,
            A0: 0.,
            A1: 0.,
            dtls: 0,
            tot: 0,
            wnt: 0,
            dtlsf: 0,
            dn: 0,
            wnlsf: 0,
            leapen: 0,
        };
        let mut path_loss_enable: i32 = 1_i32;
        navfile[0_i32 as usize] = 0_i32 as libc::c_char;
        umfile[0_i32 as usize] = 0_i32 as libc::c_char;
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
                    llh[0_i32 as usize] /= 57.2957795131f64;
                    llh[1_i32 as usize] /= 57.2957795131f64;
                    llh2xyz(llh.as_mut_ptr(), (xyz[0_i32 as usize]).as_mut_ptr());
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
        if navfile[0_i32 as usize] as i32 == 0_i32 {
            eprintln!("ERROR: GPS ephemeris file is not specified.\n");
            exit(1_i32);
        }
        if umfile[0_i32 as usize] as i32 == 0_i32 && staticLocationMode == 0 {
            staticLocationMode = 1_i32;
            llh[0_i32 as usize] = 35.681298f64 / 57.2957795131f64;
            llh[1_i32 as usize] = 139.766247f64 / 57.2957795131f64;
            llh[2_i32 as usize] = 10.0f64;
        }
        if duration < 0.0f64
            || duration > USER_MOTION_SIZE as i32 as f64 / 10.0f64 && staticLocationMode == 0
            || duration > 86400_f64 && staticLocationMode != 0
        {
            eprintln!("ERROR: Invalid duration.\n\0");
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
                numd = readUserMotionLLH(xyz.as_mut_ptr(), umfile.as_mut_ptr());
            } else {
                numd = readUserMotion(xyz.as_mut_ptr(), umfile.as_mut_ptr());
            }
            if numd == -1_i32 {
                eprintln!("ERROR: Failed to open user motion / NMEA GGA file.\n\0");
                exit(1_i32);
            } else if numd == 0_i32 {
                eprintln!("ERROR: Failed to read user motion / NMEA GGA data.\n\0");
                exit(1_i32);
            }
            if numd > iduration {
                numd = iduration;
            }
            xyz2llh((xyz[0_i32 as usize]).as_mut_ptr(), llh.as_mut_ptr());
        } else {
            eprintln!("Using static location mode.\n\0");
            numd = iduration;
            llh2xyz(llh.as_mut_ptr(), (xyz[0_i32 as usize]).as_mut_ptr());
        }

        eprintln!(
            "xyz = {}, {}, {}\n\0",
            xyz[0_i32 as usize][0_i32 as usize],
            xyz[0_i32 as usize][1_i32 as usize],
            xyz[0_i32 as usize][2_i32 as usize],
        );

        eprintln!(
            "llh = {}, {}, {}\n\0",
            llh[0_i32 as usize] * 57.2957795131f64,
            llh[1_i32 as usize] * 57.2957795131f64,
            llh[2_i32 as usize],
        );
        neph = readRinexNavAll(eph.as_mut_ptr(), &mut ionoutc, navfile.as_mut_ptr());
        if neph == 0_i32 {
            eprintln!("ERROR: No ephemeris available.\n\0",);
            exit(1_i32);
        } else if neph == -1_i32 {
            eprintln!("ERROR: ephemeris file not found.\n\0");
            exit(1_i32);
        }
        if verb == 1_i32 && ionoutc.vflg == 1_i32 {
            eprintln!(
                "  {:12.3e} {:12.3e} {:12.3e} {:12.3e}\n\0",
                ionoutc.alpha0, ionoutc.alpha1, ionoutc.alpha2, ionoutc.alpha3,
            );

            eprintln!(
                "  {:12.3e} {:12.3e} {:12.3e} {:12.3e}\n\0",
                ionoutc.beta0, ionoutc.beta1, ionoutc.beta2, ionoutc.beta3,
            );

            eprintln!(
                "   {:19.11e} {:19.11e}  {:9} {:9}\n\0",
                ionoutc.A0, ionoutc.A1, ionoutc.tot, ionoutc.wnt,
            );

            eprintln!("{:6}\n\0", ionoutc.dtls,);
        }
        sv = 0_i32;
        while sv < 32_i32 {
            if eph[0_i32 as usize][sv as usize].vflg == 1_i32 {
                gmin = eph[0_i32 as usize][sv as usize].toc;
                tmin = eph[0_i32 as usize][sv as usize].t;
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
                let mut gtmp: gpstime_t = gpstime_t { week: 0, sec: 0. };
                let mut ttmp: datetime_t = datetime_t {
                    y: 0,
                    m: 0,
                    d: 0,
                    hh: 0,
                    mm: 0,
                    sec: 0.,
                };
                let mut dsec: f64 = 0.;
                gtmp.week = g0.week;
                gtmp.sec = (g0.sec as i32 / 7200_i32) as f64 * 7200.0f64;
                dsec = subGpsTime(gtmp, gmin);
                ionoutc.wnt = gtmp.week;
                ionoutc.tot = gtmp.sec as i32;
                sv = 0_i32;
                while sv < 32_i32 {
                    i = 0_i32;
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
                eprintln!("ERROR: Invalid start time.\n\0");
                eprintln!(
                    "tmin = {:4}/{:02}/{:02},{:02}:{:02}:{:0>2.0} ({}:{:.0})\n\0",
                    tmin.y, tmin.m, tmin.d, tmin.hh, tmin.mm, tmin.sec, gmin.week, gmin.sec,
                );
                eprintln!(
                    "tmax = {:4}/{:02}/{:02},{:02}:{:02}:{:0>2.0} ({}:{:.0})\n\0",
                    tmax.y, tmax.m, tmax.d, tmax.hh, tmax.mm, tmax.sec, gmax.week, gmax.sec,
                );
                exit(1_i32);
            }
        } else {
            g0 = gmin;
            t0 = tmin;
        }

        eprintln!(
            "Start time = {:4}/{:02}/{:02},{:02}:{:02}:{:0>2.0} ({}:{:.0})\n\0",
            t0.y, t0.m, t0.d, t0.hh, t0.mm, t0.sec, g0.week, g0.sec,
        );

        eprintln!("Duration = {:.1} [sec]\n\0", numd as f64 / 10.0f64);
        ieph = -1_i32;
        i = 0_i32;
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
            eprintln!("ERROR: No current set of ephemerides has been found.\n\0",);
            exit(1_i32);
        }
        iq_buff = calloc((2_i32 * iq_buff_size) as u32, 2_i32 as u32) as *mut libc::c_short;
        if iq_buff.is_null() {
            eprintln!("ERROR: Failed to allocate 16-bit I/Q buffer.\n\0");
            exit(1_i32);
        }
        if data_format == 8_i32 {
            iq8_buff = calloc((2_i32 * iq_buff_size) as u32, 1_i32 as u32) as *mut libc::c_schar;
            if iq8_buff.is_null() {
                eprintln!("ERROR: Failed to allocate 8-bit I/Q buffer.\n\0");
                exit(1_i32);
            }
        } else if data_format == 1_i32 {
            iq8_buff = calloc((iq_buff_size / 4_i32) as u32, 1_i32 as u32) as *mut libc::c_schar;
            if iq8_buff.is_null() {
                eprintln!("ERROR: Failed to allocate compressed 1-bit I/Q buffer.\n\0");
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
                eprintln!("ERROR: Failed to open output file.\n\0");
                exit(1_i32);
            }
        } else {
            // todo: temporarily disable
            // fp = stdout;
        }
        i = 0_i32;
        while i < 16_i32 {
            chan[i as usize].prn = 0_i32;
            i += 1;
        }
        sv = 0_i32;
        while sv < 32_i32 {
            allocatedSat[sv as usize] = -1_i32;
            sv += 1;
        }
        grx = incGpsTime(g0, 0.0f64);
        allocateChannel(
            chan.as_mut_ptr(),
            (eph[ieph as usize]).as_mut_ptr(),
            ionoutc,
            grx,
            (xyz[0_i32 as usize]).as_mut_ptr(),
            elvmask,
        );
        i = 0_i32;
        while i < 16_i32 {
            if chan[i as usize].prn > 0_i32 {
                eprintln!(
                    "{:02} {:6.1} {:5.1} {:11.1} {:5.1}\n\0",
                    chan[i as usize].prn,
                    chan[i as usize].azel[0_i32 as usize] * 57.2957795131f64,
                    chan[i as usize].azel[1_i32 as usize] * 57.2957795131f64,
                    chan[i as usize].rho0.d,
                    chan[i as usize].rho0.iono_delay,
                );
            }
            i += 1;
        }
        i = 0_i32;
        while i < 37_i32 {
            ant_pat[i as usize] = pow(10.0f64, -ant_pat_db[i as usize] / 20.0f64);
            i += 1;
        }
        tstart = clock();
        grx = incGpsTime(grx, 0.1f64);
        iumd = 1_i32;
        while iumd < numd {
            i = 0_i32;
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
                            eph[ieph as usize][sv as usize],
                            &mut ionoutc,
                            grx,
                            (xyz[iumd as usize]).as_mut_ptr(),
                        );
                    } else {
                        computeRange(
                            &mut rho,
                            eph[ieph as usize][sv as usize],
                            &mut ionoutc,
                            grx,
                            (xyz[0_i32 as usize]).as_mut_ptr(),
                        );
                    }
                    chan[i as usize].azel[0_i32 as usize] = rho.azel[0_i32 as usize];
                    chan[i as usize].azel[1_i32 as usize] = rho.azel[1_i32 as usize];
                    computeCodePhase(&mut *chan.as_mut_ptr().offset(i as isize), rho, 0.1f64);
                    chan[i as usize].carr_phasestep =
                        round(512.0f64 * 65536.0f64 * chan[i as usize].f_carr * delt) as i32;
                    path_loss = 20200000.0f64 / rho.d;
                    ibs = ((90.0f64 - rho.azel[1_i32 as usize] * 57.2957795131f64) / 5.0f64) as i32;
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
                i = 0_i32;
                while i < 16_i32 {
                    if chan[i as usize].prn > 0_i32 {
                        iTable = (chan[i as usize].carr_phase >> 16_i32 & 0x1ff_i32 as u32) as i32;
                        ip = chan[i as usize].dataBit
                            * chan[i as usize].codeCA
                            * cosTable512[iTable as usize]
                            * gain[i as usize];
                        qp = chan[i as usize].dataBit
                            * chan[i as usize].codeCA
                            * sinTable512[iTable as usize]
                            * gain[i as usize];
                        i_acc += ip;
                        q_acc += qp;
                        chan[i as usize].code_phase += chan[i as usize].f_code * delt;
                        if chan[i as usize].code_phase >= 1023_f64 {
                            chan[i as usize].code_phase -= 1023_f64;
                            chan[i as usize].icode += 1;
                            if chan[i as usize].icode >= 20_i32 {
                                chan[i as usize].icode = 0_i32;
                                chan[i as usize].ibit += 1;
                                if chan[i as usize].ibit >= 30_i32 {
                                    chan[i as usize].ibit = 0_i32;
                                    chan[i as usize].iword += 1;
                                }
                                chan[i as usize].dataBit =
                                    (chan[i as usize].dwrd[chan[i as usize].iword as usize]
                                        >> (29_i32 - chan[i as usize].ibit)
                                        & 0x1_u32) as i32
                                        * 2_i32
                                        - 1_i32;
                            }
                        }
                        chan[i as usize].codeCA = chan[i as usize].ca
                            [chan[i as usize].code_phase as i32 as usize]
                            * 2_i32
                            - 1_i32;
                        chan[i as usize].carr_phase = (chan[i as usize].carr_phase)
                            .wrapping_add(chan[i as usize].carr_phasestep as u32);
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
                i = 0_i32;
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
                            i = 0_i32;
                            while i < 16_i32 {
                                if chan[i as usize].prn != 0_i32 {
                                    eph2sbf(
                                        eph[ieph as usize][(chan[i as usize].prn - 1_i32) as usize],
                                        ionoutc,
                                        (chan[i as usize].sbf).as_mut_ptr(),
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
                        chan.as_mut_ptr(),
                        (eph[ieph as usize]).as_mut_ptr(),
                        ionoutc,
                        grx,
                        (xyz[iumd as usize]).as_mut_ptr(),
                        elvmask,
                    );
                } else {
                    allocateChannel(
                        chan.as_mut_ptr(),
                        (eph[ieph as usize]).as_mut_ptr(),
                        ionoutc,
                        grx,
                        (xyz[0_i32 as usize]).as_mut_ptr(),
                        elvmask,
                    );
                }
                if verb == 1_i32 {
                    eprintln!();
                    i = 0_i32;
                    while i < 16_i32 {
                        if chan[i as usize].prn > 0_i32 {
                            eprintln!(
                                "{:02} {:6.1} {:5.1} {:11.1} {:5.1}\n\0",
                                chan[i as usize].prn,
                                chan[i as usize].azel[0_i32 as usize] * 57.2957795131f64,
                                chan[i as usize].azel[1_i32 as usize] * 57.2957795131f64,
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

        eprintln!("\nDone!\n\0");
        free(iq_buff as *mut libc::c_void);
        fclose(fp);

        eprintln!(
            "Process time = {:.1} [sec]\n\0",
            (tend - tstart) as f64 / 1000000_i32 as __clock_t as f64,
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
    unsafe { ::std::process::exit(main_0((args.len() - 1) as i32, args.as_mut_ptr())) }
}
