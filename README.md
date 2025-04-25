# GPS Signal Generator

A software-defined GPS signal simulator written in Rust, inspired by [gps-sdr-sim](https://github.com/osqzss/gps-sdr-sim). It generates GPS L1 C/A signals that can be transmitted through SDR devices.

## Table of Contents

- [GPS Signal Generator](#gps-signal-generator)
  - [Table of Contents](#table-of-contents)
  - [Features](#features)
  - [Installation](#installation)
  - [Usage](#usage)
    - [Command Line Usage](#command-line-usage)
    - [Library Usage](#library-usage)
    - [Command Line Options](#command-line-options)
    - [Usage Examples](#usage-examples)
  - [Direct Sample Access API](#direct-sample-access-api)
  - [Testing](#testing)
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

> [!NOTE]
> This project is still under development.
>
> The project is compatible with [gps-sdr-sim](https://github.com/osqzss/gps-sdr-sim) for all core features, with some parameter handling improvements.
> Future versions will extend beyond [gps-sdr-sim](https://github.com/osqzss/gps-sdr-sim) as we implement new features and improvements.

## Features

- **Signal Generation**: GPS L1 C/A signals with configurable parameters
- **Position Modes**:
  - Static positioning with ECEF or LLH coordinates
  - Dynamic trajectories from motion files or NMEA streams
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

```rust
use std::path::PathBuf;
use gps::SignalGeneratorBuilder;

// Configure the signal generator
let builder = SignalGeneratorBuilder::default()
    .navigation_file(Some(PathBuf::from("brdc0010.22n"))).unwrap()
    .location(Some(vec![35.6813, 139.7662, 10.0])).unwrap()
    .duration(Some(60.0))
    .data_format(Some(8)).unwrap()
    .ionospheric_disable(Some(true))  // Disable ionospheric delay correction
    .output_file(Some(PathBuf::from("output.bin")));

// Build and run the generator
let mut generator = builder.build().unwrap();
generator.initialize().unwrap();
generator.run_simulation().unwrap();
```

### Command Line Options

- `-e <gps_nav>`: RINEX navigation file for GPS ephemerides (required)
- `-u <user_motion>`: User motion file in ECEF x,y,z format (dynamic mode)
- `-x <user_motion>`: User motion file in lat,lon,height format (dynamic mode)
- `-g <nmea_gga>`: NMEA GGA stream (dynamic mode)
- `-c <location>`: ECEF X,Y,Z in meters (static mode) e.g. 3967283.154,1022538.181,4872414.484
- `-l <location>`: Lat,lon,height (static mode) e.g. 35.681298,139.766247,10.0
- `-t <date,time>`: Scenario start time YYYY/MM/DD,hh:mm:ss or "now" for current time
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
```

## Direct Sample Access API

The library provides an API for direct sample access without file I/O. This allows integration with other applications or real-time processing:

```rust
// After initializing the generator
let mut generator = builder.build().unwrap();
generator.initialize().unwrap();

// Instead of run_simulation(), you can process each step individually
// and access the generated samples directly
for step in 0..num_steps {
    // Update satellite parameters for current position
    generator.update_channel_parameters(current_position);

    // Generate samples for this step
    generator.generate_samples();

    // Access the sample buffer directly
    let samples = generator.get_sample_buffer();

    // Process samples as needed...
}
```

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

Note: Some tests (like leap second handling and time override) don't compare binary outputs directly due to slight implementation differences between the Rust and C versions.

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

This project is inspired by the original [gps-sdr-sim](https://github.com/osqzss/gps-sdr-sim) project and aims to provide a modern Rust implementation with improved performance, maintainability, and extensibility.

The `libhackrf` crate used in this project is a modified version of [libhackrf-rs](https://github.com/fl1ckje/libhackrf-rs), with the main change being the replacement of the `rusb` dependency with `nusb` for improved USB communication. Additional improvements include comprehensive documentation, error handling with `thiserror`, and code optimizations.
