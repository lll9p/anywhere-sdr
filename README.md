# GPS Signal Generator

A software-defined GPS signal simulator written in Rust, inspired by [gps-sdr-sim](https://github.com/osqzss/gps-sdr-sim). It generates GPS L1 C/A signals that can be transmitted through SDR devices.

> [!NOTE]
> This project is still under development.
>
> Currently, it's partially compatible with [gps-sdr-sim](https://github.com/osqzss/gps-sdr-sim), but full compatibility is not guaranteed due to parameter parsing bugs in the original project.
> Future versions will diverge from [gps-sdr-sim](https://github.com/osqzss/gps-sdr-sim) as we implement new features and improvements.

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
  - Ionospheric delay (can be disabled)
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
- `-t <date,time>`: Scenario start time YYYY/MM/DD,hh:mm:ss
- `-T`: Overwrite TOC and TOE to scenario start time
- `-d <duration>`: Duration in seconds
- `-o <output>`: I/Q sampling data file (default: gpssim.bin)
- `-s <frequency>`: Sampling frequency in Hz (default: 2600000)
- `-b <iq_bits>`: I/Q data format [1/8/16] (default: 16)
- `-i`: Disable ionospheric delay for spacecraft scenario
- `-p [fixed_gain]`: Disable path loss and hold power level constant
- `-v`: Show details about simulated channels

### Usage Examples

```bash
# Generate signal with 8-bit I/Q format for a static location
gpssim -e brdc0010.22n -b 8 -d 60.0 -l 35.681298,139.766247,10.0 -o output.bin

# Generate signal using NMEA GGA stream for dynamic motion
gpssim -e brdc0010.22n -d 120.0 -g nmea_data.txt -s 2600000

# Generate signal with custom sampling frequency and fixed gain
gpssim -e brdc0010.22n -d 30.0 -s 2000000 -p 63 -c 3967283.154,1022538.181,4872414.484
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

### Compatibility Tests

The following compatibility tests have been implemented and verified:

- Data format tests (1-bit, 8-bit, 16-bit)
- Custom sampling frequency
- NMEA GGA stream input
- Circular motion trajectory (ECEF and LLH formats)
- Static location (lat/lon/height and ECEF coordinates)
- Fixed gain (path loss disabled)
- Custom date/time setting

### Known Issues

The following features are currently being worked on:

- Date/time override functionality (`-T` flag)
- Leap second handling (`-L` flag)

## License

See the LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Authors

- Lao Lilin <lll9p.china@gmail.com>

## Roadmap

### Upcoming Features

- **Signal Generation**
  - [x] GPS L1 C/A signal generation
  - [x] Static position simulation
  - [x] Dynamic trajectory simulation
  - [ ] Advanced position movement (acceleration, jerk control)
  - [ ] Support for additional GNSS systems (Galileo, BeiDou, GLONASS)

- **Input/Output**
  - [x] RINEX navigation file support
  - [x] User motion file support (ECEF and LLH formats)
  - [x] NMEA GGA stream support
  - [x] Direct sample access API
  - [ ] Real-time streaming output
  - [ ] Direct SDR hardware integration

- **Error Handling & Performance**
  - [x] Implement error handling with thiserror
  - [x] Optimize critical path performance
  - [ ] Multi-threaded signal generation

## Acknowledgments

This project is inspired by the original [gps-sdr-sim](https://github.com/osqzss/gps-sdr-sim) project and aims to provide a modern Rust implementation with improved performance, maintainability, and extensibility.
