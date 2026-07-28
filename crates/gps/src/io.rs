#![allow(unused)]

use std::{
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
};

use crate::{Error, IqBlockSizing};

/// Stateful MSB-first packer for signed I/Q decisions.
#[derive(Clone, Copy, Debug, Default)]
struct Bits1PackingState {
    /// Partially filled output byte.
    pending_byte: u8,
    /// Number of high-order bits already filled in `pending_byte`.
    pending_bits: u8,
}

impl Bits1PackingState {
    /// Appends all complete bytes produced by `iq`, retaining any remainder.
    fn push(self, iq: &[i16], out: &mut Vec<u8>) -> Self {
        let mut state = self;
        for &sample in iq {
            state.pending_byte |=
                u8::from(sample > 0) << (7 - state.pending_bits);
            state.pending_bits += 1;
            if state.pending_bits == 8 {
                out.push(state.pending_byte);
                state = Self::default();
            }
        }
        state
    }

    /// Emits a final byte with every unused low-order bit set to zero.
    fn finish(self, out: &mut Vec<u8>) -> Self {
        if self.pending_bits != 0 {
            out.push(self.pending_byte);
        }
        Self::default()
    }
}

/// Packs interleaved i16 I/Q into 1-bit packed format.
///
/// Decisions are packed MSB first. If `iq` does not fill the final byte, its
/// unused low-order bits are zero. Semantics match a completed
/// [`IQWriter`] stream.
pub fn pack_bits1_into(iq: &[i16], out: &mut [u8]) -> Result<(), Error> {
    IqBlockSizing::checked_i16_byte_len(iq.len())?;
    let expected_len =
        iq.len().checked_add(7).ok_or_else(|| {
            Error::unsupported_workload("Bits1 length overflow")
        })? / 8;
    if out.len() != expected_len {
        return Err(Error::msg("Bits1 output length mismatch"));
    }

    let mut packed = Vec::with_capacity(out.len());
    let state = Bits1PackingState::default().push(iq, &mut packed);
    state.finish(&mut packed);
    out.copy_from_slice(&packed);
    Ok(())
}

/// Packs interleaved i16 I/Q into signed 8-bit (SC8) bytes.
///
/// Semantics MUST match `IQWriter::write_samples()` for `DataFormat::Bits8`.
pub fn pack_bits8_into(iq: &[i16], out: &mut [u8]) -> Result<(), Error> {
    IqBlockSizing::checked_i16_byte_len(iq.len())?;
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
    // SAFETY: `size_of_val` describes the same existing slice allocation.
    unsafe {
        std::slice::from_raw_parts(
            iq.as_ptr().cast::<u8>(),
            std::mem::size_of_val(iq),
        )
    }
}

