use crate::{
    DateTime, Ephemeris, GpsTime, IonoUtc, constants::*, process::date2gps, process::sub_gps_time,
};
use std::{fs, path::Path};
pub fn parse_f64(num_string: &str) -> Result<f64, std::num::ParseFloatError> {
    num_string.replace('D', "E").parse()
}

pub fn parse_i32(num_string: &str) -> Result<i32, std::num::ParseIntError> {
    num_string.parse()
}
#[allow(clippy::too_many_lines)]
pub fn read_rinex_nav_all(
    data: &mut [[Ephemeris; MAX_SAT]; EPHEM_ARRAY_SIZE],
    iono_utc: &mut IonoUtc,
    fname: &dyn AsRef<Path>,
) -> anyhow::Result<usize> {
    // Ephemeris vec of vec init;
    let mut ieph: usize = 0;
    let mut sv: usize = 0;
    let content = fs::read_to_string(fname)?;
    let mut flags = 0x0;
    // read header
    let lines = content.lines();
    let mut processing_header = true;
    let mut g = GpsTime::default();
    let mut g0 = GpsTime::default();
    let mut t = DateTime::default();
    let mut lines_of_headers: usize = 0;
    for (iline, line) in lines.enumerate() {
        let strncpy = |start, n| &line[start..start + n];
        if line.contains("END OF HEADER") {
            processing_header = false;
            lines_of_headers = iline + 1;
            continue;
        }
        if processing_header {
            if line.contains("ION ALPHA") {
                iono_utc.alpha0 = parse_f64(strncpy(2, 12).trim())?;
                iono_utc.alpha1 = parse_f64(strncpy(14, 12).trim())?;
                iono_utc.alpha2 = parse_f64(strncpy(26, 12).trim())?;
                iono_utc.alpha3 = parse_f64(strncpy(38, 12).trim())?;
                flags |= 0x1;
            }
            if line.contains("ION BETA") {
                iono_utc.beta0 = parse_f64(strncpy(2, 12).trim())?;
                iono_utc.beta1 = parse_f64(strncpy(14, 12).trim())?;
                iono_utc.beta2 = parse_f64(strncpy(26, 12).trim())?;
                iono_utc.beta3 = parse_f64(strncpy(38, 12).trim())?;
                flags |= 0x1 << 1;
            }
            if line.contains("DELTA-UTC") {
                iono_utc.A0 = parse_f64(strncpy(3, 19).trim())?;
                iono_utc.A1 = parse_f64(strncpy(22, 19).trim())?;
                iono_utc.tot = parse_i32(strncpy(41, 9).trim())?;
                iono_utc.wnt = parse_i32(strncpy(50, 9).trim())?;
                if iono_utc.tot % 4096 == 0 {
                    flags |= 0x1 << 2;
                }
            }
            if line.contains("LEAP SECONDS") {
                iono_utc.dtls = parse_i32(strncpy(0, 6).trim())?;
                flags |= 0x1 << 3;
            }
        }
        if !processing_header {
            iono_utc.vflg = false;
            // Read all Iono/UTC lines
            if flags == 0xF {
                iono_utc.vflg = true;
            }
            g0.week = -1;
            ieph = 0;
        }
    }
    let lines = content.lines();
    for (iline, line) in lines.skip(lines_of_headers).enumerate() {
        let strncpy = |start, n| &line[start..start + n];
        if iline % 8 == 0 {
            sv = strncpy(0, 2).trim().parse::<usize>()? - 1;
            t.y = strncpy(3, 2).trim().parse::<i32>()? + 2000;
            t.m = strncpy(6, 2).trim().parse::<i32>()?;
            t.d = strncpy(9, 2).trim().parse::<i32>()?;
            t.hh = strncpy(12, 2).trim().parse::<i32>()?;
            t.mm = strncpy(15, 2).trim().parse::<i32>()?;
            t.sec = strncpy(18, 2).trim().parse::<f64>()?;
            date2gps(&t, &mut g);
            // let g = GpsTime::from(&t);

            // if first line of block
            if iline == 0 {
                g0 = g;
            }
            // Check current time of clock
            let dt = sub_gps_time(g, g0);
            if dt > SECONDS_IN_HOUR {
                g0 = g;
                ieph += 1; // a new set of ephemerides
                if ieph >= EPHEM_ARRAY_SIZE {
                    break;
                }
            }
            // Date and time
            data[ieph][sv].t = t;
            // SV CLK
            data[ieph][sv].toc = g;
            data[ieph][sv].af0 = parse_f64(strncpy(22, 19).trim())?;
            data[ieph][sv].af1 = parse_f64(strncpy(41, 19).trim())?;
            data[ieph][sv].af2 = parse_f64(strncpy(60, 19).trim())?;
        }
        if iline % 8 == 1 {
            data[ieph][sv].iode = parse_f64(strncpy(3, 19).trim())? as i32;
            data[ieph][sv].crs = parse_f64(strncpy(22, 19).trim())?;
            data[ieph][sv].deltan = parse_f64(strncpy(41, 19).trim())?;
            data[ieph][sv].m0 = parse_f64(strncpy(60, 19).trim())?;
        }
        if iline % 8 == 2 {
            data[ieph][sv].cuc = parse_f64(strncpy(3, 19).trim())?;
            data[ieph][sv].ecc = parse_f64(strncpy(22, 19).trim())?;
            data[ieph][sv].cus = parse_f64(strncpy(41, 19).trim())?;
            data[ieph][sv].sqrta = parse_f64(strncpy(60, 19).trim())?;
        }
        if iline % 8 == 3 {
            data[ieph][sv].toe.sec = parse_f64(strncpy(3, 19).trim())?;
            data[ieph][sv].cic = parse_f64(strncpy(22, 19).trim())?;
            data[ieph][sv].omg0 = parse_f64(strncpy(41, 19).trim())?;
            data[ieph][sv].cis = parse_f64(strncpy(60, 19).trim())?;
        }
        if iline % 8 == 4 {
            data[ieph][sv].inc0 = parse_f64(strncpy(3, 19).trim())?;
            data[ieph][sv].crc = parse_f64(strncpy(22, 19).trim())?;
            data[ieph][sv].aop = parse_f64(strncpy(41, 19).trim())?;
            data[ieph][sv].omgdot = parse_f64(strncpy(60, 19).trim())?;
        }
        if iline % 8 == 5 {
            data[ieph][sv].idot = parse_f64(strncpy(3, 19).trim())?;
            data[ieph][sv].codeL2 = parse_f64(strncpy(22, 19).trim())? as i32;
            data[ieph][sv].toe.week = parse_f64(strncpy(41, 19).trim())? as i32;
        }
        if iline % 8 == 6 {
            data[ieph][sv].svhlth = parse_f64(strncpy(22, 19).trim())? as i32;
            if data[ieph][sv].svhlth > 0 && data[ieph][sv].svhlth < 32 {
                data[ieph][sv].svhlth += 32;
            }
            data[ieph][sv].tgd = parse_f64(strncpy(41, 19).trim())?;
            data[ieph][sv].iodc = parse_f64(strncpy(60, 19).trim())? as i32;
        }
        if iline % 8 == 7 {
            // Set valid flag
            data[ieph][sv].vflg = true;

            // Update the working variables
            data[ieph][sv].A = data[ieph][sv].sqrta * data[ieph][sv].sqrta;
            data[ieph][sv].n =
                (GM_EARTH / (data[ieph][sv].A * data[ieph][sv].A * data[ieph][sv].A)).sqrt()
                    + data[ieph][sv].deltan;
            data[ieph][sv].sq1e2 = (1.0 - data[ieph][sv].ecc * data[ieph][sv].ecc).sqrt();
            data[ieph][sv].omgkdot = data[ieph][sv].omgdot - OMEGA_EARTH;
        }
    }
    if g0.week >= 0 {
        ieph += 1;
    }
    Ok(ieph)
}
