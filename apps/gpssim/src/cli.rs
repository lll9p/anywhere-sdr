//! Command-line interface for the GPS signal simulator.
//!
//! This module defines the command-line arguments and options for the
//! application using the clap crate. It provides a compatible interface
//! with the original gps-sdr-sim tool.

use std::path::PathBuf;

use clap::{ArgAction, Parser, ValueEnum};
use gps::{IqBlockSizing, SignalGenerator, SignalGeneratorBuilder};

use crate::{
    Error,
    error::resolve_run_and_finish,
    tx::{FileTxSink, HackrfTxConfig, HackrfTxSink, NullTxSink, TxSink, TxTee},
};

fn parse_coordinate_triplet(value: &str) -> Result<[f64; 3], String> {
    let values = value
        .split(',')
        .map(|component| {
            component.parse::<f64>().map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let actual = values.len();
    values.try_into().map_err(|_| {
        format!("expected exactly 3 coordinate values, got {actual}")
    })
}

/// Transmission output backend selected via `--tx`.
#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum TxBackend {
    /// Transmit generated samples in real time using a `HackRF` device.
    Hackrf,
    /// Discard generated blocks (CPU-only benchmark mode).
    Null,
}

/*

Options:
  -e <gps_nav>     RINEX navigation file for GPS ephemerides (required)
  -u <user_motion> User motion file in ECEF x, y, z format (dynamic mode)
  -x <user_motion> User motion file in latitude/longitude degrees and height meters (dynamic mode)
  -g <nmea_gga>    NMEA GGA stream (dynamic mode)
  -c <location>    ECEF X,Y,Z in meters (static mode) e.g. 3967283.154,1022538.181,4872414.484
  -l <location>    Latitude/longitude degrees and height meters (static mode) e.g. 35.681298,139.766247,10.0
  -L <wnslf,dn,dtslf> User leap future event in GPS week number, day number, next leap second e.g. 2347,3,19
  -t <timestamp>   Scenario RFC 3339 UTC time with an explicit offset, or "now"
  -T               Overwrite TOC and TOE to scenario start time
  -d <duration>    Duration [sec] (dynamic mode max: {}, static mode max: {})
  -o <output>      I/Q sampling data file (default: gpssim.bin)
  -s <frequency>   Sampling frequency [Hz] (default: 2600000)
  -b <iq_bits>     I/Q data format [1/8/16] (default: 16)
  -i               Disable ionospheric delay for spacecraft scenario
  -p [fixed_gain]  Disable path loss and hold power level constant
  -v               Show details about simulated channels
*/
/// Command-line arguments for the GPS signal simulator.
///
/// This struct defines all the command-line options that can be passed to the
/// application. It is designed to be compatible with the original gps-sdr-sim
/// tool's command-line interface.
#[derive(Parser, Debug)]
#[command(term_width = 0)]
#[command(version, about="gps-sdr-sim compatible", long_about = None)]
#[command(propagate_version = true)]
pub struct Args {
    /// Enter interactive terminal UI (CLI args prefill the UI config).
    #[arg(long, default_value_t = false, action = ArgAction::SetTrue)]
    pub(crate) tui: bool,

    /// RINEX navigation file for GPS ephemerides (required unless --tui)
    #[arg(
        short,
        long,
        value_hint = clap::ValueHint::FilePath,
        required_unless_present = "tui"
    )]
    pub(crate) ephemerides: Option<std::path::PathBuf>,

    /// User motion file in ECEF x, y, z format (dynamic mode)
    #[arg(short = 'u', long, value_hint = clap::ValueHint::FilePath)]
    pub(crate) user_motion_ecef: Option<PathBuf>,

    /// User motion file in latitude/longitude degrees and height meters
    /// (dynamic mode)
    #[arg(short = 'x', long, value_hint = clap::ValueHint::FilePath)]
    pub(crate) user_motion_llh: Option<PathBuf>,

    /// NMEA GGA stream (dynamic mode)
    #[arg(short = 'g', long, value_hint = clap::ValueHint::FilePath)]
    pub(crate) nmea_gga: Option<PathBuf>,

    /// ECEF X,Y,Z in meters (static mode) e.g.
    /// 3967283.154,1022538.181,4872414.484
    #[arg(short = 'c', long, value_parser = parse_coordinate_triplet)]
    pub(crate) location_ecef: Option<[f64; 3]>,

    /// Latitude/longitude degrees and height meters (static mode), e.g.
    /// 35.681298,139.766247,10.0
    #[arg(short = 'l', long, value_parser = parse_coordinate_triplet)]
    pub(crate) location: Option<[f64; 3]>,

    /// User leap future event in GPS week number, day number, next leap second
    /// e.g. 2347,3,19
    #[arg(short = 'L', long, value_parser, value_delimiter = ',')]
    pub(crate) leap: Option<Vec<i32>>,

    /// Scenario RFC 3339 UTC time with an explicit offset, or "now"
    #[arg(short = 't', long)]
    pub(crate) time: Option<String>,

    /// Overwrite TOC and TOE to scenario start time
    #[arg(short = 'T', long)]
    pub(crate) time_override: Option<bool>,

    /// Duration in seconds (dynamic mode max: {}, static mode max: {})
    #[arg(short = 'd', long)]
    pub(crate) duration: Option<f64>,

    /// I/Q sampling data file (default: gpssim.bin)
    #[arg(short = 'o', long)]
    pub(crate) output: Option<PathBuf>,

    /// Transmit output backend (repeatable). Example: `--tx hackrf`
    #[arg(long, value_enum, action = ArgAction::Append)]
    pub(crate) tx: Vec<TxBackend>,

    /// Sampling frequency in hertz (default: 2600000)
    #[arg(short = 's', long, default_value_t = 2600000)]
    pub(crate) frequency: usize,

    /// I/Q data format [1/8/16] (default: 16)
    #[arg(short = 'b', long, default_value_t = 16)]
    pub(crate) bits: usize,

    /// Disable ionospheric delay for spacecraft scenario
    #[arg(short = 'i', long, default_value_t = false, action = ArgAction::SetTrue)]
    pub(crate) ionospheric_disable: bool,

    /// Disable path loss and hold power level constant (`fixed_gain`)
    #[arg(short = 'p', long)]
    pub(crate) path_loss: Option<i32>,

    /// Show details about simulated channels
    #[arg(short = 'v', long,default_value_t = false, action = ArgAction::SetTrue)]
    pub(crate) verbose: bool,

    /// `HackRF` serial number (hex). If omitted, uses the first device.
    #[arg(long)]
    pub(crate) hackrf_serial: Option<String>,

    /// `HackRF` RF center frequency in Hz (default: GPS L1)
    #[arg(long, default_value_t = 1_575_420_000)]
    pub(crate) hackrf_rf_freq_hz: u64,

    /// `HackRF` TXVGA gain (0..=47)
    #[arg(long, default_value_t = 20)]
    pub(crate) hackrf_txvga_gain: u16,

    /// Enable `HackRF` RF amplifier
    #[arg(long, default_value_t = false, action = ArgAction::SetTrue)]
    pub(crate) hackrf_amp_enable: bool,

    /// `HackRF` USB bulk transfer size in bytes
    #[arg(long, default_value_t = 256 * 1024)]
    pub(crate) hackrf_usb_transfer_bytes: usize,

    /// `HackRF` number of in-flight USB transfers
    #[arg(long, default_value_t = 16)]
    pub(crate) hackrf_usb_transfers: usize,

    /// `HackRF` bounded queue depth in generator blocks
    #[arg(long, default_value_t = 8)]
    pub(crate) hackrf_queue_blocks: usize,

    /// `HackRF` number of blocks to prefill before TX starts
    #[arg(long, default_value_t = 2)]
    pub(crate) hackrf_prefill_blocks: usize,

    /// If set, do not transmit silence on underrun
    #[arg(long, default_value_t = false, action = ArgAction::SetTrue)]
    pub(crate) hackrf_drop_on_underrun: bool,
}

