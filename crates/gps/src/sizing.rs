use std::mem::size_of;

use crate::Error;

/// Checked dimensions for one pre-format interleaved I/Q block.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IqBlockSizing {
    /// Number of complex samples represented by the block.
    complex_samples: usize,
    /// Number of scalar `i16` values after I/Q interleaving.
    interleaved_i16_len: usize,
    /// Allocation size of the interleaved scalar values.
    interleaved_bytes: usize,
}

impl IqBlockSizing {
    /// Maximum complex samples in one interleaved I/Q block.
    pub const MAX_COMPLEX_SAMPLES: usize = Self::MAX_INTERLEAVED_I16_VALUES / 2;
    /// Maximum bytes in one interleaved `i16` I/Q block (64 MiB).
    pub const MAX_INTERLEAVED_BYTES: usize = 64 * 1024 * 1024;
    /// Maximum scalar `i16` values in one interleaved I/Q block.
    pub const MAX_INTERLEAVED_I16_VALUES: usize =
        Self::MAX_INTERLEAVED_BYTES / size_of::<i16>();

    /// Derives every block dimension from a complex-sample count.
    pub fn new(complex_samples: usize) -> Result<Self, Error> {
        let interleaved_i16_len =
            complex_samples.checked_mul(2).ok_or_else(|| {
                Error::unsupported_workload("interleaved I/Q length overflow")
            })?;
        let interleaved_bytes =
            Self::checked_i16_byte_len(interleaved_i16_len)?;

        Ok(Self {
            complex_samples,
            interleaved_i16_len,
            interleaved_bytes,
        })
    }

    /// Validates and derives dimensions from an interleaved `i16` length.
    pub fn from_interleaved_i16_len(
        interleaved_i16_len: usize,
    ) -> Result<Self, Error> {
        if !interleaved_i16_len.is_multiple_of(2) {
            return Err(Error::unsupported_workload(format!(
                "interleaved I/Q length {interleaved_i16_len} is not even"
            )));
        }
        Self::new(interleaved_i16_len / 2)
    }

    /// Returns the number of complex I/Q samples.
    pub fn complex_samples(self) -> usize {
        self.complex_samples
    }

    /// Returns the number of scalar values in the interleaved `i16` block.
    pub fn interleaved_i16_len(self) -> usize {
        self.interleaved_i16_len
    }

    /// Returns the byte size of the interleaved `i16` block.
    pub fn interleaved_bytes(self) -> usize {
        self.interleaved_bytes
    }

    /// Rejects obviously unsupported update blocks before rational conversion.
    pub(crate) fn validate_update_step(
        sample_frequency_hz: f64, step_seconds: f64,
    ) -> Result<(), Error> {
        if !sample_frequency_hz.is_finite()
            || sample_frequency_hz <= 0.0
            || sample_frequency_hz.round() >= u64::MAX as f64
        {
            return Err(Error::invalid_sampling_frequency());
        }
        if !step_seconds.is_finite() || step_seconds <= 0.0 {
            return Err(Error::invalid_update_step(step_seconds));
        }

        let estimated_complex_samples =
            sample_frequency_hz.round() * step_seconds;
        if !estimated_complex_samples.is_finite()
            || estimated_complex_samples.ceil()
                > Self::MAX_COMPLEX_SAMPLES as f64
        {
            return Err(Error::unsupported_workload(format!(
                "configured update block exceeds {} complex samples",
                Self::MAX_COMPLEX_SAMPLES
            )));
        }
        Ok(())
    }

    /// Computes the byte size of scalar `i16` values with checked arithmetic.
    pub(crate) fn checked_i16_byte_len(
        value_count: usize,
    ) -> Result<usize, Error> {
        let byte_count =
            value_count.checked_mul(size_of::<i16>()).ok_or_else(|| {
                Error::unsupported_workload(
                    "interleaved I/Q byte size overflow",
                )
            })?;
        if byte_count > Self::MAX_INTERLEAVED_BYTES {
            return Err(Error::unsupported_workload(format!(
                "I/Q block requires {byte_count} bytes; maximum is {}",
                Self::MAX_INTERLEAVED_BYTES
            )));
        }
        Ok(byte_count)
    }
}
