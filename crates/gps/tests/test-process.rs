use anyhow::Result;
use gps::{process, Params, R2D};
use std::path::PathBuf;
use std::process::Command;
use std::sync::LazyLock;
use test_case::test_case;
const PATH: &str = env!("CARGO_MANIFEST_DIR");

static PARAMS: LazyLock<Vec<(Params, String, String)>> =
    LazyLock::new(|| prepare_params());

fn prepare_params() -> Vec<(Params, String, String)> {
    let ephemerides: PathBuf =
        PathBuf::from(PATH).join("resources").join("brdc0010.22n");
    let user_motion_ecef: Option<PathBuf> =
        Some(PathBuf::from(PATH).join("resources").join("circle.csv"));
    let user_motion_llh: Option<PathBuf> =
        Some(PathBuf::from(PATH).join("resources").join("circle_llh.csv"));
    let nmea_gga: Option<PathBuf> =
        Some(PathBuf::from(PATH).join("resources").join("triumphv3.txt"));
    let location_ecef: Option<Vec<f64>> =
        Some(vec![3967283.154, 1022538.181, 4872414.484]);
    let location: Option<Vec<f64>> = Some(vec![30.286502, 120.032669, 100.0]);
    let leap: Option<Vec<i32>> = Some(vec![2347, 3, 19]);
    let time: Option<String> = Some("now".to_string());
    let time_override: Option<String> =
        Some("2022-10-10T10::22:22Z".to_string());
    let duration: Option<usize> = Some(31);
    let output: Option<PathBuf>;
    let frequency: usize = 2600000;
    let bits: usize = 8;
    let ionospheric_disable: bool = false;
    let path_loss: Option<i32> = None;
    let verbose: bool = false;
    let params_1b = Params::new(
        &ephemerides, // ephemerides: &Path,
        &None,        // user_motion_ecef: &Option<PathBuf>,
        &None,        // user_motion_llh: &Option<PathBuf>,
        &None,        // nmea_gga: &Option<PathBuf>,
        None,         // location_ecef: Option<Vec<f64>>,
        None,         // location: Option<Vec<f64>>,
        &None,        // leap: &Option<Vec<i32>>,
        &None,        // time: &Option<String>,
        &None,        // time_override: &Option<String>,
        &duration,    // duration: &Option<usize>,
        &Some(PathBuf::from(PATH).join("output").join("rust_1b.bin")), // output: &Option<PathBuf>,
        frequency,           // frequency: usize,
        1,                   // bits: usize,
        ionospheric_disable, // ionospheric_disable: bool,
        &path_loss,          // path_loss: &Option<i32>,
        verbose,             // verbose: bool,
    );
    let params_8b = Params::new(
        &ephemerides, // ephemerides: &Path,
        &None,        // user_motion_ecef: &Option<PathBuf>,
        &None,        // user_motion_llh: &Option<PathBuf>,
        &None,        // nmea_gga: &Option<PathBuf>,
        None,         // location_ecef: Option<Vec<f64>>,
        None,         // location: Option<Vec<f64>>,
        &None,        // leap: &Option<Vec<i32>>,
        &None,        // time: &Option<String>,
        &None,        // time_override: &Option<String>,
        &duration,    // duration: &Option<usize>,
        &Some(PathBuf::from(PATH).join("output").join("rust_8b.bin")), // output: &Option<PathBuf>,
        frequency,           // frequency: usize,
        8,                   // bits: usize,
        ionospheric_disable, // ionospheric_disable: bool,
        &path_loss,          // path_loss: &Option<i32>,
        verbose,             // verbose: bool,
    );
    let params_16b = Params::new(
        &ephemerides, // ephemerides: &Path,
        &None,        // user_motion_ecef: &Option<PathBuf>,
        &None,        // user_motion_llh: &Option<PathBuf>,
        &None,        // nmea_gga: &Option<PathBuf>,
        None,         // location_ecef: Option<Vec<f64>>,
        None,         // location: Option<Vec<f64>>,
        &None,        // leap: &Option<Vec<i32>>,
        &None,        // time: &Option<String>,
        &None,        // time_override: &Option<String>,
        &duration,    // duration: &Option<usize>,
        &Some(PathBuf::from(PATH).join("output").join("rust_16b.bin")), // output: &Option<PathBuf>,
        frequency,           // frequency: usize,
        16,                  // bits: usize,
        ionospheric_disable, // ionospheric_disable: bool,
        &path_loss,          // path_loss: &Option<i32>,
        verbose,             // verbose: bool,
    );
    let params_gga = Params::new(
        &ephemerides, // ephemerides: &Path,
        &None,        // user_motion_ecef: &Option<PathBuf>,
        &None,        // user_motion_llh: &Option<PathBuf>,
        &nmea_gga,    // nmea_gga: &Option<PathBuf>,
        None,         // location_ecef: Option<Vec<f64>>,
        None,         // location: Option<Vec<f64>>,
        &None,        // leap: &Option<Vec<i32>>,
        &None,        // time: &Option<String>,
        &None,        // time_override: &Option<String>,
        &duration,    // duration: &Option<usize>,
        &Some(PathBuf::from(PATH).join("output").join("rust_gga.bin")), // output: &Option<PathBuf>,
        frequency,           // frequency: usize,
        1,                   // bits: usize,
        ionospheric_disable, // ionospheric_disable: bool,
        &path_loss,          // path_loss: &Option<i32>,
        verbose,             // verbose: bool,
    );
    let params_circle = Params::new(
        &ephemerides,      // ephemerides: &Path,
        &user_motion_ecef, // user_motion_ecef: &Option<PathBuf>,
        &None,             // user_motion_llh: &Option<PathBuf>,
        &None,             // nmea_gga: &Option<PathBuf>,
        None,              // location_ecef: Option<Vec<f64>>,
        None,              // location: Option<Vec<f64>>,
        &None,             // leap: &Option<Vec<i32>>,
        &None,             // time: &Option<String>,
        &None,             // time_override: &Option<String>,
        &duration,         // duration: &Option<usize>,
        &Some(PathBuf::from(PATH).join("output").join("rust_circle.bin")), // output: &Option<PathBuf>,
        frequency,           // frequency: usize,
        1,                   // bits: usize,
        ionospheric_disable, // ionospheric_disable: bool,
        &path_loss,          // path_loss: &Option<i32>,
        verbose,             // verbose: bool,
    );
    let params_circle_llh = Params::new(
        &ephemerides,     // ephemerides: &Path,
        &None,            // user_motion_ecef: &Option<PathBuf>,
        &user_motion_llh, // user_motion_llh: &Option<PathBuf>,
        &None,            // nmea_gga: &Option<PathBuf>,
        None,             // location_ecef: Option<Vec<f64>>,
        None,             // location: Option<Vec<f64>>,
        &None,            // leap: &Option<Vec<i32>>,
        &None,            // time: &Option<String>,
        &None,            // time_override: &Option<String>,
        &duration,        // duration: &Option<usize>,
        &Some(
            PathBuf::from(PATH)
                .join("output")
                .join("rust_circle_llh.bin"),
        ), // output: &Option<PathBuf>,
        frequency,        // frequency: usize,
        1,                // bits: usize,
        ionospheric_disable, // ionospheric_disable: bool,
        &path_loss,       // path_loss: &Option<i32>,
        verbose,          // verbose: bool,
    );
    let params_location = Params::new(
        &ephemerides, // ephemerides: &Path,
        &None,        // user_motion_ecef: &Option<PathBuf>,
        &None,        // user_motion_llh: &Option<PathBuf>,
        &None,        // nmea_gga: &Option<PathBuf>,
        None,         // location_ecef: Option<Vec<f64>>,
        location,     // location: Option<Vec<f64>>,
        &None,        // leap: &Option<Vec<i32>>,
        &None,        // time: &Option<String>,
        &None,        // time_override: &Option<String>,
        &duration,    // duration: &Option<usize>,
        &Some(PathBuf::from(PATH).join("output").join("rust_location.bin")), // output: &Option<PathBuf>,
        frequency,           // frequency: usize,
        1,                   // bits: usize,
        ionospheric_disable, // ionospheric_disable: bool,
        &path_loss,          // path_loss: &Option<i32>,
        verbose,             // verbose: bool,
    );
    println!("__________________________________________________________");
    vec![
        (params_1b, "rust_1b.bin".to_string(), "c_1b.bin".to_string()),
        (params_8b, "rust_8b.bin".to_string(), "c_8b.bin".to_string()),
        (
            params_16b,
            "rust_16b.bin".to_string(),
            "c_16b.bin".to_string(),
        ),
        (
            params_gga,
            "rust_gga.bin".to_string(),
            "c_gga.bin".to_string(),
        ),
        (
            params_circle,
            "rust_circle.bin".to_string(),
            "c_circle.bin".to_string(),
        ),
        (
            params_circle_llh,
            "rust_circle_llh.bin".to_string(),
            "c_circle_llh.bin".to_string(),
        ),
        (
            params_location,
            "rust_location.bin".to_string(),
            "c_location.bin".to_string(),
        ),
    ]
}
fn prepare() -> Result<()> {
    // Check if executable of gps-sdr-sim exists
    #[cfg(target_os = "windows")]
    let suffix = ".exe";
    #[cfg(not(target_os = "windows"))]
    let suffix = "";
    let gps_sim_executable = PathBuf::from(PATH)
        .join("output")
        .join(&format!("gpssim{}", suffix));
    if !gps_sim_executable.exists() {
        println!("gps-sdr-sim executable not exist, run `gcc gpssim.c -lm -O3 -o gpssim` to build");
        let output = Command::new("gcc")
            .current_dir(PATH)
            .args([
                &format!("{}/resources/gpssim.c", PATH),
                "-lm",
                "-O3",
                "-o",
                &format!("{}/output/gpssim", PATH),
            ])
            .spawn()?
            .wait_with_output()?;
    }
    assert!(
        gps_sim_executable.exists(),
        "gps-sdr-sim executable does not exist"
    );
    // check if output bins of gps-sdr-sim exists
    for (_, _, file_name) in PARAMS.iter() {
        let file_path = PathBuf::from(PATH).join("output").join(file_name);
        if !file_path.exists() {
            match file_name.as_str() {
                "c_1b.bin" => {
                    let output = Command::new(&gps_sim_executable)
                        .current_dir(PATH)
                        .args([
                            "-e",
                            "resources/brdc0010.22n",
                            "-b",
                            "1",
                            "-d",
                            "31",
                            "-o",
                            "output/c_1b.bin",
                        ])
                        .spawn()?
                        .wait_with_output()?;
                }
                "c_8b.bin" => {
                    let output = Command::new(&gps_sim_executable)
                        .current_dir(PATH)
                        .args([
                            "-e",
                            "resources/brdc0010.22n",
                            "-b",
                            "8",
                            "-d",
                            "31",
                            "-o",
                            "output/c_8b.bin",
                        ])
                        .spawn()?
                        .wait_with_output()?;
                }
                "c_16b.bin" => {
                    let output = Command::new(&gps_sim_executable)
                        .current_dir(PATH)
                        .args([
                            "-e",
                            "resources/brdc0010.22n",
                            "-b",
                            "16",
                            "-d",
                            "31",
                            "-o",
                            "output/c_16b.bin",
                        ])
                        .spawn()?
                        .wait_with_output()?;
                }
                "c_gga.bin" => {
                    let output = Command::new(&gps_sim_executable)
                        .current_dir(PATH)
                        .args([
                            "-e",
                            "resources/brdc0010.22n",
                            "-b",
                            "1",
                            "-d",
                            "31",
                            "-o",
                            "output/c_gga.bin",
                            "-g",
                            "resources/triumphv3.txt",
                        ])
                        .spawn()?
                        .wait_with_output()?;
                }
                "c_circle.bin" => {
                    let output = Command::new(&gps_sim_executable)
                        .current_dir(PATH)
                        .args([
                            "-e",
                            "resources/brdc0010.22n",
                            "-b",
                            "1",
                            "-d",
                            "31",
                            "-o",
                            "output/c_circle.bin",
                            "-u",
                            "resources/circle.csv",
                        ])
                        .spawn()?
                        .wait_with_output()?;
                }
                "c_circle_llh.bin" => {
                    let output = Command::new(&gps_sim_executable)
                        .current_dir(PATH)
                        .args([
                            "-e",
                            "resources/brdc0010.22n",
                            "-b",
                            "1",
                            "-d",
                            "31",
                            "-o",
                            "output/c_circle_llh.bin",
                            "-x",
                            "resources/circle_llh.csv",
                        ])
                        .spawn()?
                        .wait_with_output()?;
                }
                "c_location.bin" => {
                    let output = Command::new(&gps_sim_executable)
                        .current_dir(PATH)
                        .args([
                            "-e",
                            "resources/brdc0010.22n",
                            "-b",
                            "1",
                            "-d",
                            "31",
                            "-o",
                            "output/c_location.bin",
                            "-l",
                            "30.286502,120.032669,100",
                        ])
                        .spawn()?
                        .wait_with_output()?;
                }
                filename => {
                    panic!("Not expected file name: {}", file_name)
                }
            }
        }
    }
    Ok(())
}
#[test]
fn test() -> Result<()> {
    prepare()?;

    for (params, rust_filename, c_filename) in PARAMS.iter() {
        process(params.clone());
        let rust_file = PathBuf::from(PATH).join("output").join(rust_filename);
        assert!(rust_file.exists(), "Rust file not exists{}", rust_filename);
        let output = Command::new("diff")
            .current_dir(PathBuf::from(PATH).join("output"))
            .args([rust_filename, c_filename])
            .spawn()?
            .wait_with_output()?;
        let success = output.status.success();
        if success {
            println!("{} and {} are the same!", rust_filename, c_filename);
            std::fs::remove_file(rust_file)?;
        }
        assert!(success, "Files are different! {}", rust_filename);
    }
    Ok(())
}
