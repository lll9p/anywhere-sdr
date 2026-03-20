use std::path::Path;

use constants::{
    GM_EARTH, OMEGA_EARTH, PI, POW2_M5, POW2_M19, POW2_M29, POW2_M31, POW2_M33,
    POW2_M43, POW2_M55,
};
use gps::{BroadcastEphemeris, DateTime, Error, GpsTime};

use super::{EphemerisDiagnostics, RecoveredSubframe};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedEphemeris {
    pub prn: usize,
    pub week: u16,
    pub code_l2: u8,
    pub sv_health: u8,
    pub iodc: u16,
    pub iode: u8,
    pub toc: u16,
    pub toe: u16,
    pub tgd: i8,
    pub af2: i8,
    pub af1: i16,
    pub af0: i32,
    pub crs: i16,
    pub deltan: i16,
    pub m0: i32,
    pub cuc: i16,
    pub ecc: u32,
    pub cus: i16,
    pub sqrta: u32,
    pub cic: i16,
    pub omg0: i32,
    pub cis: i16,
    pub inc0: i32,
    pub crc: i16,
    pub aop: i32,
    pub omgdot: i32,
    pub idot: i16,
}

pub type QuantizedEphemeris = DecodedEphemeris;

pub fn select_rinex_record(
    navigation_path: &Path, prn: usize, reference_time: &GpsTime,
) -> Result<rinex::ephemeris::Ephemeris, Error> {
    let rinex = rinex::Rinex::read_file(&navigation_path)?;
    let mut best_match: Option<(&rinex::ephemeris::Ephemeris, GpsTime)> = None;

    for record in rinex.ephemerides.iter().filter(|record| record.prn == prn) {
        let record_time = record_time_of_clock(record)?;
        let should_replace =
            best_match.as_ref().is_none_or(|(_, best_time)| {
                record_time
                    .diff_secs(reference_time)
                    .abs()
                    .total_cmp(&best_time.diff_secs(reference_time).abs())
                    .is_lt()
            });
        if should_replace {
            best_match = Some((record, record_time));
        }
    }

    best_match
        .map(|(record, _)| record.clone())
        .ok_or(Error::NoEphemeris)
}

pub fn quantize_rinex_record(
    record: &rinex::ephemeris::Ephemeris, reference_week: i32,
) -> Result<QuantizedEphemeris, Error> {
    let toc_time =
        GpsTime::from(&DateTime::from(record.time_of_clock.in_tz("UTC")?));
    let sv_health = normalize_sv_health(record.orbit6.sv_health as i32);
    Ok(QuantizedEphemeris {
        prn: record.prn,
        week: reference_week.rem_euclid(1024) as u16,
        code_l2: record.orbit5.code_l2 as u8,
        sv_health: sv_health as u8,
        iodc: record.orbit6.iodc as u16,
        iode: record.orbit1.iode as u8,
        toc: (toc_time.sec / 16.0) as u16,
        toe: (record.orbit3.toe / 16.0) as u16,
        tgd: (record.orbit6.tgd / POW2_M31) as i8,
        af2: (record.sv_clock.drift_rate / POW2_M55) as i8,
        af1: (record.sv_clock.drift / POW2_M43) as i16,
        af0: (record.sv_clock.bias / POW2_M31) as i32,
        crs: (record.orbit1.crs / POW2_M5) as i16,
        deltan: (record.orbit1.delta_n / POW2_M43 / PI) as i16,
        m0: (record.orbit1.m0 / POW2_M31 / PI) as i32,
        cuc: (record.orbit2.cuc / POW2_M29) as i16,
        ecc: (record.orbit2.ecc / POW2_M33) as u32,
        cus: (record.orbit2.cus / POW2_M29) as i16,
        sqrta: (record.orbit2.sqrta / POW2_M19) as u32,
        cic: (record.orbit3.cic / POW2_M29) as i16,
        omg0: (record.orbit3.omega / POW2_M31 / PI) as i32,
        cis: (record.orbit3.cis / POW2_M29) as i16,
        inc0: (record.orbit4.i0 / POW2_M31 / PI) as i32,
        crc: (record.orbit4.crc / POW2_M5) as i16,
        aop: (record.orbit4.omega / POW2_M31 / PI) as i32,
        omgdot: (record.orbit4.omega_dot / POW2_M43 / PI) as i32,
        idot: (record.orbit5.idot / POW2_M43 / PI) as i16,
    })
}

