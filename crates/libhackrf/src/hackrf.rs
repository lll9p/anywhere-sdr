use futures_lite::future::block_on;
use nusb::{
    Device, DeviceInfo, Interface,
    transfer::{
        ControlIn, ControlOut, ControlType, Direction, Queue, Recipient,
        RequestBuffer,
    },
};

use crate::{constants::*, enums::*, error::Error};

/// Main interface for controlling a `HackRF` device
///
/// This struct provides methods for configuring and operating a `HackRF`
/// software-defined radio device. It handles USB communication, device
/// configuration, and data transfer operations.
///
/// # Examples
///
/// ```rust,no_run
/// use libhackrf::prelude::*;
///
/// fn main() -> Result<(), Error> {
///     // Open the first available HackRF device
///     let mut sdr = HackRF::new_auto()?;
///
///     // Configure the device
///     sdr.set_freq(915_000_000)?; // Set frequency to 915 MHz
///     sdr.set_sample_rate_auto(10.0e6)?; // Set sample rate to 10 MHz
///
///     // Print device information
///     println!("Board ID: {}", sdr.board_id()?);
///     println!("Firmware version: {}", sdr.version()?);
///
///     Ok(())
/// }
/// ```
pub struct HackRF {
    /// Current operating mode of the device
    mode: DeviceMode,
    /// USB device handle
    #[allow(unused)]
    device: Device,
    /// Device firmware version
    device_version: u16,
    /// USB interface handle
    interface: Interface,
}
impl std::fmt::Debug for HackRF {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DeviceMode: {:?}, DeviceVersion: {}",
            self.mode, self.device_version
        )
    }
}
impl HackRF {
    /// Opens the first available `HackRF` device
    ///
    /// This method scans for connected `HackRF` devices and opens the first one
    /// found. It's the simplest way to connect to a `HackRF` when only one
    /// device is connected.
    ///
    /// # Returns
    ///
    /// A new `HackRF` instance if a device was found and successfully opened.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No `HackRF` devices are found
    /// - There was a problem opening the USB device
    /// - There was a problem claiming the USB interface
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use libhackrf::prelude::*;
    ///
    /// fn main() -> Result<(), Error> {
    ///     let sdr = HackRF::new_auto()?;
    ///     println!("Connected to HackRF device");
    ///     Ok(())
    /// }
    /// ```
    pub fn new_auto() -> Result<Self, Error> {
        // Open first found HackRF
        let devices = Self::list_devices()?;
        let deviceinfo = devices.first().ok_or(Error::InvalidDevice)?;
        let device_version = deviceinfo.device_version();
        let device = deviceinfo.open()?;
        let interface = device.claim_interface(0)?;
        Ok(Self {
            mode: DeviceMode::Off,
            device,
            device_version,
            interface,
        })
    }

    /// Opens a specific `HackRF` device by serial number
    ///
    /// This method allows you to connect to a specific `HackRF` device when
    /// multiple devices are connected to the system.
    ///
    /// # Parameters
    ///
    /// * `serial_number` - The serial number of the device to open
    ///
    /// # Returns
    ///
    /// A new `HackRF` instance if the device was found and successfully opened.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No `HackRF` device with the specified serial number is found
    /// - There was a problem opening the USB device
    /// - There was a problem claiming the USB interface
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use libhackrf::prelude::*;
    ///
    /// fn main() -> Result<(), Error> {
    ///     let serial = "0123456789abcdef0123456789abcdef";
    ///     let sdr = HackRF::new(&serial)?;
    ///     println!("Connected to HackRF device with serial {}", serial);
    ///     Ok(())
    /// }
    /// ```
    pub fn new(serial_number: &dyn AsRef<str>) -> Result<Self, Error> {
        // Open HackRF with port_number
        let devices = Self::list_devices()?;
        let deviceinfo = devices
            .iter()
            .find(|devinfo| {
                devinfo.serial_number().is_some_and(|sn| {
                    sn.eq_ignore_ascii_case(serial_number.as_ref())
                })
            })
            .ok_or_else(|| {
                Error::InvalidSerialNumber(serial_number.as_ref().to_string())
            })?;
        let device_version = deviceinfo.device_version();
        let device = deviceinfo.open()?;
        let interface = device.claim_interface(0)?;
        Ok(Self {
            mode: DeviceMode::Off,
            device,
            device_version,
            interface,
        })
    }

