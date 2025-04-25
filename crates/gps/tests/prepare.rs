use std::{
    path::PathBuf,
    process::Command,
    sync::{Mutex, Once},
};

use gps::Error;
pub static WORKSPACE_DIR: &str = env!("CARGO_WORKSPACE_DIR");
pub static OUTPUT_DIR: &str = concat!(env!("CARGO_WORKSPACE_DIR"), "/output");
pub static RESOURCES_DIR: &str =
    concat!(env!("CARGO_WORKSPACE_DIR"), "/resources");

// Use a static mutex to ensure only one test accesses gpssim at a time
static GPSSIM_MUTEX: Mutex<()> = Mutex::new(());
// Use a Once to ensure we only compile gpssim once
static COMPILE_ONCE: Once = Once::new();

pub fn check_gpssim() -> Result<PathBuf, Error> {
    // Ensure output directory exists
    let output_dir = PathBuf::from(OUTPUT_DIR);
    if !output_dir.exists() {
        std::fs::create_dir_all(&output_dir)?;
    }

    // Set gpssim executable path
    let gps_sim_executable = output_dir
        .join(format!("gpssim{}", if cfg!(windows) { ".exe" } else { "" }));

    // Use Once to ensure we only compile gpssim once
    COMPILE_ONCE.call_once(|| {
        // Only compile if the executable doesn't exist
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

            if let Some(gpssim_c_path) = gpssim_c_path {
                // Get source and target paths as strings
                if let (Some(source_path), Some(target_path)) =
                    (gpssim_c_path.to_str(), gps_sim_executable.to_str())
                {
                    // Compile gpssim
                    let _ = Command::new("gcc")
                        .current_dir(WORKSPACE_DIR)
                        .args([source_path, "-lm", "-O3", "-o", target_path])
                        .spawn()
                        .and_then(std::process::Child::wait_with_output);
                }
            }
        }
    });

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

    // Acquire mutex lock to ensure only one test uses gpssim at a time
    // This prevents "Text file busy" errors in parallel test execution
    let _lock = GPSSIM_MUTEX.lock().map_err(|e| {
        Error::msg(format!("Failed to acquire mutex lock: {e}"))
    })?;

    // Check gpssim executable after acquiring lock
    let gps_sim_executable = check_gpssim()?;

    // Check again if file exists (another test might have created it while we
    // were waiting)
    if c_bin_file_path.exists() {
        return Ok(());
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
