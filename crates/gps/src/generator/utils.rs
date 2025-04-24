use std::path::Path;

use constants::{
    EPHEM_ARRAY_SIZE, GM_EARTH, MAX_SAT, OMEGA_EARTH, SECONDS_IN_HOUR,
};

use crate::{
    datetime::{DateTime, GpsTime},
    ephemeris::Ephemeris,
    ionoutc::IonoUtc,
};
#[derive(Debug)]
pub enum MotionMode {
    Static,
    Dynamic,
    // UserControl
}
type Data = (
    usize,
    IonoUtc,
    Box<[[Ephemeris; MAX_SAT]; EPHEM_ARRAY_SIZE]>,
);

/// Reads Iono/UTC parameters and ephemeris data from RINEX navigation file.
///
/// Ephemeris data is stored in approximately hourly groupings.
pub fn read_navigation_data(
    file: &dyn AsRef<Path>,
) -> Result<Data, crate::Error> {
    let rinex_data = rinex::Rinex::read_file(file)?;
    let mut ephemeris_data: Box<[[Ephemeris; MAX_SAT]; EPHEM_ARRAY_SIZE]> =
        std::array::from_fn(|_| std::array::from_fn(|_| Ephemeris::default()))
            .into();
    let mut iono_utc = IonoUtc::default();

    iono_utc.read_from_rinex(&rinex_data);
    let mut current_set_index = 0;
    let mut current_set_start_time: Option<GpsTime> = None;

    // Iterate through all ephemeris records in RINEX file
    for rinex_record in &rinex_data.ephemerides {
        // Calculate satellite index (0-based)
        // PRN (Pseudo-Random Noise) code number, often 1-based in RINEX
        let Some(sv) = rinex_record.prn.checked_sub(1) else {
            // Log or handle PRN 0 case if necessary
            eprintln!(
                "Warning: Encountered RINEX record with PRN 0, skipping."
            );
            continue;
        };
        if sv >= MAX_SAT {
            eprintln!(
                "Warning: Skipping ephemeris for SV PRN {} (index {}) as it \
                 exceeds MAX_SAT {}",
                rinex_record.prn, sv, MAX_SAT
            );
            continue;
        }

        let utc_datetime =
            DateTime::from(rinex_record.time_of_clock.in_tz("UTC")?);
        let gps_time = GpsTime::from(&utc_datetime);

        // --- Determine which time set this ephemeris belongs to ---
        let mut update_set = false;
        match current_set_start_time {
            Some(ref start_time) => {
                // If time difference exceeds 1 hour
                if gps_time.diff_secs(start_time).abs() > SECONDS_IN_HOUR {
                    update_set = true;
                }
            }
            None => {
                // First valid ephemeris record, set current set start time
                current_set_start_time = Some(gps_time.clone());
                // No need to update index since we start at index 0
            }
        }

        if update_set {
            // Move to next ephemeris set
            current_set_index += 1;
            // Check if new set index exceeds bounds
            if current_set_index >= EPHEM_ARRAY_SIZE {
                println!(
                    "Warning: Reached maximum ephemeris sets \
                     ({EPHEM_ARRAY_SIZE}). Stopping processing."
                );
                break; // Stop processing more records
            }
            // 更新新集合的开始时间
            current_set_start_time = Some(gps_time.clone());
        }
        // --- End set index logic ---

        // 获取要填充的目标 Ephemeris 结构体的可变引用
        // current_set_index is guaranteed to be in bounds here
        let eph = &mut ephemeris_data[current_set_index][sv];
        eph.t = utc_datetime;
        eph.toc = gps_time;
        eph.af0 = rinex_record.sv_clock.bias;
        eph.af1 = rinex_record.sv_clock.drift;
        eph.af2 = rinex_record.sv_clock.drift_rate;

        // orbit1
        eph.iode = rinex_record.orbit1.iode as i32;
        eph.crs = rinex_record.orbit1.crs;
        eph.deltan = rinex_record.orbit1.delta_n;
        eph.m0 = rinex_record.orbit1.m0;

        // orbit2
        eph.cuc = rinex_record.orbit2.cuc;
        eph.ecc = rinex_record.orbit2.ecc;
        eph.cus = rinex_record.orbit2.cus;
        eph.sqrta = rinex_record.orbit2.sqrta;

        // orbit3
        eph.toe.sec = rinex_record.orbit3.toe;
        eph.cic = rinex_record.orbit3.cic;
        eph.omg0 = rinex_record.orbit3.omega;
        eph.cis = rinex_record.orbit3.cis;

        // orbit4
        eph.inc0 = rinex_record.orbit4.i0;
        eph.crc = rinex_record.orbit4.crc;
        eph.aop = rinex_record.orbit4.omega;
        eph.omgdot = rinex_record.orbit4.omega_dot;

        // orbit5
        eph.idot = rinex_record.orbit5.idot;
        eph.codeL2 = rinex_record.orbit5.code_l2 as i32;
        eph.toe.week = rinex_record.orbit5.week as i32;

        // orbit6
        eph.svhlth = rinex_record.orbit6.sv_health as i32;
        if eph.svhlth > 0 && eph.svhlth < 32 {
            eph.svhlth += 32;
        }
        eph.tgd = rinex_record.orbit6.tgd;
        eph.iodc = rinex_record.orbit6.iodc as i32;

        // Set valid flag
        eph.vflg = true;
        eph.A = eph.sqrta * eph.sqrta;
        eph.n = (GM_EARTH / (eph.A * eph.A * eph.A)).sqrt() + eph.deltan;
        eph.sq1e2 = (1.0 - eph.ecc * eph.ecc).sqrt();
        eph.omgkdot = eph.omgdot - OMEGA_EARTH;
    }

    if current_set_start_time.is_some() {
        current_set_index += 1;
    }
    Ok((current_set_index, iono_utc, ephemeris_data))
}