impl Args {
    /// Runs the GPS signal simulation based on the command-line arguments.
    ///
    /// This method configures the signal generator with the provided options,
    /// initializes it, and runs the simulation.
    ///
    /// # Returns
    /// * `Ok(())` - If the simulation completes successfully
    /// * `Err(Error)` - If an error occurs during simulation
    pub fn run(&self) -> Result<(), Error> {
        let tx_enabled = !self.tx.is_empty();

        let output_path = self.resolve_output_path(tx_enabled);

        let cpu_only_bench = tx_enabled
            && output_path.is_none()
            && !self.tx.is_empty()
            && self.tx.iter().all(|b| matches!(b, TxBackend::Null));

        let mut generator =
            self.build_generator(tx_enabled, output_path.clone())?;
        generator.initialize()?;

        if !tx_enabled {
            generator.run_simulation()?;
            return Ok(());
        }

        let mut tee = TxTee::new(self.build_tx_sinks(&generator, output_path)?);
        if tee.is_empty() {
            return Err(Error::cli_error(
                "no output selected: use -o/--output and/or --tx <backend>"
                    .to_string(),
            ));
        }

        let time_start = std::time::Instant::now();
        let mut blocks: u64 = 0;
        let mut total_samples: u64 = 0;
        let streaming_result = generator.run_streaming::<_, Error>(|block| {
            blocks = blocks.checked_add(1).ok_or_else(|| {
                gps::Error::unsupported_workload(
                    "streaming block count overflow",
                )
            })?;
            let block_samples =
                IqBlockSizing::from_interleaved_i16_len(block.len())?
                    .complex_samples();
            let block_samples = u64::try_from(block_samples).map_err(|_| {
                gps::Error::unsupported_workload(
                    "streaming sample count exceeds supported range",
                )
            })?;
            total_samples =
                total_samples.checked_add(block_samples).ok_or_else(|| {
                    gps::Error::unsupported_workload(
                        "streaming sample count overflow",
                    )
                })?;
            tee.write_block_i16(block)
        });

        let elapsed = time_start.elapsed();

        if streaming_result.is_err() {
            tee.request_cancel();
        }
        let finish_result = tee.finish();
        resolve_run_and_finish(streaming_result, finish_result)?;

        if cpu_only_bench {
            let maximum_samples_per_block = generator.iq_buffer_size as u64;
            let elapsed_seconds = elapsed.as_secs_f64();
            let samples_per_second = if elapsed_seconds > 0.0 {
                total_samples as f64 / elapsed_seconds
            } else {
                0.0
            };

            println!(
                "cpu_bench sample_frequency_hz={} step_seconds={:.6} \
                 blocks={} maximum_samples_per_block={} total_samples={} \
                 elapsed_seconds={:.3} throughput_msps={:.3}",
                generator.sample_frequency,
                generator.sample_rate,
                blocks,
                maximum_samples_per_block,
                total_samples,
                elapsed_seconds,
                samples_per_second / 1_000_000.0,
            );
        }
        Ok(())
    }

