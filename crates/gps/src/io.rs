#![allow(unused)]

use std::{
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
};

use crate::Error;

/// Packs interleaved i16 I/Q into 1-bit packed format.
///
/// Semantics MUST match `IQWriter::write_samples()`.
pub fn pack_bits1_into(iq: &[i16], out: &mut [u8]) -> Result<(), Error> {
    if !iq.len().is_multiple_of(8) {
        return Err(Error::msg("Bits1 requires I/Q length divisible by 8"));
    }
    if out.len() != iq.len() / 8 {
        return Err(Error::msg("Bits1 output length mismatch"));
    }

    for (byte_index, chunk) in iq.chunks_exact(8).enumerate() {
        let mut byte: u8 = 0;
        for (bit_index, &sample) in chunk.iter().enumerate() {
            byte |= u8::from(sample > 0) << (7 - bit_index as u8);
        }
        out[byte_index] = byte;
    }

    Ok(())
}

/// Packs interleaved i16 I/Q into signed 8-bit (SC8) bytes.
///
/// Semantics MUST match `IQWriter::write_samples()` for `DataFormat::Bits8`.
pub fn pack_bits8_into(iq: &[i16], out: &mut [u8]) -> Result<(), Error> {
    if out.len() != iq.len() {
        return Err(Error::msg("Bits8 output length mismatch"));
    }

    for (dst, &src) in out.iter_mut().zip(iq.iter()) {
        *dst = (i32::from(src) >> 4) as u8;
    }

    Ok(())
}

/// Views an i16 slice as raw bytes (native endian).
///
/// Semantics MUST match `IQWriter::write_samples()` for `DataFormat::Bits16`.
pub fn as_bytes_i16(iq: &[i16]) -> &[u8] {
    // SAFETY: This is a read-only view of an existing i16 slice.
    unsafe {
        std::slice::from_raw_parts(iq.as_ptr().cast::<u8>(), iq.len() * 2)
    }
}

/// Defines the bit depth format for I/Q sample data.
///
/// This enum specifies the number of bits used to represent each I/Q sample
/// in the output file. Different bit depths offer trade-offs between file size
/// and signal quality.
#[derive(Debug, Copy, Clone)]
pub enum DataFormat {
    /// 1-bit I/Q samples (smallest file size, lowest quality)
    Bits1 = 1,

    /// 8-bit I/Q samples (medium file size and quality)
    Bits8 = 8,

    /// 16-bit I/Q samples (largest file size, highest quality)
    Bits16 = 16,
}

/// Handles writing I/Q samples to an output file.
///
/// This structure manages the buffering and formatting of I/Q samples
/// for writing to a binary file. It supports different bit depths (1, 8, or 16
/// bits) and handles the necessary conversions and optimizations.
#[derive(Debug)]
pub struct IQWriter {
    /// Buffered file writer for efficient I/O
    writer: BufWriter<File>,

    /// Format specification for the output data
    format: DataFormat,

    /// Buffer for storing I/Q samples before writing to file
    pub buffer: Vec<i16>,

    /// Size of the I/Q buffer in samples
    pub buffer_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_bits8_matches_i8_write_semantics() -> Result<(), Error> {
        let iq: [i16; 13] = [
            -32768, -2049, -2048, -17, -16, -1, 0, 1, 15, 16, 2047, 2048, 32767,
        ];
        let mut out = vec![0u8; iq.len()];
        pack_bits8_into(&iq, &mut out)?;

        let expected: Vec<u8> = iq
            .iter()
            .map(|&s| ((i32::from(s) >> 4) as i8) as u8)
            .collect();
        assert_eq!(out, expected);

        assert_eq!(out[5], 255); // -1 >> 4 == -1
        assert_eq!(out[3], 254); // -17 >> 4 == -2
        assert_eq!(out[11], 128); // 2048 >> 4 == 128 (wraps in i8)
        assert_eq!(out[12], 255); // 32767 >> 4 == 2047 (wraps in i8)
        Ok(())
    }

    #[test]
    fn pack_bits8_output_length_mismatch_is_error() {
        let mut out = [0u8; 1];
        let err = pack_bits8_into(&[0i16, 1i16], &mut out).unwrap_err();
        assert!(err.to_string().contains("Bits8 output length mismatch"));
    }

    #[test]
    fn pack_bits1_bit_order_and_sign() -> Result<(), Error> {
        let iq: [i16; 8] = [1, -1, 1, -1, 1, -1, 1, -1];
        let mut out = [0u8; 1];
        pack_bits1_into(&iq, &mut out)?;
        assert_eq!(out[0], 0b1010_1010);
        Ok(())
    }
}
impl IQWriter {
    /// Creates a new I/Q sample writer.
    ///
    /// This method initializes a new writer for I/Q samples with the specified
    /// format and buffer size. It creates the output file and allocates the
    /// necessary buffer memory.
    ///
    /// # Arguments
    /// * `path` - Path to the output file
    /// * `format` - Format specification for the output data (1, 8, or 16 bits)
    /// * `buffer_size` - Size of the I/Q buffer in samples
    ///
    /// # Returns
    /// * `Ok(Self)` - A new `IQWriter` instance
    /// * `Err(Error)` - If the file cannot be created
    ///
    /// # Errors
    /// * Returns an error if the output file cannot be created
    pub fn new(
        path: &PathBuf, format: DataFormat, buffer_size: usize,
    ) -> Result<Self, Error> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        // Allocate buffer for I/Q samples (2 values per sample: I and Q)
        let buffer = vec![0; 2 * buffer_size];
        Ok(Self {
            writer,
            format,
            buffer,
            buffer_size,
        })
    }

    /// Writes the current I/Q sample buffer to the output file.
    ///
    /// This method processes the I/Q samples in the buffer according to the
    /// specified data format and writes them to the output file. The processing
    /// depends on the bit depth:
    ///
    /// - For 1-bit format: Packs 8 samples into each byte
    /// - For 8-bit format: Converts 16-bit samples to 8-bit
    /// - For 16-bit format: Writes samples directly
    ///
    /// # Returns
    /// * `Ok(())` - If the samples were successfully written
    /// * `Err(Error)` - If there was an error writing to the file
    ///
    /// # Errors
    /// * Returns an error if writing to the output file fails
    #[inline]
    pub fn write_samples(&mut self) -> Result<(), Error> {
        match self.format {
            DataFormat::Bits1 => {
                // For 1-bit format, pack 8 samples into each byte
                let mut packed = vec![0u8; self.buffer_size / 4];
                pack_bits1_into(&self.buffer, &mut packed)?;
                self.writer.write_all(&packed)?;
            }
            DataFormat::Bits8 => {
                // For 8-bit format, convert 16-bit samples to 8-bit
                let mut packed = vec![0u8; 2 * self.buffer_size];
                pack_bits8_into(&self.buffer, &mut packed)?;
                self.writer.write_all(&packed)?;
            }
            DataFormat::Bits16 => {
                // For 16-bit format, write samples directly
                self.writer.write_all(as_bytes_i16(&self.buffer))?;
            }
        }
        Ok(())
    }
}
