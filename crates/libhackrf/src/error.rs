use thiserror::Error;

/// Errors that can occur when interacting with `HackRF` devices
#[derive(Error, Debug)]
pub enum Error {
    /// USB communication error
    #[error("USB error: {0}")]
    Usb(#[from] nusb::Error),

    /// No `HackRF` device was found
    #[error("No HackRF device found")]
    InvalidDevice,

    /// The specified serial number does not match any connected device
    #[error("No HackRF device with serial number '{0}' found")]
    InvalidSerialNumber(String),

    /// The device firmware version is too old for the requested operation
    #[error(
        "Device firmware version {device} is older than required version \
         {minimal}"
    )]
    VersionMismatch {
        /// Current device firmware version
        device: u16,
        /// Minimum required firmware version
        minimal: u16,
    },

    /// USB transfer error
    #[error("USB transfer error: {0}")]
    Transfer(#[from] nusb::transfer::TransferError),

    /// USB control transfer error with mismatched data length
    #[error(
        "USB control transfer error ({direction:?}): transferred {actual} \
         bytes, expected {expected} bytes"
    )]
    ControlTransfer {
        /// Direction of the transfer (In/Out)
        direction: nusb::transfer::Direction,
        /// Actual number of bytes transferred
        actual: usize,
        /// Expected number of bytes to transfer
        expected: usize,
    },

    /// Error converting between slice and array
    #[error("Error converting between slice and array: {0}")]
    TryFromSlice(#[from] std::array::TryFromSliceError),

    /// Invalid argument provided to a function
    #[error("Invalid argument provided")]
    Argument,

    /// Error converting from UTF-8
    #[error("UTF-8 conversion error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),

    /// Formatting error
    #[error("Formatting error: {0}")]
    Fmt(#[from] std::fmt::Error),
}