    /// Lists all connected `HackRF` devices
    ///
    /// This method scans the USB bus for connected `HackRF` devices and returns
    /// information about each device found.
    ///
    /// # Returns
    ///
    /// A vector of `DeviceInfo` objects, each representing a connected `HackRF`
    /// device.
    ///
    /// # Errors
    ///
    /// Returns an error if there was a problem accessing the USB bus.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use libhackrf::prelude::*;
    ///
    /// fn main() -> Result<(), Error> {
    ///     let devices = HackRF::list_devices()?;
    ///     println!("Found {} HackRF devices", devices.len());
    ///
    ///     for (i, device) in devices.iter().enumerate() {
    ///         if let Some(serial) = device.serial_number() {
    ///             println!("Device {}: Serial {}", i, serial);
    ///         }
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn list_devices() -> Result<Vec<DeviceInfo>, Error> {
        Ok(nusb::list_devices().map(|devices| {
            devices
                .filter(|device| {
                    device.vendor_id() == HACKRF_USB_VID
                        && device.product_id() == HACKRF_ONE_USB_PID
                })
                .collect::<Vec<DeviceInfo>>()
        })?)
    }

    /// Returns the maximum transmission unit (MTU) size for bulk transfers
    ///
    /// This value represents the maximum size of data that can be transferred
    /// in a single USB bulk transfer operation.
    ///
    /// # Returns
    ///
    /// The MTU size in bytes.
    pub fn max_transmission_unit(&self) -> usize {
        HACKRF_TRANSFER_BUFFER_SIZE
        // HACKRF_DEVICE_BUFFER_SIZE
    }

    /// Checks if the device firmware version is at least the specified minimum
    ///
    /// Some operations require a minimum firmware version to work correctly.
    /// This method checks if the device's firmware version is sufficient.
    ///
    /// # Parameters
    ///
    /// * `minimal` - The minimum required firmware version
    ///
    /// # Returns
    ///
    /// `Ok(())` if the device firmware version is sufficient, or an error
    /// otherwise.
    ///
    /// # Errors
    ///
    /// Returns a `VersionMismatch` error if the device firmware version is less
    /// than the specified minimum.
    fn check_api_version(&self, minimal: u16) -> Result<(), Error> {
        if self.device_version >= minimal {
            Ok(())
        } else {
            Err(Error::VersionMismatch {
                device: self.device_version,
                minimal,
            })
        }
    }

    /// Returns the device firmware version
    ///
    /// # Returns
    ///
    /// The firmware version as a 16-bit unsigned integer.
    pub fn device_version(&self) -> u16 {
        self.device_version
    }

    /// Sends a USB control request to the device and reads the response
    ///
    /// This is a low-level method used by other methods to communicate with the
    /// device.
    ///
    /// # Parameters
    ///
    /// * `request` - The request code to send
    /// * `value` - The value parameter for the control request
    /// * `index` - The index parameter for the control request
    ///
    /// # Type Parameters
    ///
    /// * `N` - The number of bytes to read from the device
    ///
    /// # Returns
    ///
    /// A vector containing the data read from the device.
    ///
    /// # Errors
    ///
    /// Returns an error if the USB transfer fails.
    fn read_control<const N: u16>(
        &self, request: Request, value: u16, index: u16,
    ) -> Result<Vec<u8>, Error> {
        let data = block_on(self.interface.control_in(ControlIn {
            control_type: ControlType::Vendor,
            recipient: Recipient::Device,
            request: request.into(),
            value,
            index,
            length: N,
        }))
        .into_result()?;
        Ok(data)
    }

