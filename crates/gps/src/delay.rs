//! Signal propagation delay calculations.
//!
//! This module provides functions to calculate various signal propagation
//! delays that affect GPS signals, such as ionospheric delay.

/// Ionospheric delay calculation implementation
mod ionospheric;
pub use ionospheric::ionospheric_delay;
