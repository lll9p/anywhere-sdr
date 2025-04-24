use std::{path::PathBuf, process::Command};

use gps::Error;
pub static WORKSPACE_DIR: &str = env!("CARGO_WORKSPACE_DIR");
pub static OUTPUT_DIR: &str = concat!(env!("CARGO_WORKSPACE_DIR"), "/output");
pub static RESOURCES_DIR: &str =
    concat!(env!("CARGO_WORKSPACE_DIR"), "/resources");

pub fn check_gpssim() -> Result<PathBuf, Error> {
    let output_dir = PathBuf::from(OUTPUT_DIR);
    let gps_sim_executable = output_dir
        .join(format!("gpssim{}", if cfg!(windows) { ".exe" } else { "" }));
    // Check if executable of gps-sdr-sim exists
    if !gps_sim_executable.exists() {
        println!(
            "gps-sdr-sim executable not exist, run `gcc gpssim.c -lm -O3 -o \
             gpssim` to build"
        );
        let status = Command::new("gcc")
            .current_dir(WORKSPACE_DIR)
            .args([
                &format!("{RESOURCES_DIR}/gpssim.c"),
                "-lm",
                "-O3",
                "-o",
                &format!("{OUTPUT_DIR}/gpssim"),
            ])
            .spawn()?
            .wait_with_output()?
            .status;
        assert!(status.success(), "Failed to compile gps-sdr-sim");
    }
    assert!(
        gps_sim_executable.exists(),
        "gps-sdr-sim executable does not exist"
    );
    Ok(gps_sim_executable)
}
pub fn prepare_c_bin(
    params: &[Vec<String>], c_bin_file: &str,
) -> Result<(), Error> {
    let gps_sim_executable = check_gpssim()?;
    let c_bin_file_path_buf = PathBuf::from(OUTPUT_DIR).join(c_bin_file);
    if c_bin_file_path_buf.exists() {
        return Ok(());
    }
    let mut args: Vec<_> = params
        .iter()
        .filter(|v| v[0] != "-o")
        .flat_map(|v| {
            if v[0] == "-i" || v[0] == "-v" || v[0] == "-T" {
                vec![v[0].clone()]
            } else {
                v.clone()
            }
        })
        .collect();
    args.push("-o".to_string());
    args.push(format!("{OUTPUT_DIR}/{c_bin_file}"));
    let status = Command::new(gps_sim_executable)
        .current_dir(WORKSPACE_DIR)
        .args(args)
        .spawn()?
        .wait_with_output()?;
    println!("{}", String::from_utf8(status.stderr)?);
    println!("{}", String::from_utf8(status.stdout)?);
    println!("{}", status.status);

    Ok(())
}