    /// Resolves the output file path, preserving the legacy file-default
    /// behavior when no TX backend is selected.
    fn resolve_output_path(&self, tx_enabled: bool) -> Option<PathBuf> {
        if tx_enabled {
            self.output.clone()
        } else {
            self.output
                .clone()
                .or_else(|| Some(PathBuf::from("gpssim.bin")))
        }
    }

    /// Builds and configures the signal generator. In TX mode the generator is
    /// built without a file output writer.
    fn build_generator(
        &self, tx_enabled: bool, output_path: Option<PathBuf>,
    ) -> Result<SignalGenerator, Error> {
        if !tx_enabled && output_path.is_none() {
            return Err(Error::cli_error(
                "no output selected: use -o/--output or --tx <backend>"
                    .to_string(),
            ));
        }

        SignalGeneratorBuilder::default()
            .navigation_file(self.ephemerides.clone())?
            .user_motion_file(self.user_motion_ecef.clone())?
            .user_motion_llh_file(self.user_motion_llh.clone())?
            .user_motion_nmea_gga_file(self.nmea_gga.clone())?
            .location_ecef(self.location_ecef.map(|value| value.to_vec()))?
            .location(self.location.map(|value| value.to_vec()))?
            .leap(self.leap.clone())
            .utc_time(self.time.clone())?
            .time_override(self.time_override)
            .duration(self.duration)
            .output_file(if tx_enabled { None } else { output_path })
            .frequency(Some(self.frequency))?
            .data_format(Some(self.bits))?
            .ionospheric_disable(Some(self.ionospheric_disable))
            .path_loss(self.path_loss)
            .verbose(Some(self.verbose))
            .build()
            .map_err(Into::into)
    }

    /// Builds the list of active TX sinks.
    fn build_tx_sinks(
        &self, generator: &SignalGenerator, output_path: Option<PathBuf>,
    ) -> Result<Vec<Box<dyn TxSink>>, Error> {
        let mut sinks: Vec<Box<dyn TxSink>> = Vec::new();

        if let Some(path) = output_path {
            sinks.push(Box::new(FileTxSink::new(
                path,
                generator.data_format,
                generator.iq_buffer_size,
            )?));
        }

        for backend in &self.tx {
            match backend {
                TxBackend::Hackrf => {
                    sinks.push(Box::new(self.build_hackrf_sink(generator)?));
                }
                TxBackend::Null => {
                    // Avoid duplicating null sinks if the flag is repeated.
                    if !sinks.iter().any(|s| s.backend() == "null") {
                        sinks.push(Box::new(NullTxSink::new()));
                    }
                }
            }
        }

        Ok(sinks)
    }

