#![allow(unused)]

use std::{
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
};

use crate::Error;

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
                let mut iq8_buff = vec![0; self.buffer_size / 4];
                for isamp in 0..2 * self.buffer_size {
                    if isamp % 8 == 0 {
                        iq8_buff[isamp / 8] = 0;
                    }
                    let curr_bit = &mut iq8_buff[isamp / 8];

                    // Set the appropriate bit based on sample sign
                    *curr_bit = (i32::from(*curr_bit)
                        | i32::from(i32::from(self.buffer[isamp]) > 0)
                            << (7 - isamp as i32 % 8))
                        as i8;
                }

                // Write the packed bits to the file
                // SAFETY: We're creating a byte slice from a valid vector of i8
                // values. The vector is allocated and
                // initialized above, and we're only reading the
                // raw bytes to write them to a file. The slice lifetime is
                // limited to this function call and doesn't
                // outlive the vector.
                unsafe {
                    self.writer.write_all(std::slice::from_raw_parts(
                        iq8_buff.as_ptr().cast::<u8>(),
                        self.buffer_size / 4,
                    ))?;
                }
            }
            DataFormat::Bits8 => {
                // For 8-bit format, convert 16-bit samples to 8-bit
                let mut iq8_buff = vec![0; 2 * self.buffer_size];
                for (isamp, buff) in iq8_buff.iter_mut().enumerate() {
                    // Convert 16-bit to 8-bit by right-shifting 4 bits
                    *buff = (i32::from(self.buffer[isamp]) >> 4) as i8;
                    // 12-bit bladeRF -> 8-bit HackRF
                    //iq8_buff[isamp] = iq_buff[isamp] >> 8; // for PocketSDR
                }

                // Write the 8-bit samples to the file
                // SAFETY: We're creating a byte slice from a valid vector of i8
                // values. The vector is allocated and
                // initialized above, and we're only reading the
                // raw bytes to write them to a file. The slice lifetime is
                // limited to this function call and doesn't
                // outlive the vector.
                unsafe {
                    self.writer.write_all(std::slice::from_raw_parts(
                        iq8_buff.as_ptr().cast::<u8>(),
                        2 * self.buffer_size,
                    ))?;
                }
            }
            DataFormat::Bits16 => {
                // For 16-bit format, write samples directly
                // SAFETY: We're creating a byte slice from the internal i16
                // buffer. The buffer is allocated and
                // initialized before this function is called,
                // and we're only reading the raw bytes to write them to a file.
                // The slice lifetime is limited to this function call and
                // doesn't outlive the buffer.
                let byte_slice = unsafe {
                    std::slice::from_raw_parts(
                        self.buffer.as_ptr().cast::<u8>(),
                        2 * self.buffer_size * 2, // 2 bytes per sample
                    )
                };
                self.writer.write_all(byte_slice)?;
            }
        }
        Ok(())
    }
}
