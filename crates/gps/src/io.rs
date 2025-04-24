#![allow(unused)]

use std::{
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
};

use crate::Error;
#[derive(Debug, Copy, Clone)]
pub enum DataFormat {
    Bits1 = 1,
    Bits8 = 8,
    Bits16 = 16,
}

#[derive(Debug)]
pub struct IQWriter {
    writer: BufWriter<File>,
    format: DataFormat,
    pub buffer: Vec<i16>,
    pub buffer_size: usize,
}
impl IQWriter {
    pub fn new(
        path: &PathBuf, format: DataFormat, buffer_size: usize,
    ) -> Result<Self, Error> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        let buffer = vec![0; 2 * buffer_size];
        Ok(Self {
            writer,
            format,
            buffer,
            buffer_size,
        })
    }

    #[inline]
    pub fn write_samples(&mut self) -> Result<(), Error> {
        match self.format {
            DataFormat::Bits1 => {
                let mut iq8_buff = vec![0; self.buffer_size / 4];
                for isamp in 0..2 * self.buffer_size {
                    if isamp % 8 == 0 {
                        iq8_buff[isamp / 8] = 0;
                    }
                    let curr_bit = &mut iq8_buff[isamp / 8];

                    *curr_bit = (i32::from(*curr_bit)
                        | i32::from(i32::from(self.buffer[isamp]) > 0)
                            << (7 - isamp as i32 % 8))
                        as i8;
                }

                unsafe {
                    self.writer.write_all(std::slice::from_raw_parts(
                        iq8_buff.as_ptr().cast::<u8>(),
                        self.buffer_size / 4,
                    ))?;
                }
            }
            DataFormat::Bits8 => {
                let mut iq8_buff = vec![0; 2 * self.buffer_size];
                for (isamp, buff) in iq8_buff.iter_mut().enumerate() {
                    *buff = (i32::from(self.buffer[isamp]) >> 4) as i8;
                    // 12-bit bladeRF -> 8-bit HackRF
                    //iq8_buff[isamp] = iq_buff[isamp] >> 8; // for
                    // PocketSDR
                }

                unsafe {
                    self.writer.write_all(std::slice::from_raw_parts(
                        iq8_buff.as_ptr().cast::<u8>(),
                        2 * self.buffer_size,
                    ))?;
                }
            }
            DataFormat::Bits16 => {
                // data_format==SC16
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
