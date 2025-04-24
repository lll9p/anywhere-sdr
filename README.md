# GPS Signal Generator

A software-defined GPS signal simulator written in Rust, inspired by [gps-sdr-sim](https://github.com/osqzss/gps-sdr-sim). It generates GPS L1 C/A signals that can be transmitted through SDR devices.

> [!NOTE]
> This project is still under development.
>
> Currently, it's partially compatible with [gps-sdr-sim](https://github.com/osqzss/gps-sdr-sim), but full compatibility is not guaranteed due to parameter parsing bugs in the original project.
> Future versions will diverge from [gps-sdr-sim](https://github.com/osqzss/gps-sdr-sim) as we implement new features and improvements.

## Features

- Generates GPS L1 C/A signals
- Supports both static and dynamic scenarios
- Compatible with various input formats:
  - RINEX navigation files for GPS ephemerides
  - User motion in ECEF (X,Y,Z) format
  - User motion in LLH (Latitude, Longitude, Height) format
  - NMEA GGA streams
- Configurable I/Q sampling parameters
- Supports different output formats (1-bit, 8-bit, 16-bit)
- Ionospheric delay modeling (can be disabled)
- Path loss simulation with configurable gain

## Installation

Since this project is not yet published to crates.io, you can install it directly from the GitHub repository:

```bash
cargo install --git https://github.com/lll9p/anywhere-sdr
```

Or clone the repository and build it locally:

```bash
git clone https://github.com/lll9p/anywhere-sdr
cd anywhere-sdr
cargo install --path .
```

## Usage

Basic usage example:

```bash
gpssim -e brdc0010.22n -l 35.681298,139.766247,10.0 -d 30
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

Generate signal with different I/Q data formats (1-bit, 8-bit, or 16-bit):

```bash
gpssim -e brdc0010.22n -b 1 -d 31.0 -o output_1bit.bin
```

Generate signal for a static location using lat/lon/height:

```bash
gpssim -e brdc0010.22n -b 16 -d 31.0 -l 30.286502,120.032669,100
```

Generate signal using NMEA GGA stream for dynamic motion:

```bash
gpssim -e brdc0010.22n -d 31.0 -g nmea_data.txt
```

Generate signal with custom sampling frequency and fixed gain:

```bash
gpssim -e brdc0010.22n -d 31.0 -s 2000000 -p 63
```

## Building from Source

```bash
git clone https://github.com/lll9p/anywhere-sdr
cd anywhere-sdr
cargo build --release
```

## Testing

Run the test suite:

```bash
cargo test
```

Note that the integration tests in `@crates/gps/tests/test-generator.rs` only run in release mode. To run these tests, use:

```bash
cargo test --release
```

These tests are designed to verify that our implementation produces output identical to the original [gps-sdr-sim](https://github.com/osqzss/gps-sdr-sim) project, ensuring compatibility where needed.

### Completed Tests

The following tests have been implemented and verified:

- Data format tests (1-bit, 8-bit, 16-bit)
- Custom sampling frequency
- NMEA GGA stream input
- Circular motion trajectory (ECEF format)
- Circular motion trajectory (LLH format)
- Static location (lat/lon/height)
- Static location (ECEF coordinates)
- Fixed gain (path loss disabled)
- Custom date/time setting

### Known Failing Tests

The following tests are currently failing and marked for future fixes:

- Date/time override functionality (`-T` flag)
- Leap second handling (`-L` flag)

## License

See the LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Authors

- Lao Lilin <lll9p.china@gmail.com>

## Roadmap

The following features are planned for future releases:

### Signal Generation Features

- [x] GPS L1 C/A signal generation
- [x] Static position simulation
- [x] Dynamic trajectory simulation
- [ ] Advanced position movement capabilities (acceleration, jerk control)

### Input/Output Enhancements

- [x] RINEX navigation file support
- [x] User motion file support (ECEF and LLH formats)
- [x] NMEA GGA stream support
- [ ] Real-time streaming output
- [ ] Direct SDR hardware integration

## Acknowledgments

This project is inspired by the original [gps-sdr-sim](https://github.com/osqzss/gps-sdr-sim) project and aims to provide a Rust implementation with improved performance and maintainability.