    /// Sends a USB control request with data to the device
    ///
    /// This is a low-level method used by other methods to communicate with the
    /// device.
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
    fn write_control(
        &mut self, request: Request, value: u16, index: u16, data: &[u8],
    ) -> Result<(), Error> {
        let result = block_on(self.interface.control_out(ControlOut {
            control_type: ControlType::Vendor,
            recipient: Recipient::Device,
            request: request.into(),
            value,
            index,
            data,
        }));
        let n = result.data.actual_length();
        if n == data.len() {
            Ok(result.status?)
        } else {
            Err(Error::ControlTransfer {
                direction: Direction::In,
                actual: n,
                expected: data.len(),
            })
        }
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
        let data = self.read_control::<1>(Request::BoardIdRead, 0, 0)?;
        Ok(data[0])
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
        let data =
            self.read_control::<32>(Request::BoardPartidSerialnoRead, 0, 0)?;
        let part_id_1: u32 = u32::from_le_bytes(data[0..4].try_into()?);
        let part_id_2: u32 = u32::from_le_bytes(data[4..8].try_into()?);

        let mut serial_number: String = String::new();

        for i in 0..4 {
            let bytes = data[8 + 4 * i..12 + 4 * i].try_into()?;
            let value = u32::from_le_bytes(bytes);
            use std::fmt::Write;
            write!(serial_number, "{value:08x?}")?;
        }

        Ok(((part_id_1, part_id_2), serial_number))
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
        let data = self.read_control::<16>(Request::VersionStringRead, 0, 0)?;
        Ok(String::from_utf8_lossy(&data[..]).into())
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

    /// Sets the RF frequency
    ///
    /// This method sets the center frequency for reception or transmission.
    ///
    /// # Parameters
    ///
    /// * `hz` - The frequency in Hertz
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
    ///     // Set frequency to 915 MHz
    ///     sdr.set_freq(915_000_000)?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn set_freq(&mut self, hz: u64) -> Result<(), Error> {
        let buffer: [u8; 8] = freq_params(hz);
        self.write_control(Request::SetFreq, 0, 0, &buffer)
    }