/// Checks the configured block limit before creating a native-endian byte view.
fn checked_as_bytes_i16(iq: &[i16]) -> Result<&[u8], Error> {
    IqBlockSizing::checked_i16_byte_len(iq.len())?;
    Ok(as_bytes_i16(iq))
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

/// Formats and writes one validated interleaved sample block.
fn write_samples_to<W: Write>(
    writer: &mut W, format: DataFormat, buffer: &[i16], buffer_size: usize,
    bits1_state: &mut Bits1PackingState,
) -> Result<(), Error> {
    let sizing = IqBlockSizing::from_interleaved_i16_len(buffer.len())?;
    if sizing.complex_samples() != buffer_size {
        return Err(Error::unsupported_workload(
            "I/Q writer buffer size does not match its complex sample count",
        ));
    }

    match format {
        DataFormat::Bits1 => {
            let packed_capacity = usize::from(bits1_state.pending_bits)
                .checked_add(sizing.interleaved_i16_len())
                .ok_or_else(|| {
                    Error::unsupported_workload("Bits1 capacity overflow")
                })?
                / 8;
            let mut packed = Vec::with_capacity(packed_capacity);
            let next_state = (*bits1_state).push(buffer, &mut packed);
            writer.write_all(&packed)?;
            *bits1_state = next_state;
        }
        DataFormat::Bits8 => {
            let mut packed = vec![0u8; sizing.interleaved_i16_len()];
            pack_bits8_into(buffer, &mut packed)?;
            writer.write_all(&packed)?;
        }
        DataFormat::Bits16 => {
            writer.write_all(checked_as_bytes_i16(buffer)?)?;
        }
    }
    Ok(())
}

/// Writes any pending format-level terminal data.
fn finish_packing_to<W: Write>(
    writer: &mut W, format: DataFormat, bits1_state: &mut Bits1PackingState,
) -> Result<(), Error> {
    if matches!(format, DataFormat::Bits1) && bits1_state.pending_bits != 0 {
        let mut packed = Vec::with_capacity(1);
        let next_state = (*bits1_state).finish(&mut packed);
        writer.write_all(&packed)?;
        *bits1_state = next_state;
    }
    Ok(())
}

/// Attempts format completion followed by an unconditional flush.
fn finish_writer<W: Write>(
    writer: &mut W, format: DataFormat, bits1_state: &mut Bits1PackingState,
) -> Result<(), Error> {
    let packing_result = finish_packing_to(writer, format, bits1_state);
    let flush_result = writer.flush().map_err(Error::from);
    crate::error::resolve_with_finalization(packing_result, flush_result)
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

    /// Size of the I/Q buffer in complex samples
    pub buffer_size: usize,

    /// Pending 1-bit decisions shared across callback blocks.
    bits1_state: Bits1PackingState,
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
        let sizing = IqBlockSizing::new(buffer_size)?;
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        let buffer = vec![0; sizing.interleaved_i16_len()];
        Ok(Self {
            writer,
            format,
            buffer,
            buffer_size: sizing.complex_samples(),
            bits1_state: Bits1PackingState::default(),
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
        write_samples_to(
            &mut self.writer,
            self.format,
            &self.buffer,
            self.buffer_size,
            &mut self.bits1_state,
        )
    }

    /// Finalizes format-level packing without flushing the buffered file.
    ///
    /// For [`DataFormat::Bits1`], a pending partial byte is written with
    /// zero-valued low-order padding bits. The broader fallible file-flush
    /// contract is intentionally handled separately.
    pub fn finish_packing(&mut self) -> Result<(), Error> {
        finish_packing_to(&mut self.writer, self.format, &mut self.bits1_state)
    }

    /// Completes pending format packing and flushes buffered output.
    ///
    /// Flushing is attempted even if final format packing fails. Repeated
    /// successful calls do not emit duplicate [`DataFormat::Bits1`] padding.
    pub fn finish(&mut self) -> Result<(), Error> {
        finish_writer(&mut self.writer, self.format, &mut self.bits1_state)
    }
}

#[cfg(test)]
#[path = "io/finalization_tests.rs"]
mod finalization_tests;

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
        let result = pack_bits8_into(&[0i16, 1i16], &mut out);
        assert!(matches!(
            result,
            Err(ref error)
                if error
                    .to_string()
                    .contains("Bits8 output length mismatch")
        ));
    }

    #[test]
    fn pack_bits1_bit_order_and_sign() -> Result<(), Error> {
        let iq: [i16; 8] = [1, -1, 1, -1, 1, -1, 1, -1];
        let mut out = [0u8; 1];
        pack_bits1_into(&iq, &mut out)?;
        assert_eq!(out[0], 0b1010_1010);
        Ok(())
    }

    #[test]
    fn bits1_state_spans_blocks_and_zero_pads_the_final_byte() {
        let mut packed = Vec::new();
        let state = Bits1PackingState::default()
            .push(&[1, -1, 1], &mut packed)
            .push(&[-1, 1, -1, 1, -1, 1], &mut packed);
        assert_eq!(packed, vec![0b1010_1010]);
        state.finish(&mut packed);
        assert_eq!(packed, vec![0b1010_1010, 0b1000_0000]);
    }
}
