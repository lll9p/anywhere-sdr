#![cfg(not(debug_assertions))]
use std::{path::PathBuf, println, process::Command};

use gps::{Error, SignalGeneratorBuilder};
use test_case::test_case;
mod prepare;
use prepare::{OUTPUT_DIR, PATH, prepare_c_bin};
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
                // convert YYYY/MM/DD,hh:mm:ss to YYYY-MM-DD hh:mm:ss
                let value = value.replace('/', "-").replace(',', " ") + "-00";
                builder = builder.time(Some(value))?;
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
// -T <date,time>
// -d <duration>
// -o <output>
// -s <frequency>
// -b <iq_bits>
// -i
// -p [fixed_gain]
// -v
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=31.0;-o=output/rust_1b.bin", "c_1b.bin"; "1bit")]
#[test_case("-e=resources/brdc0010.22n;-b=8;-d=31.0;-o=output/rust_8b.bin", "c_8b.bin"; "8bit")]
#[test_case("-e=resources/brdc0010.22n;-b=16;-d=31.0;-o=output/rust_16b.bin", "c_16b.bin"; "16bit")]
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=31.0;-o=output/rust_1b_f.bin;-s=2000000", "c_1b_f.bin"; "frequency")]
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=31.0;-o=output/rust_gga.bin;-g=resources/triumphv3.txt", "c_gga.bin"; "gga")]
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=31.0;-o=output/rust_circle.bin;-u=resources/circle.csv", "c_circle.bin"; "circle")]
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=31.0;-o=output/rust_circle_llh.bin;-x=resources/circle_llh.csv", "c_circle_llh.bin"; "circle_llh")]
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=31.0;-o=output/rust_location.bin;-l=30.286502,120.032669,100", "c_location.bin"; "location")]
// original gpssim output is wrong skip it.
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=31.0;-o=output/rust_location_ecef.bin;-c=6378137.0,0.0,0.0", "c_location_ecef.bin"; "location_ecef")]
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=31.0;-o=output/rust_1b_p.bin;-p=63", "c_1b_p.bin"; "fixed_gain")]
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=31.0;-o=output/rust_1b_d.bin;-t=2022/01/01,11:45:14", "c_1b_d.bin"; "set_datetime")]
// TODO: should be fixed
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=31.0;-o=output/rust_1b_dr.bin;-t=2022/01/01,11:45:14;-T", "c_1b_dr.bin" => matches Err(_); "set_datetime_override")]
// TODO: should be fixed
#[test_case("-e=resources/brdc0010.22n;-b=1;-d=31.0;-o=output/rust_1b_leap.bin;-l=42.3569048,-71.2564075,0;-t=2022/01/01,23:55;-T;-L=2347,3,17", "c_1b_leap.bin" => matches Err(_); "leap")]
fn test_builder(params: &str, c_bin_file: &str) -> Result<(), Error> {
    let args = string_to_args(params);
    prepare_c_bin(&args, c_bin_file)?;
    let builder = to_builder(&args)?;
    let mut generator = builder.build()?;
    generator.initialize()?;
    generator.run_simulation()?;
    let rust_file_name = generator
        .output_file
        .as_ref()
        .and_then(|p| p.file_name().map(|n| n.to_str()))
        .flatten()
        .ok_or(gps::Error::msg("Can not get file name"))?;
    let rust_file = PathBuf::from(OUTPUT_DIR).join(rust_file_name);
    assert!(rust_file.exists(), "Rust file not exists{rust_file_name}");
    let output = Command::new("diff")
        .current_dir(PathBuf::from(PATH).join("output"))
        .args([rust_file_name, c_bin_file])
        .spawn()?
        .wait_with_output()?;
    let success = output.status.success();
    if success {
        println!("{rust_file_name} and {c_bin_file} are the same!");
        std::fs::remove_file(rust_file)?;
    }
    assert!(success, "Files are different! {rust_file_name}");
    Ok(())
}