pub fn decode_ephemeris(
    prn: usize, subframes: &[RecoveredSubframe], _reference_week: i32,
) -> Result<DecodedEphemeris, Error> {
    let subframe_1 = subframes
        .iter()
        .find(|subframe| subframe.subframe_id == 1)
        .ok_or_else(|| Error::msg(format!("PRN {prn} missing subframe 1")))?;
    let subframe_2 = subframes
        .iter()
        .find(|subframe| subframe.subframe_id == 2)
        .ok_or_else(|| Error::msg(format!("PRN {prn} missing subframe 2")))?;
    let subframe_3 = subframes
        .iter()
        .find(|subframe| subframe.subframe_id == 3)
        .ok_or_else(|| Error::msg(format!("PRN {prn} missing subframe 3")))?;

    let sf1 = &subframe_1.data_words;
    let sf2 = &subframe_2.data_words;
    let sf3 = &subframe_3.data_words;

    let week = ((sf1[2] >> 14) & 0x03ff) as u16;
    let code_l2 = ((sf1[2] >> 12) & 0x03) as u8;
    let sv_health = ((sf1[2] >> 2) & 0x3f) as u8;
    let iodc = (((sf1[2] & 0x3) as u16) << 8) | ((sf1[7] >> 16) as u16);
    let tgd = sign_extend(sf1[6] & 0xff, 8) as i8;
    let toc = (sf1[7] & 0xffff) as u16;
    let af2 = sign_extend((sf1[8] >> 16) & 0xff, 8) as i8;
    let af1 = sign_extend(sf1[8] & 0xffff, 16) as i16;
    let af0 = sign_extend((sf1[9] >> 2) & 0x003f_ffff, 22);

    let iode = ((sf2[2] >> 16) & 0xff) as u8;
    let crs = sign_extend(sf2[2] & 0xffff, 16) as i16;
    let deltan = sign_extend((sf2[3] >> 8) & 0xffff, 16) as i16;
    let m0 = combine_signed_32(sf2[3] & 0xff, sf2[4]);
    let cuc = sign_extend((sf2[5] >> 8) & 0xffff, 16) as i16;
    let ecc = ((sf2[5] & 0xff) << 24) | sf2[6];
    let cus = sign_extend((sf2[7] >> 8) & 0xffff, 16) as i16;
    let sqrta = ((sf2[7] & 0xff) << 24) | sf2[8];
    let toe = ((sf2[9] >> 8) & 0xffff) as u16;

    let cic = sign_extend((sf3[2] >> 8) & 0xffff, 16) as i16;
    let omg0 = combine_signed_32(sf3[2] & 0xff, sf3[3]);
    let cis = sign_extend((sf3[4] >> 8) & 0xffff, 16) as i16;
    let inc0 = combine_signed_32(sf3[4] & 0xff, sf3[5]);
    let crc = sign_extend((sf3[6] >> 8) & 0xffff, 16) as i16;
    let aop = combine_signed_32(sf3[6] & 0xff, sf3[7]);
    let omgdot = sign_extend(sf3[8], 24);
    let idot = sign_extend((sf3[9] >> 2) & 0x3fff, 14) as i16;
    Ok(DecodedEphemeris {
        prn,
        week,
        code_l2,
        sv_health,
        iodc,
        iode,
        toc,
        toe,
        tgd,
        af2,
        af1,
        af0,
        crs,
        deltan,
        m0,
        cuc,
        ecc,
        cus,
        sqrta,
        cic,
        omg0,
        cis,
        inc0,
        crc,
        aop,
        omgdot,
        idot,
    })
}

macro_rules! compare_ephemeris_fields {
    ($differences:expr, $decoded:expr, $expected:expr, [$($field:ident),+ $(,)?]) => {
        $(
            compare_field(
                $differences,
                stringify!($field),
                i64::from($decoded.$field),
                i64::from($expected.$field),
            );
        )+
    };
}

