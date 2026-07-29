use std::time::Duration;

use nusb::{
    MaybeFuture,
    transfer::{ControlIn, ControlOut, ControlType, Direction, Recipient},
};

use super::HackRF;
use crate::{enums::Request, error::Error};

/// Converts a successful fixed-format control-IN reply to its exact array.
pub(crate) fn exact_control_reply<const N: usize>(
    data: Vec<u8>,
) -> Result<[u8; N], Error> {
    let actual = data.len();
    data.try_into().map_err(|_| Error::ControlTransfer {
        direction: Direction::In,
        actual,
        expected: N,
    })
}

/// Decodes an exact board-ID reply.
pub(crate) fn decode_board_id([board_id]: [u8; 1]) -> u8 {
    board_id
}

/// Decodes an exact part-ID and serial-number reply.
pub(crate) fn decode_part_id_serial(
    data: [u8; 24],
) -> Result<((u32, u32), String), Error> {
    let part_id_1 = u32::from_le_bytes(data[0..4].try_into()?);
    let part_id_2 = u32::from_le_bytes(data[4..8].try_into()?);
    let mut serial_number = String::new();

    for index in 0..4 {
        let start = 8 + 4 * index;
        let value = u32::from_le_bytes(data[start..start + 4].try_into()?);
        use std::fmt::Write;
        write!(serial_number, "{value:08x}")?;
    }

    Ok(((part_id_1, part_id_2), serial_number))
}

/// Decodes a bounded variable-length firmware version reply lossily.
pub(crate) fn decode_firmware_version(data: Vec<u8>) -> String {
    String::from_utf8_lossy(&data).into_owned()
}

/// Decodes an exact gain acknowledgement reply.
pub(crate) fn decode_gain_ack([acknowledgement]: [u8; 1]) -> Result<(), Error> {
    if acknowledgement == 0 {
        Err(Error::Argument)
    } else {
        Ok(())
    }
}

impl HackRF {
    /// Reads a bounded variable-length USB control response.
    pub(super) fn read_control_variable(
        &self, request: Request, value: u16, index: u16, maximum: u16,
    ) -> Result<Vec<u8>, Error> {
        Ok(self
            .interface
            .control_in(
                ControlIn {
                    control_type: ControlType::Vendor,
                    recipient: Recipient::Device,
                    request: request.into(),
                    value,
                    index,
                    length: maximum,
                },
                Duration::from_secs(1),
            )
            .wait()?)
    }

    /// Reads a fixed-size USB control response and validates its exact length.
    pub(super) fn read_control_exact<const N: usize>(
        &self, request: Request, value: u16, index: u16,
    ) -> Result<[u8; N], Error> {
        let length = u16::try_from(N).map_err(|_| Error::Argument)?;
        let data = self.read_control_variable(request, value, index, length)?;
        exact_control_reply(data)
    }

    /// Sends a USB control request with data to the device
    ///
    /// This is a low-level method used by other methods to communicate with the
    /// device. This operation blocks synchronously using `.wait()` until the
    /// transfer completes.
    ///
    /// # Parameters
    ///
    /// * `request` - The request code to send
    /// * `value` - The value parameter for the control request
    /// * `index` - The index parameter for the control request
    /// * `data` - The data to send to the device
    ///
    /// # Returns
    ///
    /// `Ok(())` if the transfer was successful.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The USB transfer fails
    /// - The number of bytes transferred doesn't match the expected count
    pub(super) fn write_control(
        &mut self, request: Request, value: u16, index: u16, data: &[u8],
    ) -> Result<(), Error> {
        self.interface
            .control_out(
                ControlOut {
                    control_type: ControlType::Vendor,
                    recipient: Recipient::Device,
                    request: request.into(),
                    value,
                    index,
                    data,
                },
                Duration::from_secs(1),
            )
            .wait()?;
        Ok(())
    }

    /// Reads the board ID from the device
    ///
    /// The board ID identifies the specific type of `HackRF` hardware.
    ///
    /// # Returns
    ///
    /// The board ID as an 8-bit unsigned integer.
    ///
    /// # Errors
    ///
    /// Returns an error if the USB communication fails.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use libhackrf::prelude::*;
    ///
    /// fn main() -> Result<(), Error> {
    ///     let sdr = HackRF::new_auto()?;
    ///     let board_id = sdr.board_id()?;
    ///     println!("Board ID: {}", board_id);
    ///     Ok(())
    /// }
    /// ```
    pub fn board_id(&self) -> Result<u8, Error> {
        let data = self.read_control_exact::<1>(Request::BoardIdRead, 0, 0)?;
        Ok(decode_board_id(data))
    }