    /// Validates `HackRF`-specific CLI arguments.
    fn validate_hackrf_args(&self) -> Result<(), Error> {
        if self.hackrf_usb_transfer_bytes == 0 {
            return Err(Error::cli_error(
                "--hackrf-usb-transfer-bytes must be > 0".to_string(),
            ));
        }
        if self.hackrf_usb_transfers == 0 {
            return Err(Error::cli_error(
                "--hackrf-usb-transfers must be > 0".to_string(),
            ));
        }
        if self.hackrf_queue_blocks == 0 {
            return Err(Error::cli_error(
                "--hackrf-queue-blocks must be > 0".to_string(),
            ));
        }
        if self.hackrf_txvga_gain > 47 {
            return Err(Error::cli_error(
                "--hackrf-txvga-gain must be in 0..=47".to_string(),
            ));
        }

        Ok(())
    }

    /// Builds a `HackRF` TX sink configured from CLI + generator settings.
    fn build_hackrf_sink(
        &self, generator: &SignalGenerator,
    ) -> Result<HackrfTxSink, Error> {
        self.validate_hackrf_args()?;

        let config = HackrfTxConfig {
            serial: self.hackrf_serial.clone(),
            rf_freq_hz: self.hackrf_rf_freq_hz,
            sample_frequency_hz: generator.sample_frequency,
            step_duration: std::time::Duration::from_secs_f64(
                generator.sample_rate,
            ),
            txvga_gain: self.hackrf_txvga_gain,
            amp_enable: self.hackrf_amp_enable,
            usb_transfer_bytes: self.hackrf_usb_transfer_bytes,
            usb_transfers: self.hackrf_usb_transfers,
            queue_blocks: self.hackrf_queue_blocks,
            prefill_blocks: self.hackrf_prefill_blocks,
            silence_on_underrun: !self.hackrf_drop_on_underrun,
            underrun_counter: None,
        };

        let expected_i16_len =
            IqBlockSizing::new(generator.iq_buffer_size)?.interleaved_i16_len();
        HackrfTxSink::new(config, expected_i16_len)
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{Args, TxBackend};
    use crate::tui_config::TuiConfig;

    #[test]
    fn parses_tui_without_ephemerides() -> Result<(), clap::Error> {
        let args = Args::try_parse_from(["gpssim", "--tui"])?;
        assert!(args.tui);
        assert!(args.ephemerides.is_none());

        assert!(Args::try_parse_from(["gpssim"]).is_err());
        Ok(())
    }

    #[test]
    fn static_coordinates_require_exactly_three_values() -> Result<(), String> {
        for option in ["--location", "--location-ecef"] {
            for values in [None, Some("1"), Some("1,2"), Some("1,2,3,4")] {
                let mut arguments = vec!["gpssim", "--tui", option];
                if let Some(values) = values {
                    arguments.push(values);
                }
                if Args::try_parse_from(arguments).is_ok() {
                    return Err(format!(
                        "{option} unexpectedly accepted {values:?}"
                    ));
                }
            }

            Args::try_parse_from(["gpssim", "--tui", option, "1,2,3"])
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    #[test]
    fn applies_cli_overrides_to_tui_config()
    -> Result<(), Box<dyn std::error::Error>> {
        let args = Args::try_parse_from([
            "gpssim", "--tui", "--tx", "null", "-s", "123", "-b", "8",
        ])?;

        let mut config = TuiConfig::default();
        config.apply_overrides_from_args(&args)?;

        assert_eq!(config.tx, vec![TxBackend::Null]);
        assert_eq!(config.frequency, 123);
        assert_eq!(config.bits, 8);
        Ok(())
    }

    #[test]
    fn tui_ecef_override_propagates_geometry_errors()
    -> Result<(), Box<dyn std::error::Error>> {
        let args = Args::try_parse_from([
            "gpssim",
            "--tui",
            "--location-ecef",
            "NaN,0,0",
        ])?;
        let mut config = TuiConfig::default();
        assert!(matches!(
            config.apply_overrides_from_args(&args),
            Err(crate::Error::Geometry(geometry::Error::InvalidEcef { .. }))
        ));
        Ok(())
    }
}