pub fn compare_ephemeris(
    decoded: &DecodedEphemeris, expected: &QuantizedEphemeris,
) -> EphemerisDiagnostics {
    let mut differences = Vec::new();
    compare_ephemeris_fields!(
        &mut differences,
        decoded,
        expected,
        [
            week, code_l2, sv_health, iodc, iode, toc, toe, tgd, af2, af1, af0,
            crs, deltan, m0, cuc, ecc, cus, sqrta, cic, omg0, cis, inc0, crc,
            aop, omgdot, idot,
        ]
    );

    EphemerisDiagnostics { differences }
}

fn record_time_of_clock(
    record: &rinex::ephemeris::Ephemeris,
) -> Result<GpsTime, Error> {
    Ok(GpsTime::from(&DateTime::from(
        record.time_of_clock.in_tz("UTC")?,
    )))
}

#[allow(non_snake_case)]
pub fn build_ephemeris_from_decoded(
    decoded: &DecodedEphemeris, reference_week: i32,
) -> BroadcastEphemeris {
    let full_week = expand_week(decoded.week, reference_week);
    let toc = GpsTime {
        week: full_week,
        sec: f64::from(decoded.toc) * 16.0,
    };
    let toe = GpsTime {
        week: full_week,
        sec: f64::from(decoded.toe) * 16.0,
    };

    let sqrta = f64::from(decoded.sqrta) * POW2_M19;
    let A = sqrta * sqrta;
    let ecc = f64::from(decoded.ecc) * POW2_M33;
    let deltan = f64::from(decoded.deltan) * POW2_M43 * PI;
    let omgdot = f64::from(decoded.omgdot) * POW2_M43 * PI;

    BroadcastEphemeris {
        vflg: true,
        t: DateTime::from(&toc),
        toc: toc.clone(),
        toe,
        iodc: i32::from(decoded.iodc),
        iode: i32::from(decoded.iode),
        deltan,
        cuc: f64::from(decoded.cuc) * POW2_M29,
        cus: f64::from(decoded.cus) * POW2_M29,
        cic: f64::from(decoded.cic) * POW2_M29,
        cis: f64::from(decoded.cis) * POW2_M29,
        crc: f64::from(decoded.crc) * POW2_M5,
        crs: f64::from(decoded.crs) * POW2_M5,
        ecc,
        sqrta,
        m0: f64::from(decoded.m0) * POW2_M31 * PI,
        omg0: f64::from(decoded.omg0) * POW2_M31 * PI,
        inc0: f64::from(decoded.inc0) * POW2_M31 * PI,
        aop: f64::from(decoded.aop) * POW2_M31 * PI,
        omgdot,
        idot: f64::from(decoded.idot) * POW2_M43 * PI,
        af0: f64::from(decoded.af0) * POW2_M31,
        af1: f64::from(decoded.af1) * POW2_M43,
        af2: f64::from(decoded.af2) * POW2_M55,
        tgd: f64::from(decoded.tgd) * POW2_M31,
        svhlth: i32::from(decoded.sv_health),
        codeL2: i32::from(decoded.code_l2),
        n: (GM_EARTH / (A * A * A)).sqrt() + deltan,
        sq1e2: (1.0 - ecc * ecc).sqrt(),
        A,
        omgkdot: omgdot - OMEGA_EARTH,
    }
}

fn compare_field(
    differences: &mut Vec<&'static str>, field: &'static str, decoded: i64,
    expected: i64,
) {
    if decoded != expected {
        differences.push(field);
    }
}

fn normalize_sv_health(raw_value: i32) -> i32 {
    if raw_value > 0 && raw_value < 32 {
        raw_value + 32
    } else {
        raw_value
    }
}

fn expand_week(week_mod_1024: u16, reference_week: i32) -> i32 {
    let base = reference_week - reference_week.rem_euclid(1024);
    let mut expanded = base + i32::from(week_mod_1024);
    if expanded - reference_week > 512 {
        expanded -= 1024;
    }
    if reference_week - expanded > 512 {
        expanded += 1024;
    }
    expanded
}

fn combine_signed_32(msb8: u32, lsb24: u32) -> i32 {
    sign_extend((msb8 << 24) | (lsb24 & 0x00ff_ffff), 32)
}

fn sign_extend(value: u32, width: u32) -> i32 {
    let shift = 32 - width;
    ((value << shift) as i32) >> shift
}