    /// Reads the part ID and serial number from the device
    ///
    /// This method returns both the part ID (which consists of two 32-bit
    /// values) and the serial number as a hexadecimal string.
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// - A tuple of two 32-bit unsigned integers representing the part ID
    /// - A string containing the serial number in hexadecimal format
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The USB communication fails
    /// - The data conversion fails
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use libhackrf::prelude::*;
    ///
    /// fn main() -> Result<(), Error> {
    ///     let sdr = HackRF::new_auto()?;
    ///     let ((part_id_1, part_id_2), serial) = sdr.part_id_serial_read()?;
    ///     println!("Part ID: 0x{:08x} 0x{:08x}", part_id_1, part_id_2);
    ///     println!("Serial: {}", serial);
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Note
    ///
    /// This method consumes the `HackRF` instance because it needs to transfer
    /// ownership of the device to read the serial number.
    pub fn part_id_serial_read(self) -> Result<((u32, u32), String), Error> {
        let data = self.read_control_exact::<24>(
            Request::BoardPartidSerialnoRead,
            0,
            0,
        )?;
        decode_part_id_serial(data)
    }

    /// Reads the firmware version string from the device
    ///
    /// # Returns
    ///
    /// A string containing the firmware version.
    ///
    /// # Errors
    ///
    /// Returns an error if the USB communication fails.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use libhackrf::prelude::*;
    ///
    /// fn main() -> Result<(), Error> {
    ///     let sdr = HackRF::new_auto()?;
    ///     let version = sdr.version()?;
    ///     println!("Firmware version: {}", version);
    ///     Ok(())
    /// }
    /// ```
    pub fn version(&self) -> Result<String, Error> {
        let data =
            self.read_control_variable(Request::VersionStringRead, 0, 0, 16)?;
        Ok(decode_firmware_version(data))
    }

    /// Enables or disables the RF amplifier
    ///
    /// The RF amplifier provides additional gain for received signals.
    ///
    /// # Parameters
    ///
    /// * `en` - `true` to enable the amplifier, `false` to disable it
    ///
    /// # Returns
    ///
    /// `Ok(())` if the operation was successful.
    ///
    /// # Errors
    ///
    /// Returns an error if the USB communication fails.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use libhackrf::prelude::*;
    ///
    /// fn main() -> Result<(), Error> {
    ///     let mut sdr = HackRF::new_auto()?;
    ///
    ///     // Enable the RF amplifier
    ///     sdr.set_amp_enable(true)?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn set_amp_enable(&mut self, en: bool) -> Result<(), Error> {
        self.write_control(Request::AmpEnable, en.into(), 0, &[])
    }

    /// Enables or disables the antenna port power
    ///
    /// This controls the power to the antenna port, which can be used to
    /// power an external active antenna or other device.
    ///
    /// # Parameters
    ///
    /// * `value` - 0 to disable, 1 to enable
    ///
    /// # Returns
    ///
    /// `Ok(())` if the operation was successful.
    ///
    /// # Errors
    ///
    /// Returns an error if the USB communication fails.
    pub fn set_antenna_enable(&mut self, value: u8) -> Result<(), Error> {
        self.write_control(Request::AntennaEnable, value.into(), 0, &[])
    }

    /// Enables or disables the clock output
    ///
    /// The clock output can be used to synchronize external devices.
    ///
    /// # Parameters
    ///
    /// * `value` - `true` to enable, `false` to disable
    ///
    /// # Returns
    ///
    /// `Ok(())` if the operation was successful.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The device firmware version is too old
    /// - The USB communication fails
    pub fn set_clkout_enable(&mut self, value: bool) -> Result<(), Error> {
        self.check_api_version(0x0103)?;
        self.write_control(Request::ClkoutEnable, value.into(), 0, &[])
    }

    /// Sets the hardware synchronization mode
    ///
    /// This controls how the device synchronizes with external timing sources.
    ///
    /// # Parameters
    ///
    /// * `value` - The synchronization mode (0 for off, 1 for on)
    ///
    /// # Returns
    ///
    /// `Ok(())` if the operation was successful.
    ///
    /// # Errors
    ///
    /// Returns an error if the USB communication fails.
    pub fn set_hw_sync_mode(&mut self, value: u8) -> Result<(), Error> {
        self.write_control(Request::SetHwSyncMode, value.into(), 0, &[])
    }
}