    /// Sets the baseband filter bandwidth
    ///
    /// The baseband filter limits the bandwidth of the signal to prevent
    /// aliasing.
    ///
    /// # Parameters
    ///
    /// * `hz` - The filter bandwidth in Hertz
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
    ///     // Set baseband filter bandwidth to 10 MHz
    ///     sdr.set_baseband_filter_bandwidth(10_000_000)?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn set_baseband_filter_bandwidth(
        &mut self, hz: u32,
    ) -> Result<(), Error> {
        self.write_control(
            Request::BasebandFilterBandwidthSet,
            (hz & 0xFFFF) as u16,
            (hz >> 16) as u16,
            &[],
        )
    }

    /// Sets the sample rate with manual frequency and divider values
    ///
    /// This method allows precise control over the sample rate by specifying
    /// both the frequency and the divider.
    ///
    /// # Parameters
    ///
    /// * `freq_hz` - The frequency in Hertz
    /// * `divider` - The divider value
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
    ///     // Set sample rate to 10 MHz (10 MHz / 1)
    ///     sdr.set_sample_rate_manual(10_000_000, 1)?;
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Note
    ///
    /// This method also automatically sets the baseband filter bandwidth to
    /// an appropriate value based on the sample rate.
    pub fn set_sample_rate_manual(
        &mut self, freq_hz: u32, divider: u32,
    ) -> Result<(), Error> {
        // only support little endian computer for now
        let hz = freq_hz.to_le();
        let div = divider.to_le();
        let mut bytes: [u8; 8] = [0; 8];
        bytes[0..4].copy_from_slice(&freq_hz.to_le_bytes());
        bytes[4..8].copy_from_slice(&divider.to_le_bytes());
        self.write_control(Request::SampleRateSet, 0, 0, &bytes)?;
        self.set_baseband_filter_bandwidth(compute_baseband_filter_bw(
            (0.75 * (hz as f32) / (div as f32)) as u32,
        ))
    }

    /// For anti-aliasing, the baseband filter bandwidth is automatically set to
    /// the widest available setting that is no more than 75% of the sample
    /// rate. This happens every time the sample rate is set. If you want to
    /// override the baseband filter selection, you must do so after setting
    /// the sample rate. Sets the sample rate automatically based on the
    /// desired frequency
    ///
    /// This method calculates appropriate frequency and divider values to
    /// achieve the requested sample rate, using an algorithm that finds
    /// optimal values.
    ///
    /// # Parameters
    ///
    /// * `freq` - The desired sample rate in Hz
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
    ///     // Set sample rate to 10 MHz
    ///     sdr.set_sample_rate_auto(10.0e6)?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn set_sample_rate_auto(&mut self, freq: f64) -> Result<(), Error> {
        // Define the maximum number of iterations
        const MAX_N: usize = 32;
        // Calculate the fractional part of the frequency and add 1.0
        let freq_frac: f64 = 1.0 + freq.fract();
        // Initialize accumulator and multiplier
        let mut acc: u64 = 0;
        let mut multiplier: usize = 1;
        // Convert frequency to bit representation
        let freq_bits = freq.to_bits();
        // Extract exponent part (with bias of 1023)
        let exponent = ((freq_bits >> 52) & 0x7FF) as i32 - 1023;
        // Initialize mask for extracting mantissa
        let mut mask = (1u64 << 52) - 1;
        // Convert fractional part to bit representation
        let mut frac_bits = freq_frac.to_bits();
        frac_bits &= mask;
        // Update mask to clear bits higher than specific position
        mask &= !((1u64 << (exponent + 4)) - 1);
        // Iterate to find suitable multiplier, up to MAX_N times
        for ii in 1..=MAX_N {
            multiplier = ii;
            acc += frac_bits;
            // Check if bitwise AND of accumulator and mask is zero
            if (acc & mask == 0) || (!acc & mask == 0) {
                break;
            }
        }
        // If no suitable multiplier found, default to 1
        if multiplier == MAX_N {
            multiplier = 1;
        }
        // Calculate frequency in Hz, rounded to integer
        let freq_hz = (freq * multiplier as f64).round() as u32;
        // Get final divider
        let divider = multiplier as u32;
        self.set_sample_rate_manual(freq_hz, divider)
    }

    /// Sets the LNA (Low Noise Amplifier) gain
    ///
    /// The LNA gain affects the sensitivity of the receiver.
    ///
    /// # Parameters
    ///
    /// * `value` - The gain value (0-40 dB)
    ///
    /// # Returns
    ///
    /// `Ok(())` if the operation was successful.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The gain value is out of range (>40)
    /// - The USB communication fails
    /// - The device rejects the gain setting
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use libhackrf::prelude::*;
    ///
    /// fn main() -> Result<(), Error> {
    ///     let mut sdr = HackRF::new_auto()?;
    ///
    ///     // Set LNA gain to 16 dB
    ///     sdr.set_lna_gain(16)?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn set_lna_gain(&mut self, value: u16) -> Result<(), Error> {
        if value > 40 {
            Err(Error::Argument)
        } else {
            let buffer =
                self.read_control::<1>(Request::SetLnaGain, 0, value & !0x07)?;
            if buffer[0] == 0 {
                Err(Error::Argument)
            } else {
                Ok(())
            }
        }
    }

    /// Sets the VGA (Variable Gain Amplifier) gain for the receiver
    ///
    /// The VGA gain provides additional amplification in the receive path.
    ///
    /// # Parameters
    ///
    /// * `value` - The gain value (0-62 dB)
    ///
    /// # Returns
    ///
    /// `Ok(())` if the operation was successful.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The gain value is out of range (>62)
    /// - The USB communication fails
    /// - The device rejects the gain setting
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use libhackrf::prelude::*;
    ///
    /// fn main() -> Result<(), Error> {
    ///     let mut sdr = HackRF::new_auto()?;
    ///
    ///     // Set VGA gain to 20 dB
    ///     sdr.set_vga_gain(20)?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn set_vga_gain(&mut self, value: u16) -> Result<(), Error> {
        if value > 62 {
            Err(Error::Argument)
        } else {
            let buffer =
                self.read_control::<1>(Request::SetVgaGain, 0, value & !0b1)?;
            if buffer[0] == 0 {
                Err(Error::Argument)
            } else {
                Ok(())
            }
        }
    }

    /// Sets the TX VGA (Variable Gain Amplifier) gain for the transmitter
    ///
    /// The TX VGA gain controls the output power of the transmitter.
    ///
    /// # Parameters
    ///
    /// * `value` - The gain value (0-47 dB)
    ///
    /// # Returns
    ///
    /// `Ok(())` if the operation was successful.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The gain value is out of range (>47)
    /// - The USB communication fails
    /// - The device rejects the gain setting
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use libhackrf::prelude::*;
    ///
    /// fn main() -> Result<(), Error> {
    ///     let mut sdr = HackRF::new_auto()?;
    ///
    ///     // Set TX VGA gain to 30 dB
    ///     sdr.set_txvga_gain(30)?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn set_txvga_gain(&mut self, value: u16) -> Result<(), Error> {
        if value > 47 {
            Err(Error::Argument)
        } else {
            let buffer =
                self.read_control::<1>(Request::SetTxvgaGain, 0, value)?;
            if buffer[0] == 0 {
                Err(Error::Argument)
            } else {
                Ok(())
            }
        }
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

    /// Sets the transceiver mode (internal method)
    ///
    /// This is an internal method used by other methods to set the operating
    /// mode of the transceiver.
    ///
    /// # Parameters
    ///
    /// * `mode` - The transceiver mode to set
    ///
    /// # Returns
    ///
    /// `Ok(())` if the operation was successful.
    ///
    /// # Errors
    ///
    /// Returns an error if the USB communication fails.
    fn set_transceiver_mode(
        &mut self, mode: TransceiverMode,
    ) -> Result<(), Error> {
        self.write_control(Request::SetTransceiverMode, mode.into(), 0, &[])
    }

    /// Puts the device into receive mode
    ///
    /// This method configures the device to receive RF signals.
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
    ///     // Configure for reception
    ///     sdr.set_freq(915_000_000)?;
    ///     sdr.set_sample_rate_auto(10.0e6)?;
    ///
    ///     // Enter receive mode
    ///     sdr.enter_rx_mode()?;
    ///
    ///     // Now ready to receive data...
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn enter_rx_mode(&mut self) -> Result<(), Error> {
        self.set_transceiver_mode(TransceiverMode::Receive)?;
        self.mode = DeviceMode::Rx;
        Ok(())
    }

    /// Puts the device into transmit mode
    ///
    /// This method configures the device to transmit RF signals.
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
    ///     // Configure for transmission
    ///     sdr.set_freq(915_000_000)?;
    ///     sdr.set_sample_rate_auto(10.0e6)?;
    ///
    ///     // Enter transmit mode
    ///     sdr.enter_tx_mode()?;
    ///
    ///     // Now ready to transmit data...
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn enter_tx_mode(&mut self) -> Result<(), Error> {
        self.set_transceiver_mode(TransceiverMode::Transmit)?;
        self.mode = DeviceMode::Tx;
        Ok(())
    }

    /// Gets a queue for receiving data from the device
    ///
    /// This method returns a queue that can be used to receive data from the
    /// device in receive mode.
    ///
    /// # Returns
    ///
    /// A queue for receiving data.
    pub fn rx_queue(&mut self) -> Queue<RequestBuffer> {
        self.interface.bulk_in_queue(HACKRF_RX_ENDPOINT_ADDRESS)
    }

    /// Gets a queue for sending data to the device
    ///
    /// This method returns a queue that can be used to send data to the device
    /// in transmit mode.
    ///
    /// # Returns
    ///
    /// A queue for sending data.
    pub fn tx_queue(&mut self) -> Queue<Vec<u8>> {
        self.interface.bulk_out_queue(HACKRF_TX_ENDPOINT_ADDRESS)
    }

    /// Stops receiving mode
    ///
    /// This method stops the device from receiving and returns it to the idle
    /// state.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the operation was successful.
    ///
    /// # Errors
    ///
    /// Returns an error if the USB communication fails.
    pub fn stop_rx(&mut self) -> Result<(), Error> {
        self.set_transceiver_mode(TransceiverMode::Off)?;
        self.mode = DeviceMode::Off;
        Ok(())
    }

    /// Stops transmitting mode
    ///
    /// This method stops the device from transmitting and returns it to the
    /// idle state.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the operation was successful.
    ///
    /// # Errors
    ///
    /// Returns an error if the USB communication fails.
    pub fn stop_tx(&mut self) -> Result<(), Error> {
        self.set_transceiver_mode(TransceiverMode::Off)?;
        self.mode = DeviceMode::Off;
        Ok(())
    }

    /// Resets the device
    ///
    /// This method performs a full reset of the device, returning it to its
    /// initial state.
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
    ///
    /// # Note
    ///
    /// This method consumes the `HackRF` instance. After calling this method,
    /// you will need to create a new instance to continue using the device.
    pub fn reset(mut self) -> Result<(), Error> {
        self.check_api_version(0x0102)?;
        self.write_control(Request::Reset, 0, 0, &[])?;
        self.mode = DeviceMode::Off;
        Ok(())
    }
}
/// Converts a frequency in Hertz to the format required by the `HackRF` device
///
/// This function splits the frequency into MHz and Hz components and packs them
/// into an 8-byte array in little-endian format.
///
/// # Parameters
///
/// * `hz` - The frequency in Hertz
///
/// # Returns
///
/// An 8-byte array containing the frequency in the format required by the
/// device.
fn freq_params(hz: u64) -> [u8; 8] {
    let l_freq_mhz = (hz / MHZ) as u32;
    let l_freq_hz = (hz % MHZ) as u32;
    let mut bytes: [u8; 8] = [0; 8];
    bytes[0..4].copy_from_slice(&l_freq_mhz.to_le_bytes());
    bytes[4..8].copy_from_slice(&l_freq_hz.to_le_bytes());
    bytes
}

