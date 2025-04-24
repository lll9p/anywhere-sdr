use std::{path::PathBuf, process::Command};

use gps::Error;
pub static WORKSPACE_DIR: &str = env!("CARGO_WORKSPACE_DIR");
pub static OUTPUT_DIR: &str = concat!(env!("CARGO_WORKSPACE_DIR"), "/output");
pub static RESOURCES_DIR: &str =
    concat!(env!("CARGO_WORKSPACE_DIR"), "/resources");

pub fn check_gpssim() -> Result<PathBuf, Error> {
    // Ensure output directory exists
    let output_dir = PathBuf::from(OUTPUT_DIR);
    if !output_dir.exists() {
        std::fs::create_dir_all(&output_dir)?;
    }

    // Set gpssim executable path
    let gps_sim_executable = output_dir
        .join(format!("gpssim{}", if cfg!(windows) { ".exe" } else { "" }));

    // Check if gpssim executable exists
    if !gps_sim_executable.exists() {
        // Try to find gpssim.c file in multiple possible locations
        let possible_paths = vec![
            PathBuf::from(RESOURCES_DIR).join("gpssim.c"),
            PathBuf::from(WORKSPACE_DIR)
                .join("resources")
                .join("gpssim.c"),
        ];

        let mut gpssim_c_path = None;
        for path in &possible_paths {
            if path.exists() {
                gpssim_c_path = Some(path.clone());
                break;
            }
        }

        let Some(gpssim_c_path) = gpssim_c_path else {
            return Err(Error::msg(format!(
                "Could not find gpssim.c source file, tried paths: \
                 {possible_paths:?}"
            )));
        };

        // Get source and target paths as strings
        let source_path = gpssim_c_path
            .to_str()
            .ok_or_else(|| Error::msg("Invalid source path"))?;
        let target_path = gps_sim_executable
            .to_str()
            .ok_or_else(|| Error::msg("Invalid target path"))?;

        // Compile gpssim
        let status = Command::new("gcc")
            .current_dir(WORKSPACE_DIR)
            .args([source_path, "-lm", "-O3", "-o", target_path])
            .spawn()?
            .wait_with_output()?;

        // Check if compilation was successful
        if !status.status.success() {
            return Err(Error::msg(format!(
                "Failed to compile gpssim, error code: {}",
                status.status
            )));
        }
    }

    // Final check if executable exists
    if !gps_sim_executable.exists() {
        return Err(Error::msg(format!(
            "gpssim executable does not exist: {}",
            gps_sim_executable.display()
        )));
    }

    Ok(gps_sim_executable)
}
pub fn prepare_c_bin(
    params: &[Vec<String>], c_bin_file: &str,
) -> Result<(), Error> {
    let gps_sim_executable = check_gpssim()?;

    // Get full path
    let c_bin_file_path = PathBuf::from(c_bin_file);

    // If file already exists, return immediately
    if c_bin_file_path.exists() {
        return Ok(());
    }

    // Ensure output directory exists
    if let Some(parent) = c_bin_file_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Convert parameters to C-style arguments
    let mut args: Vec<String> = Vec::new();
    for v in params.iter().filter(|v| v[0] != "-o") {
        if v[0] == "-i" || v[0] == "-v" || v[0] == "-T" {
            // Flags without values
            args.push(v[0].clone());
        } else {
            // Parameters with values
            args.push(v[0].clone());
            args.push(v[1].clone());
        }
    }

    // Add output file parameter
    args.push("-o".to_string());
    args.push(c_bin_file.to_string());

    // Run gpssim command with stdout and stderr redirected to null

    let status = Command::new(gps_sim_executable)
        .current_dir(WORKSPACE_DIR)
        .args(&args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?
        .wait_with_output()?;

    // Check if command executed successfully
    if !status.status.success() {
        return Err(Error::msg(format!(
            "Failed to generate C version output file: {c_bin_file}"
        )));
    }

    // Verify file was created
    if !c_bin_file_path.exists() {
        return Err(Error::msg(format!(
            "C version output file was not generated: {c_bin_file}"
        )));
    }

    Ok(())
}
