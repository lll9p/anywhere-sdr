//! Utility functions for RINEX parsing.
//!
//! This module provides helper functions for parsing numeric values from
//! RINEX navigation files, handling the specific format requirements.

/// Parses a floating-point number from a RINEX file string.
///
/// RINEX files often use 'D' instead of 'E' for scientific notation exponents.
/// This function replaces 'D' with 'E' before parsing.
///
/// # Arguments
/// * `num_string` - The string containing the floating-point number
///
/// # Returns
/// * `Ok(f64)` - The parsed floating-point value
/// * `Err(ParseFloatError)` - If the string cannot be parsed as a float
pub fn parse_rinex_f64(
    num_string: &str,
) -> Result<f64, std::num::ParseFloatError> {
    num_string.replace('D', "E").parse()
}

/// Parses an integer from a RINEX file string.
///
/// # Arguments
/// * `num_string` - The string containing the integer
///
/// # Returns
/// * `Ok(i32)` - The parsed integer value
/// * `Err(ParseIntError)` - If the string cannot be parsed as an integer
pub fn parse_i32(num_string: &str) -> Result<i32, std::num::ParseIntError> {
    num_string.parse()
}