/// Computes a baseband filter bandwidth that is less than or equal to the
/// requested bandwidth
///
/// This function finds the largest available bandwidth from the MAX2837 chip
/// that is less than or equal to the requested bandwidth.
///
/// # Parameters
///
/// * `bandwidth_hz` - The requested bandwidth in Hertz
///
/// # Returns
///
/// The selected bandwidth in Hertz.
#[allow(unused)]
fn compute_baseband_filter_bw_round_down_lt(bandwidth_hz: u32) -> u32 {
    let mut p: u32 = 0;
    let mut ix: usize = 0;
    for (i, v) in MAX2837.iter().enumerate() {
        if *v >= bandwidth_hz {
            p = *v;
            ix = i;
            break;
        }
    }

    /* Round down (if no equal to first entry) and if > bandwidth_hz */
    if ix != 0 {
        p = MAX2837[ix - 1];
    }
    p
}

/// Computes an appropriate baseband filter bandwidth for the given sample rate
///
/// This function selects a bandwidth from the available MAX2837 chip settings
/// that is appropriate for the requested bandwidth.
///
/// # Parameters
///
/// * `bandwidth_hz` - The requested bandwidth in Hertz
///
/// # Returns
///
/// The selected bandwidth in Hertz.
fn compute_baseband_filter_bw(bandwidth_hz: u32) -> u32 {
    let mut p: u32 = 0;
    let mut ix: usize = 0;
    for (i, v) in MAX2837.iter().enumerate() {
        if *v >= bandwidth_hz {
            p = *v;
            ix = i;
            break;
        }
    }

    /* Round down (if no equal to first entry) and if > bandwidth_hz */
    if ix != 0 && p > bandwidth_hz {
        p = MAX2837[ix - 1];
    }
    p
}
