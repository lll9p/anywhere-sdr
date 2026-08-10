<!-- markdownlint-disable -->
<br />
<div align="center">
  <h1>
    Anywhere-SDR, a GPS Signal Generator
 </h1>
  <p align="center">
    <a href="https://github.com/lll9p/anywhere-sdr/discussions/new?category=ideas">Request Feature</a>
    ·
    <a href="https://github.com/lll9p/anywhere-sdr/discussions/new?category=q-a">Ask Question</a>
  </p>
  <p>
    <strong>
A software-defined GPS signal simulator written in <a href="https://www.rust-lang.org/">Rust</a>, inspired by <a href="https://github.com/osqzss/gps-sdr-sim">gps-sdr-sim</a>.
</strong>
<br />
It generates GPS L1 C/A signals that can be transmitted through SDR devices.
  </p>
  <p>🦀</p>

[![Rust][rust-shield]][rust-url]
[![MIT License][license-shield]][license-url]
[![Issues][issues-shield]][issues-url]
[![Build Status][ci-shield]][ci-url]
[![Latest Release][release-shield]][release-url]
[![PayPal][paypal-shield]][paypal-donations-url]

</div>
<!-- markdownlint-restore -->

## Legal Disclaimer

> [!WARNING]
> **Legal Disclaimer**
>
> This project is intended for research and educational purposes only. Users must comply with all applicable laws and regulations in their jurisdiction. Unauthorized transmission of GPS signals may be illegal in certain jurisdictions. It is the user's responsibility to understand and comply with local regulations.
>
> The authors and contributors of this project accept no legal liability for any illegal actions or damages resulting from the use of this software.

## Project Status

> [!NOTE]
> This project is still under development.
>
> The project is compatible with [gps-sdr-sim][gps-sdr-sim-url] for all core features, with some parameter handling improvements.
>
> Future versions will extend beyond [gps-sdr-sim][gps-sdr-sim-url] as we implement new features and improvements.

## Table of Contents

