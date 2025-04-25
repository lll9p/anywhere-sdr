//! Re-exports the most commonly used types and functions from the crate
//!
//! This module provides a convenient way to import all the essential
//! components of the `libhackrf` crate with a single import statement.
//!
//! # Example
//!
//! ```rust,no_run
//! use libhackrf::prelude::*;
//!
//! fn main() -> Result<(), Error> {
//!     let sdr = HackRF::new_auto()?;
//!     // Now you can use HackRF, Error, and all constants and enums
//!     Ok(())
//! }
//! ```

pub use crate::{constants::*, enums::*, error::Error, hackrf::HackRF};
