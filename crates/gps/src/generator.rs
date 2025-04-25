//! GPS signal generator implementation.
//!
//! This module provides the core functionality for generating GPS L1 C/A
//! signals. It includes the builder pattern for configuring the signal
//! generator and the main signal generation implementation.

/// Builder pattern implementation for signal generator configuration
mod builder;
/// Core signal generation implementation
mod signal_generator;
/// Utility functions and types for signal generation
mod utils;

pub use builder::SignalGeneratorBuilder;
pub use signal_generator::SignalGenerator;
pub use utils::MotionMode;