- [Legal Disclaimer](#legal-disclaimer)
- [Project Status](#project-status)
- [Table of Contents](#table-of-contents)
- [Features](#features)
- [Installation](#installation)
- [Usage](#usage)
  - [Command Line Usage](#command-line-usage)
  - [Library Usage](#library-usage)
  - [Command Line Options](#command-line-options)
  - [Usage Examples](#usage-examples)
- [Direct Sample Access API](#direct-sample-access-api)
- [Runtime Motion Control API](#runtime-motion-control-api)
- [Testing](#testing)
  - [Hardware-Dependent Tests](#hardware-dependent-tests)
  - [Compatibility Tests](#compatibility-tests)
  - [Completed Features](#completed-features)
- [License](#license)
- [Contributing](#contributing)
- [Authors](#authors)
- [Roadmap](#roadmap)
  - [Upcoming Features](#upcoming-features)
    - [Signal Generation](#signal-generation)
    - [Input/Output](#inputoutput)
    - [Error Handling \& Performance](#error-handling--performance)
- [Acknowledgments](#acknowledgments)

## Features

- **Signal Generation**: GPS L1 C/A signals with configurable parameters
- **Position Modes**:
  - Static positioning with ECEF or LLH coordinates
  - Dynamic trajectories from motion files or NMEA streams
  - Runtime motion control (library API)
- **Input Formats**:
  - RINEX navigation files for GPS ephemerides
  - User motion in ECEF (X,Y,Z) format
  - User motion in LLH (Latitude, Longitude, Height) format
  - NMEA GGA streams
- **Output Options**:
  - Multiple I/Q data formats (1-bit, 8-bit, 16-bit)
  - Configurable sampling frequency
  - File output or direct buffer access via API
- **Signal Modeling**:
  - Ionospheric delay correction (can be disabled with `-i` flag)
  - Path loss simulation with configurable gain

## Installation

This project is not yet published to crates.io. To use it, clone the repository and build it locally:

```bash
git clone https://github.com/lll9p/anywhere-sdr
cd anywhere-sdr
cargo build --release
```

## Usage

### Command Line Usage

Basic usage example:

```bash
gpssim -e brdc0010.22n -l 35.681298,139.766247,10.0 -d 30
```

### Library Usage

```rust,no_run
use std::path::PathBuf;

use gps::SignalGeneratorBuilder;

fn main() -> Result<(), gps::Error> {
    let mut generator = SignalGeneratorBuilder::default()
        .navigation_file(Some(PathBuf::from("brdc0010.22n")))?
        .location(Some(vec![35.6813, 139.7662, 10.0]))?
        .duration(Some(60.0))
        .data_format(Some(8))?
        .ionospheric_disable(Some(true))
        .output_file(Some(PathBuf::from("output.bin")))
        .build()?;

    generator.initialize()?;
    generator.run_simulation()?;
    Ok(())
}
```

### Command Line Options

- `--tui`: Launch the interactive terminal UI. The Config tab projects every effective configuration field and marks each one as editable, toggleable, clearable, or a read-only CLI prefill before starting.
- `-e <gps_nav>`: RINEX navigation file for GPS ephemerides (required)
- `-u <user_motion>`: User motion file in ECEF x,y,z format (dynamic mode)
- `-x <user_motion>`: User motion file in latitude/longitude degrees and height meters (dynamic mode)
- `-g <nmea_gga>`: NMEA GGA stream (dynamic mode)
- `-c <location>`: ECEF X,Y,Z in meters (static mode) e.g. 3967283.154,1022538.181,4872414.484
- `-l <location>`: Latitude/longitude degrees and height meters (static mode), e.g. 35.681298,139.766247,10.0
- `-t <timestamp>`: Scenario RFC 3339 UTC start time with an explicit offset (for example, `2026-07-14T00:00:00Z`) or `now`
- `-T`: Overwrite TOC and TOE to scenario start time
- `-d <duration>`: Duration in seconds
- `-o <output>`: I/Q sampling data file (default: gpssim.bin)
- `-s <frequency>`: Sampling frequency in Hz (default: 2600000)
- `-b <iq_bits>`: I/Q data format [1/8/16] (default: 16)
- `-i`: Disable ionospheric delay correction (useful for spacecraft scenarios)
- `-p [fixed_gain]`: Disable path loss and hold power level constant
- `-v`: Show details about simulated channels

### Usage Examples

```bash
# Generate signal with 8-bit I/Q format for a static location
gpssim -e brdc0010.22n -b 8 -d 60.0 -l 35.681298,139.766247,10.0 -o output.bin

# Interactive mode (CLI flags prefill the visible effective configuration)
gpssim --tui -e brdc0010.22n -l 35.681298,139.766247,10.0 -d 30

# Generate signal using NMEA GGA stream for dynamic motion
gpssim -e brdc0010.22n -d 120.0 -g nmea_data.txt -s 2600000

# Generate signal with custom sampling frequency and fixed gain
gpssim -e brdc0010.22n -d 30.0 -s 2000000 -p 63 -c -3813477.954,3554276.552,3662785.237

# Generate signal with current time
gpssim -e brdc0010.22n -d 30.0 -t now -T -l 35.681298,139.766247,10.0

# Generate signal with leap second parameters
gpssim -e brdc0010.22n -d 30.0 -L 2347,3,17 -l 42.3569048,-71.2564075,0

# Generate signal with ionospheric delay correction disabled
gpssim -e brdc0010.22n -d 30.0 -i -l 35.681298,139.766247,10.0

# Transmit in real time via HackRF (SC8)
# Note (Windows): HackRF must be bound to WinUSB (e.g. via Zadig) for `nusb`.
gpssim -e brdc0010.22n -l 35.681298,139.766247,10.0 -d 30 \
  --tx hackrf \
  --hackrf-rf-freq-hz 1575420000 \
  --hackrf-txvga-gain 20

# Transmit via HackRF and also write an 8-bit SC8 file
gpssim -e brdc0010.22n -l 35.681298,139.766247,10.0 -d 30 -b 8 -o gpssim_sc8.bin \
  --tx hackrf
```

### Interactive TUI Controls

The Config tab exposes the complete effective configuration, including CLI
prefills, and labels every field with its available interaction. Select manual
motion before starting to enable live receiver control.

On the Run tab while a manual run is active, use Left/Right to change heading,
Up/Down to change speed, Space to stop, Enter to resume the retained cruise
speed, and `c` or Esc to cancel the run. These controls are available only while
the run is in the Running state; the first terminal worker outcome removes them.
On a normal TUI exit, the latest typed run failure is returned to the process
and produces a nonzero exit status. No run, a finished run, or a cleanly
cancelled run exits successfully.

## Direct Sample Access API

The library provides an API for direct sample access without file I/O. This allows integration with other applications or real-time processing:

```rust,no_run
use std::path::PathBuf;

use gps::SignalGeneratorBuilder;

fn process_iq_block(_interleaved_iq: &[i16]) {
    // Forward the interleaved I/Q block to your application.
}

fn main() -> Result<(), gps::Error> {
    let mut generator = SignalGeneratorBuilder::default()
        .navigation_file(Some(PathBuf::from("brdc0010.22n")))?
        .location(Some(vec![35.6813, 139.7662, 10.0]))?
        .duration(Some(1.0))
        .frequency(Some(2_600_000))?
        .data_format(Some(16))?
        .output_file(None)
        .build()?;

    generator.initialize()?;
    generator.run_streaming::<_, gps::Error>(|interleaved_iq| {
        process_iq_block(interleaved_iq);
        Ok(())
    })?;
    Ok(())
}
```

## Runtime Motion Control API

The library supports a runtime motion controller that lets you change receiver
motion while streaming (heading/speed/acceleration, stop/start, and
target-tracking with limits).

```rust,no_run
use std::{error::Error as StdError, fmt, path::PathBuf};

use geometry::Ecef;
use gps::{MotionCommand, RuntimeMotionControl, SignalGeneratorBuilder};

#[derive(Debug)]
enum StreamError {
    Generator(gps::Error),
    StopRequested,
}

impl fmt::Display for StreamError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Generator(error) => write!(formatter, "generator failed: {error}"),
            Self::StopRequested => formatter.write_str("example stop requested"),
        }
    }
}

impl StdError for StreamError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Generator(error) => Some(error),
            Self::StopRequested => None,
        }
    }
}

impl From<gps::Error> for StreamError {
    fn from(error: gps::Error) -> Self {
        Self::Generator(error)
    }
}

fn main() -> Result<(), StreamError> {
    let origin = Ecef::new(-3_813_477.954, 3_554_276.552, 3_662_785.237);
    let control = RuntimeMotionControl::new(origin);

    let mut generator = SignalGeneratorBuilder::default()
        .navigation_file(Some(PathBuf::from("brdc0010.22n")))?
        .runtime_motion_control(Some(control.clone()))?
        .frequency(Some(2_600_000))?
        .data_format(Some(16))?
        .output_file(None)
        .build()?;
    generator.initialize()?;

    control.submit(MotionCommand::SetHeadingSpeed {
        heading_deg: 90.0,
        speed_mps: 10.0,
        climb_mps: 0.0,
    })?;

    let mut block_count = 0usize;
    match generator.run_streaming_user_control::<_, StreamError>(|_iq| {
        block_count += 1;
        let _snapshot = control.try_snapshot();

        if block_count >= 3 {
            return Err(StreamError::StopRequested);
        }

        Ok(())
    }) {
        Err(StreamError::StopRequested) => Ok(()),
        result => result,
    }
}
```

Notes and constraints:

- Commands take effect at actual emitted-sample simulation-step boundaries
  (`dt = sample_rate`, default 0.1s).
- GPSsim's TUI manual mode uses this API for live heading, speed, stop, cruise,
  and cancellation controls.
- Some parts of the codebase still assume a ~10 Hz step rate; changing
  `sample_rate` may require additional work.
- The streaming loop runs until the callback returns an error. Use a dedicated
  callback error for intentional cancellation and match it exactly so generator
  failures remain distinct and propagate.
- The generator hot path uses non-blocking reads for pending commands and
  snapshots. Pending storage is bounded to one command per variant. A
  successful replacement removes the older command and moves that variant to
  the ordered tail; retained variants replay in global oldest-to-newest order
  by each variant's last successful submission.
- Numeric command fields must be finite. Horizontal/start/target speeds and
  acceleration/turn-rate limits must also be nonnegative. A zero target limit
  freezes that transition while keeping the target active; the TUI continues
  to require positive configured limits.

Extension points:

- Timestamped commands (apply at a specific simulation time)
- Waypoint / autopilot commands (drive `SetTargetHeading`/`SetTargetSpeed`)
- Jerk-limited motion models and alternative integrators

## Testing

Run the standard test suite:

```bash
cargo test
```

The integration tests in `@crates/gps/tests/test-generator.rs` only run in release mode and compare output with the original C implementation:

```bash
cargo test --release
```

To run specific test cases:

```bash
# Run a specific test by name
cargo test --release -p gps --test test-generator test_data_format_1bit

# Run all tests related to sampling frequency
cargo test --release -p gps --test test-generator test_sampling_frequency
```

### Hardware-Dependent Tests

Some tests in the `libhackrf` crate require physical `HackRF` hardware to be connected. These tests are marked with `#[ignore]` to prevent them from running during normal test execution. To run these tests when hardware is available:

```bash
# Run all hardware-dependent tests
cargo test -p libhackrf -- --ignored

# Run a specific hardware-dependent test
cargo test -p libhackrf list_device -- --ignored
```

The `gpssim` application also includes an ignored HackRF TX smoke test. To run it:

```bash
cargo test -p gpssim -- --ignored
```

### Compatibility Tests

The following compatibility tests have been implemented and verified:

- Data format tests (1-bit, 8-bit, 16-bit)
- Custom sampling frequency (1MHz, 2MHz, 5MHz)
- NMEA GGA stream input
- Circular motion trajectory (ECEF and LLH formats)
- Static location (lat/lon/height and ECEF coordinates)
- Fixed gain (path loss disabled)
- Custom date/time setting
- Date/time override functionality
- Leap second handling
- Ionospheric delay disable
- Verbose output mode
- Different simulation durations
- Parameter combinations (location + frequency + bit format, etc.)

### Completed Features

All core features have been implemented, including:

- Date/time override functionality (`-T` flag)
- Leap second handling (`-L` flag)
- ECEF coordinate parsing (`-c` parameter)
- Ionospheric delay correction (with `-i` flag to disable)
- Comprehensive test suite with meaningful test cases

## License

See the LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Authors

- Lao Lilin <lll9p.china@gmail.com>

## Roadmap

### Upcoming Features

#### Signal Generation

- [x] GPS L1 C/A signal generation
- [x] Static position simulation
- [x] Dynamic trajectory simulation
- [ ] Advanced position movement (acceleration, jerk control)
- [ ] Support for additional GNSS systems (Galileo, BeiDou, GLONASS)

#### Input/Output

- [x] RINEX navigation file support
- [x] User motion file support (ECEF and LLH formats)
- [x] NMEA GGA stream support
- [x] Direct sample access API
- [ ] Real-time streaming output
- [ ] Direct SDR hardware integration

#### Error Handling & Performance

- [x] Implement error handling with thiserror
- [x] Optimize critical path performance
- [ ] Multi-threaded signal generation

## Acknowledgments

This project is inspired by the original [gps-sdr-sim][gps-sdr-sim-url] project and aims to provide a modern Rust implementation with improved performance, maintainability, and extensibility.

The `libhackrf` crate used in this project is a modified version of [libhackrf-rs][libhackrf-rs-url], with the main change being the replacement of the `rusb` dependency with `nusb` for improved USB communication. Additional improvements include comprehensive documentation, error handling with `thiserror`, and code optimizations.

<!-- markdownlint-disable -->
<!-- prettier-ignore-end -->

<!-- MARKDOWN LINKS & IMAGES -->
[gps-sdr-sim-url]: https://github.com/osqzss/gps-sdr-sim
[libhackrf-rs-url]: https://github.com/fl1ckje/libhackrf-rs

[rust-shield]: https://img.shields.io/badge/rustc-1.86.0+-green.svg?style=for-the-badge
[rust-url]: https://www.rust-lang.org/

[license-shield]: https://img.shields.io/github/license/lll9p/anywhere-sdr.svg?style=for-the-badge
[license-url]: https://github.com/lll9p/anywhere-sdr/blob/master/LICENSE

[issues-shield]: https://img.shields.io/github/issues/lll9p/anywhere-sdr.svg?style=for-the-badge
[issues-url]: https://github.com/lll9p/anywhere-sdr/issues

[ci-shield]: https://img.shields.io/github/actions/workflow/status/lll9p/anywhere-sdr/ci.yaml?style=for-the-badge
[ci-url]: https://github.com/lll9p/anywhere-sdr/actions/workflows/ci.yaml

[release-shield]: https://img.shields.io/github/v/release/lll9p/anywhere-sdr?include_prereleases&sort=semver&style=for-the-badge
[release-url]: https://github.com/lll9p/anywhere-sdr/releases

[paypal-shield]: https://img.shields.io/badge/paypal-donate-green.svg?style=for-the-badge
[paypal-donations-url]: https://paypal.me/laolilin
