//! GPS L1 C/A signal generator and simulator for software-defined radio
//! applications.
//!
//! This crate provides functionality to generate GPS L1 C/A signals that can be
//! transmitted through software-defined radio devices. It simulates GPS
//! satellite signals based on ephemeris data from RINEX navigation files and
//! user-defined receiver positions.
//!
//! The main entry point is the `SignalGeneratorBuilder` which allows
//! configuring all aspects of the simulation before generating the signal with
//! `SignalGenerator`.

/// GPS channel simulation and signal generation
mod channel;
/// GPS time system representation and utilities
mod datetime;
/// Signal propagation delay calculations
mod delay;
/// GPS satellite ephemeris processing
mod ephemeris;
/// Error types for GPS signal generation
mod error;
/// Main signal generator implementation
mod generator;
/// I/Q data format handling and file I/O
mod io;
/// Ionospheric and UTC parameter handling
mod ionoutc;
/// Satellite position and velocity propagation
mod propagation;
/// Lookup tables for signal generation
mod table;

pub use error::Error;
pub use generator::{MotionMode, SignalGenerator, SignalGeneratorBuilder};
pub use io::DataFormat;
