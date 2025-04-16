#![allow(clippy::large_stack_arrays)]
use std::{io::Write, time::Instant};

use crate::{
    channel::Channel,
    constants::*,
    datetime::{DateTime, GpsTime, TimeRange},
    eph::Ephemeris,
    params::Params,
    read_nmea_gga::read_nmea_gga,
    read_rinex::read_rinex_nav_all,
    read_user_motion::{read_user_motion, read_user_motion_llh},
    table::{ANT_PAT_DB, COS_TABLE512, SIN_TABLE512},
    utils::{
        allocate_channel, compute_code_phase, compute_range, eph2sbf,
        generate_nav_msg, llh2xyz, xyz2llh,
    },
};

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
        rho0: TimeRange::default(),
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
        llh[0] = 35.681_298 / R2D;
        llh[1] = 139.766_247 / R2D;
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
            let mut gtmp = GpsTime {
                week: g0.week,
                sec: f64::from(g0.sec as i32 / 7200) * 7200.0,
            };
            // let mut gtmp: GpsTime = GpsTime::default();
            // gtmp.week = g0.week;
            // gtmp.sec = f64::from(g0.sec as i32 / 7200) * 7200.0;
            // Overwrite the UTC reference week number
            let dsec = gtmp.diff_secs(&gmin);
            ionoutc.wnt = gtmp.week;
            ionoutc.tot = gtmp.sec as i32;
            // Iono/UTC parameters may no longer valid
            //ionoutc.vflg = FALSE;
            for sv in 0..MAX_SAT {
                for i_eph in eph.iter_mut().take(neph) {
                    if i_eph[sv].vflg {
                        gtmp = i_eph[sv].toc.add_secs(dsec);
                        let ttmp = DateTime::from(&gtmp);
                        i_eph[sv].toc = gtmp;
                        i_eph[sv].t = ttmp;
                        gtmp = i_eph[sv].toe.add_secs(dsec);
                        i_eph[sv].toe = gtmp;
                    }
                }
            }
        } else if g0.diff_secs(&gmin) < 0.0 || gmax.diff_secs(&g0) < 0.0f64 {
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
                let dt = g0.diff_secs(&e.toc);
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
    // let out_file = String::from_utf8(outfile.iter().map(|&c| c as
    // u8).collect()); if let Ok(out_file) = out_file {
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
    let mut grx = g0.add_secs(0.0);
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
    grx = grx.add_secs(0.1);
    for iumd in 1..numd {
        for i in 0..MAX_CHAN {
            if chan[i].prn > 0 {
                // Refresh code phase and data bit counters
                let mut rho: TimeRange = TimeRange {
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
                let path_loss = 20_200_000.0 / rho.d;
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
                    //                     iTable =
                    // (int)floor(chan[i].carr_phase*512.0);
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
                    //                     chan[i].carr_phase += chan[i].f_carr
                    // * delt;
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
                    | i32::from(i32::from(iq_buff[isamp]) > 0)
                        << (7 - isamp as i32 % 8))
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
                    let dt = eph[ieph + 1][sv].toc.diff_secs(&grx);
                    if dt < SECONDS_IN_HOUR {
                        ieph += 1;
                        // for i in 0..MAX_CHAN {
                        for ichan in chan.iter_mut().take(MAX_CHAN) {
                            // Generate new subframes if allocated
                            if ichan.prn != 0_i32 {
                                eph2sbf(
                                    &eph[ieph][(ichan.prn - 1) as usize],
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
        grx = grx.add_secs(0.1);

        // Update time counter
        eprint!("\rTime into run = {:4.1}\0", grx.diff_secs(&g0));
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
