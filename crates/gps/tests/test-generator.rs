#![cfg(not(debug_assertions))]
use std::{collections::BTreeSet, path::PathBuf};

use gps::{DataFormat, Error, IqBlockSizing, SignalGeneratorBuilder};
use test_case::test_case;
#[path = "test-generator/carrier_phase_golden.rs"]
mod carrier_phase_golden;
mod prepare;
use carrier_phase_golden::{
    CARRIER_PHASE_GOLDENS, RELEASE_C_CASE_COUNT, RustOutputFingerprint,
    expected_fingerprint,
};
use prepare::{OUTPUT_DIR, RESOURCES_DIR, prepare_c_bin};

fn content_fingerprint(bytes: &[u8]) -> RustOutputFingerprint {
    let mut hash_a = 0xcbf2_9ce4_8422_2325_u64;
    let mut hash_b = 0x8422_2325_cbf2_9ce4_u64;
    for byte in bytes {
        hash_a ^= u64::from(*byte);
        hash_a = hash_a.wrapping_mul(0x0000_0100_0000_01b3);
        hash_b ^= u64::from(*byte);
        hash_b = hash_b.rotate_left(5).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
    RustOutputFingerprint {
        length: bytes.len() as u64,
        hash_a,
        hash_b,
    }
}

#[test]
fn carrier_phase_goldens_cover_every_release_c_case() {
    assert_eq!(CARRIER_PHASE_GOLDENS.len(), RELEASE_C_CASE_COUNT);
    let unique_paths = CARRIER_PHASE_GOLDENS
        .iter()
        .map(|(path, _)| *path)
        .collect::<BTreeSet<_>>();
    assert_eq!(unique_paths.len(), RELEASE_C_CASE_COUNT);
}
#[allow(non_snake_case)]
fn to_builder(args: &[Vec<String>]) -> Result<SignalGeneratorBuilder, Error> {
    let mut builder = SignalGeneratorBuilder::default();
    for arg in args {
        match arg.as_slice() {
            [e, navfile] if e == "-e" => {
                builder =
                    builder.navigation_file(Some(PathBuf::from(navfile)))?;
            }
            [u, value] if u == "-u" => {
                builder =
                    builder.user_motion_file(Some(PathBuf::from(value)))?;
            }
            [x, value] if x == "-x" => {
                builder =
                    builder.user_motion_llh_file(Some(PathBuf::from(value)))?;
            }
            [g, value] if g == "-g" => {
                builder = builder
                    .user_motion_nmea_gga_file(Some(PathBuf::from(value)))?;
            }
            [c, value] if c == "-c" => {
                let location = value
                    .split(',')
                    .map(|s| s.parse::<f64>().unwrap())
                    .collect::<Vec<_>>();
                builder = builder.location_ecef(Some(location))?;
            }
            [l, value] if l == "-l" => {
                let location = value
                    .split(',')
                    .map(|s| s.parse::<f64>().unwrap())
                    .collect::<Vec<_>>();
                builder = builder.location(Some(location))?;
            }
            [L, value] if L == "-L" => {
                let leap = value
                    .split(',')
                    .map(|s| s.parse::<i32>().unwrap())
                    .collect::<Vec<_>>();
                builder = builder.leap(Some(leap));
            }
            [t, value] if t == "-t" => {
                // The original C accepts a GPS-system calendar label here.
                let value = value.replace('/', "-").replace(',', "T");
                builder = builder.gps_calendar_time(Some(value))?;
            }
            [T, ..] if T == "-T" => {
                builder = builder.time_override(Some(true));
            }
            [d, value] if d == "-d" => {
                let duration: f64 = value.parse()?;
                builder = builder.duration(Some(duration));
            }
            [o, value] if o == "-o" => {
                builder = builder.output_file(Some(PathBuf::from(value)));
            }
            [s, value] if s == "-s" => {
                let freq = value.parse()?;
                builder = builder.frequency(Some(freq))?;
            }
            [b, value] if b == "-b" => {
                let data_format = value.parse()?;
                builder = builder.data_format(Some(data_format))?;
            }
            [i, ..] if i == "-i" => {
                builder = builder.ionospheric_disable(Some(true));
            }
            [p, value] if p == "-p" => {
                let loss = value.parse()?;
                builder = builder.path_loss(Some(loss));
            }
            [v, ..] if v == "-v" => {
                builder = builder.verbose(Some(true));
            }
            _ => {
                panic!()
            }
        }
    }
    Ok(builder)
}
fn string_to_args(value: &str) -> Vec<Vec<String>> {
    value
        .split(';')
        .map(|s| {
            let s = s.trim();
            if s.starts_with("-i") || s.starts_with("-v") || s.starts_with("-T")
            {
                vec![s.to_string(), String::new()]
            } else {
                let arg: Vec<String> =
                    s.split('=').map(ToString::to_string).collect();
                assert!(arg.len() == 2);
                arg
            }
        })
        .collect()
}

// -e <gps_nav>
// -u <user_motion>
// -x <user_motion>
// -g <nmea_gga>
// -c <location>
// -l <location>
// -L <wnslf,dn,dtslf>
// -t <date,time>
// -T
// -d <duration>
// -o <output>
// -s <frequency>
// -b <iq_bits>
// -i
// -p [fixed_gain]
// -v
// Every case runs the legacy C fixture for process, length, and explicit
// zero-phase divergence checks, then validates the Rust output against its
// reviewed fingerprint. C sample bytes are intentionally not a content oracle.

// Basic data format tests
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=31.0;-o=output/format_1bit.bin", "output/c_format_1bit.bin"; "test_data_format_1bit")]
#[test_case("-e=resources/brdc0010.22n;-b=8;-d=31.0;-o=output/format_8bit.bin", "output/c_format_8bit.bin"; "test_data_format_8bit")]
#[test_case("-e=resources/brdc0010.22n;-b=16;-d=31.0;-o=output/format_16bit.bin", "output/c_format_16bit.bin"; "test_data_format_16bit")]
// Sampling frequency tests
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=31.0;-o=output/freq_2mhz_1bit.bin;-s=2000000", "output/c_freq_2mhz_1bit.bin"; "test_sampling_frequency_2mhz")]
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=31.0;-o=output/freq_1mhz_1bit.bin;-s=1000000", "output/c_freq_1mhz_1bit.bin"; "test_sampling_frequency_1mhz")]
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=31.0;-o=output/freq_5mhz_1bit.bin;-s=5000000", "output/c_freq_5mhz_1bit.bin"; "test_sampling_frequency_5mhz")]
#[test_case("-e=resources/brdc0010.22n;-b=8;-d=31.0;-o=output/freq_2mhz_8bit.bin;-s=2000000", "output/c_freq_2mhz_8bit.bin"; "test_data_format_8bit_with_2mhz")]
#[test_case("-e=resources/brdc0010.22n;-b=16;-d=31.0;-o=output/freq_2mhz_16bit.bin;-s=2000000", "output/c_freq_2mhz_16bit.bin"; "test_data_format_16bit_with_2mhz")]
// User motion tests
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=31.0;-o=output/motion_nmea_gga.bin;-g=resources/triumphv3.txt", "output/c_motion_nmea_gga.bin"; "test_user_motion_nmea_gga")]
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=31.0;-o=output/motion_ecef_circle.bin;-u=resources/circle.csv", "output/c_motion_ecef_circle.bin"; "test_user_motion_ecef_circle")]
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=31.0;-o=output/motion_llh_circle.bin;-x=resources/circle_llh.csv", "output/c_motion_llh_circle.bin"; "test_user_motion_llh_circle")]
// Static location tests
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=31.0;-o=output/static_llh_hangzhou.bin;-l=30.286502,120.032669,100", "output/c_static_llh_hangzhou.bin"; "test_static_location_llh_hangzhou")]
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=31.0;-o=output/static_llh_tokyo.bin;-l=35.681298,139.766247,100", "output/c_static_llh_tokyo.bin"; "test_static_location_llh_tokyo")]
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=31.0;-o=output/static_ecef_coords.bin;-c=-3813477.954,3554276.552,3662785.237", "output/c_static_ecef_coords.bin"; "test_static_location_ecef")]
// Signal gain tests
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=31.0;-o=output/gain_fixed_63.bin;-p=63", "output/c_gain_fixed_63.bin"; "test_fixed_gain_63")]
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=31.0;-o=output/gain_fixed_128.bin;-p=128", "output/c_gain_fixed_128.bin"; "test_fixed_gain_128")]
// Time setting tests
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=31.0;-o=output/time_custom_start.bin;-t=2022/01/01,11:45:14", "output/c_time_custom_start.bin"; "test_custom_start_time")]
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=31.0;-o=output/time_override_toc_toe.bin;-t=2022/01/01,11:45:14;-T", "output/c_time_override_toc_toe.bin"; "test_time_override_toc_toe")]
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=31.0;-o=output/time_leap_second.bin;-l=42.3569048,-71.2564075,0;-t=2022/01/01,23:55;-T;-L=2347,3,17", "output/c_time_leap_second.bin"; "test_leap_second_settings")]
// Ionospheric and verbose output tests
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=31.0;-o=output/iono_disabled.bin;-i", "output/c_iono_disabled.bin"; "test_ionospheric_delay_disable")]
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=31.0;-o=output/verbose_output.bin;-v", "output/c_verbose_output.bin"; "test_verbose_output_mode")]
// Duration tests
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=10.0;-o=output/duration_10sec.bin", "output/c_duration_10sec.bin"; "test_simulation_duration_10sec")]
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=60.0;-o=output/duration_60sec.bin", "output/c_duration_60sec.bin"; "test_simulation_duration_60sec")]
// Parameter combination tests
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=31.0;-o=output/combo_tokyo_2mhz_8bit.bin;-l=35.681298,139.766247,100;-s=2000000;-b=8", "output/c_combo_tokyo_2mhz_8bit.bin"; "test_combo_tokyo_2mhz_8bit")]
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=31.0;-o=output/combo_hangzhou_gain100_noiono.bin;-l=30.286502,120.032669,100;-p=100;-i", "output/c_combo_hangzhou_gain100_noiono.bin"; "test_combo_hangzhou_gain100_noiono")]
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=31.0;-o=output/combo_ecef_3mhz_16bit.bin;-c=-3813477.954,3554276.552,3662785.237;-s=3000000;-b=16", "output/c_combo_ecef_3mhz_16bit.bin"; "test_combo_ecef_3mhz_16bit")]
fn test_builder(params: &str, c_bin_file: &str) -> Result<(), Error> {
    // Replace paths in the parameters
    let mut modified_params = params.to_string();
    modified_params =
        modified_params.replace("resources/", &format!("{}/", RESOURCES_DIR));
    modified_params =
        modified_params.replace("output/", &format!("{}/", OUTPUT_DIR));

    // Ensure C version output file path is correct
    let c_bin_file_full = if !c_bin_file.starts_with(OUTPUT_DIR) {
        format!(
            "{}/{}",
            OUTPUT_DIR,
            c_bin_file.trim_start_matches("output/")
        )
    } else {
        c_bin_file.to_string()
    };

    let args = string_to_args(&modified_params);
    prepare_c_bin(&args, &c_bin_file_full)?;
    let builder = to_builder(&args)?;
    let mut generator = builder.build()?;
    generator.initialize()?;
    generator.run_simulation()?;

    // Get the full path of the output file
    let rust_file = generator
        .output_file
        .clone()
        .ok_or_else(|| gps::Error::msg("Output file not set"))?;

    assert!(
        rust_file.exists(),
        "Rust file does not exist: {:?}",
        rust_file
    );

    let rust_file_name = rust_file
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| gps::Error::msg("Cannot get file name"))?;

    let c_bin_path = PathBuf::from(&c_bin_file_full);
    if !c_bin_path.exists() {
        return Err(Error::msg(format!(
            "C version output file does not exist: {c_bin_file_full}"
        )));
    }

    let rust_bytes = std::fs::read(&rust_file)?;
    let c_bytes = std::fs::read(&c_bin_path)?;
    let block_sizing = IqBlockSizing::new(generator.iq_buffer_size)?;
    let bytes_per_block = match generator.data_format {
        DataFormat::Bits1 => block_sizing.interleaved_i16_len() / 8,
        DataFormat::Bits8 => block_sizing.interleaved_i16_len(),
        DataFormat::Bits16 => block_sizing.interleaved_bytes(),
    };

    // The legacy C fixture still exercises process and length compatibility,
    // but its zero-phase samples are not a Rust content oracle. It emits one
    // fewer interval than requested; Rust emits that final interval by
    // contract.
    assert_eq!(
        rust_bytes.len(),
        c_bytes.len() + bytes_per_block,
        "Rust output must exceed legacy C output by exactly one block: \
         {rust_file_name}"
    );
    assert_ne!(
        &rust_bytes[..c_bytes.len()],
        c_bytes,
        "modeled-range carrier phase must diverge from the zero-phase C \
         prefix: {rust_file_name}"
    );
    let expected = expected_fingerprint(c_bin_file).ok_or_else(|| {
        Error::msg(format!(
            "Missing carrier-phase golden for release C case {c_bin_file}"
        ))
    })?;
    assert_eq!(
        content_fingerprint(&rust_bytes),
        expected,
        "Rust carrier-phase golden differs for {rust_file_name}"
    );
    std::fs::remove_file(&rust_file)?;
    Ok(())
}
