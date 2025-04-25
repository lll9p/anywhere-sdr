//! Constants used in GPS signal generation and processing.
//!
//! This crate provides a centralized collection of constants used throughout
//! the GPS signal generation and processing pipeline. It includes physical
//! constants, mathematical constants, GPS system parameters, and configuration
//! values.
//!
//! The constants are organized into logical groups:
//! - Physical constants (speed of light, Earth parameters)
//! - GPS system parameters (frequencies, wavelengths)
//! - Time conversion constants
//! - Mathematical constants and precomputed values
//! - Configuration limits and defaults

#![allow(unused)]
/// Maximum number of user motion records that can be processed
pub const USER_MOTION_SIZE: usize = 3000;

/// Mathematical constant π (pi) with high precision
#[allow(clippy::approx_constant)]
pub const PI: f64 = 3.141_592_653_589_8;

/// Maximum number of satellites in RINEX file
pub const MAX_SAT: usize = 32;

/// Maximum number of satellite channels that can be simulated simultaneously
pub const MAX_CHAN: usize = 16;

/// Maximum duration for static mode simulation in seconds (24 hours)
pub const STATIC_MAX_DURATION: usize = 86400;

/// Number of subframes in a GPS navigation message frame
pub const N_SBF: usize = 5;

/// Number of 30-bit words per subframe in GPS navigation message
pub const N_DWRD_SBF: usize = 10;

/// Total number of words in the navigation message buffer
/// Includes an extra subframe for processing overhead
pub const N_DWRD: usize = (N_SBF + 1) * N_DWRD_SBF;

/// Length of the GPS C/A (Coarse/Acquisition) code sequence in chips
pub const CA_SEQ_LEN: usize = 1023;

/// Floating-point representation of the C/A code sequence length
/// Used for precise calculations involving the code sequence
pub const CA_SEQ_LEN_FLOAT: f64 = CA_SEQ_LEN as f64;

/// Number of seconds in a GPS week (7 days)
pub const SECONDS_IN_WEEK: f64 = 604_800.0;

/// Number of seconds in half a GPS week (3.5 days)
/// Used for time difference calculations to handle week rollover
pub const SECONDS_IN_HALF_WEEK: f64 = 302_400.0;

/// Number of seconds in a day (24 hours)
pub const SECONDS_IN_DAY: f64 = 86400.0;

/// Number of seconds in an hour
pub const SECONDS_IN_HOUR: f64 = 3600.0;

/// Number of seconds in a minute
pub const SECONDS_IN_MINUTE: f64 = 60.0;

/// Precomputed value of 2^(-5) = 0.03125
/// Used for efficient binary scaling operations
pub const POW2_M5: f64 = 0.03125;

/// Precomputed value of 2^(-19)
/// Used in GPS navigation message parameter scaling
pub const POW2_M19: f64 = 1.907_348_632_812_5e-6;

/// Precomputed value of 2^(-29)
/// Used in GPS navigation message parameter scaling
pub const POW2_M29: f64 = 1.862_645_149_230_957e-9;

/// Precomputed value of 2^(-31)
/// Used in GPS navigation message parameter scaling
pub const POW2_M31: f64 = 4.656_612_873_077_393e-10;

/// Precomputed value of 2^(-33)
/// Used in GPS navigation message parameter scaling
pub const POW2_M33: f64 = 1.164_153_218_269_348e-10;

/// Precomputed value of 2^(-43)
/// Used in GPS navigation message parameter scaling
pub const POW2_M43: f64 = 1.136_868_377_216_16e-13;

/// Precomputed value of 2^(-55)
/// Used in GPS navigation message parameter scaling
pub const POW2_M55: f64 = 2.775_557_561_562_891e-17;

/// Precomputed value of 2^(-50)
/// Used in GPS navigation message parameter scaling
pub const POW2_M50: f64 = 8.881_784_197_001_252e-16;

/// Precomputed value of 2^(-30)
/// Used in GPS navigation message parameter scaling
pub const POW2_M30: f64 = 9.313_225_746_154_785e-10;

/// Precomputed value of 2^(-27)
/// Used in GPS navigation message parameter scaling
pub const POW2_M27: f64 = 7.450_580_596_923_828e-9;

/// Precomputed value of 2^(-24)
/// Used in GPS navigation message parameter scaling
pub const POW2_M24: f64 = 5.960_464_477_539_063e-8;

/// Earth's gravitational constant (μ) in m³/s²
/// Standard value from GPS Interface Control Document (ICD-GPS-200)
pub const GM_EARTH: f64 = 3.986_005e14;

/// Earth's rotation rate (ω) in rad/s
/// Standard value from GPS Interface Control Document (ICD-GPS-200)
pub const OMEGA_EARTH: f64 = 7.292_115_146_7e-5;

/// WGS-84 ellipsoid semi-major axis (equatorial radius) in meters
pub const WGS84_RADIUS: f64 = 6_378_137.0;

/// WGS-84 ellipsoid eccentricity (first eccentricity)
/// Defines the flattening of the ellipsoid
pub const WGS84_ECCENTRICITY: f64 = 0.081_819_190_842_6;

/// Conversion factor from radians to degrees (180/π)
/// Used to convert angular measurements
pub const R2D: f64 = 57.295_779_513_1;

/// Speed of light in vacuum in meters per second
pub const SPEED_OF_LIGHT: f64 = 2.997_924_58e8;

/// Reciprocal of speed of light (1/c)
/// Precomputed for efficient time-distance conversions
pub const SPEED_OF_LIGHT_INV: f64 = SPEED_OF_LIGHT.recip();

/// GPS L1 signal wavelength in meters
/// Calculated as c/f where f is the L1 carrier frequency (1575.42 MHz)
pub const LAMBDA_L1: f64 = 0.190_293_672_798_365;

/// Reciprocal of L1 wavelength (1/λ)
/// Precomputed for efficient wavelength-frequency conversions
pub const LAMBDA_L1_INV: f64 = LAMBDA_L1.recip();

/// GPS L1 carrier frequency in Hz (1575.42 MHz)
#[allow(dead_code)]
pub const CARR_FREQ: f64 = 1575.42e6;

/// C/A code chipping rate in Hz (1.023 MHz)
pub const CODE_FREQ: f64 = 1.023e6;

/// Ratio between carrier frequency and code frequency
/// Equal to 1/1540, as the L1 carrier (1575.42 MHz) is 1540 times the C/A code
/// rate (1.023 MHz)
pub const CARR_TO_CODE: f64 = 1.0 / 1540.0;

/// Sampling data format: 1-bit I/Q samples
/// Used for compact file size at the cost of signal quality
pub const SC01: i32 = 1;

/// Sampling data format: 8-bit I/Q samples
/// Balanced compromise between file size and signal quality
pub const SC08: i32 = 8;

/// Sampling data format: 16-bit I/Q samples
/// Highest quality but largest file size
#[allow(dead_code)]
pub const SC16: i32 = 16;

/// Size of the ephemeris array for storing satellite orbit data
/// Set to 15 to accommodate a full day of GPS broadcast ephemeris data
/// Each element represents approximately 1.6 hours of data
pub const EPHEM_ARRAY_SIZE: usize = 15;

/// Default sample rate for simulation updates in seconds (10 Hz)
pub const SAMPLE_RATE: f64 = 0.1;
