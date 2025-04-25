/*!
 * # `LibHackRF`
 *
 * A Rust library for interfacing with `HackRF` software-defined radio
 * devices.
 *
 * This crate provides a safe and idiomatic Rust interface to `HackRF`
 * devices, allowing for configuration, control, and data transfer
 * operations.
 *
 * ## Origin
 *
 * This library is a modified version of [libhackrf-rs](https://github.com/fl1ckje/libhackrf-rs)
 * with the main change being the replacement of the `rusb` dependency with
 * `nusb` for improved USB communication. Additional improvements include
 * comprehensive documentation, error handling with `thiserror`, and code
 * optimizations.
 *
 * ## Features
 *
 * - Device discovery and connection
 * - Configuration of radio parameters (frequency, sample rate, gain, etc.)
 * - Transmission and reception of radio signals
 * - Error handling with specific error types
 *
 * ## Example
 *
 * ```rust,no_run
 * use libhackrf::prelude::*;
 *
 * fn main() -> Result<(), Error> {
 *     // Open the first available HackRF device
 *     let mut sdr = HackRF::new_auto()?;
 *
 *     // Configure the device
 *     sdr.set_freq(915_000_000)?; // Set frequency to 915 MHz
 *     sdr.set_sample_rate_auto(10.0e6)?; // Set sample rate to 10 MHz
 *
 *     // Print device information
 *     println!("Board ID: {}", sdr.board_id()?);
 *     println!("Firmware version: {}", sdr.version()?);
 *
 *     Ok(())
 * }
 * ```
 */

pub mod constants;
pub mod enums;
pub mod error;
pub mod hackrf;
pub mod prelude;
#[cfg(test)]
mod tests;
