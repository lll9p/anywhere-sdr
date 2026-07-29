/// Radio frequency, sample rate, filter, and gain configuration.
mod configuration;
/// USB control transfers and device metadata operations.
mod control;
/// Streaming mode and bulk endpoint operations.
mod stream;

#[cfg(test)]
pub(crate) use control::{
    decode_board_id, decode_firmware_version, decode_gain_ack,
    decode_part_id_serial, exact_control_reply,
};
use nusb::{Device, DeviceInfo, Interface, MaybeFuture};

use crate::{constants::*, enums::DeviceMode, error::Error};

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
    /// This operation is synchronous and blocks until the device is opened and
    /// the interface is claimed via `.wait()`.
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
        let device = deviceinfo.open().wait()?;
        let interface = device.claim_interface(0).wait()?;
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
    /// This operation is synchronous and blocks until the device is opened and
    /// the interface is claimed via `.wait()`.
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
        let device = deviceinfo.open().wait()?;
        let interface = device.claim_interface(0).wait()?;
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
    /// This operation is synchronous and blocks using `.wait()` to retrieve the
    /// device list.
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
        Ok(nusb::list_devices()
            .wait()?
            .filter(|device| {
                device.vendor_id() == HACKRF_USB_VID
                    && device.product_id() == HACKRF_ONE_USB_PID
            })
            .collect::<Vec<DeviceInfo>>())
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
}
