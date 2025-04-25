/// USB Vendor ID for `HackRF` devices
pub const HACKRF_USB_VID: u16 = 0x1D50;

/// USB Product ID for `HackRF` One devices
pub const HACKRF_ONE_USB_PID: u16 = 0x6089;

/// USB endpoint address for receiving data from `HackRF`
pub const HACKRF_RX_ENDPOINT_ADDRESS: u8 = 0x81;

/// USB endpoint address for transmitting data to `HackRF`
pub const HACKRF_TX_ENDPOINT_ADDRESS: u8 = 0x02;

/// Size of the transfer buffer for bulk USB transfers (256 KB)
pub const HACKRF_TRANSFER_BUFFER_SIZE: usize = 2 * 128 * 1024;

/// Size of the device buffer (32 KB)
pub const HACKRF_DEVICE_BUFFER_SIZE: usize = 32 * 1024;

/// Conversion constant for MHz to Hz (1,000,000)
pub const MHZ: u64 = 1_000_000;

/// Available baseband filter bandwidths for MAX2837 transceiver (in Hz)
///
/// These values represent the possible filter bandwidths that can be set
/// on the MAX2837 RF transceiver chip used in `HackRF` devices.
pub const MAX2837: [u32; 17] = [
    1_750_000, 2_500_000, 3_500_000, 5_000_000, 5_500_000, 6_000_000,
    7_000_000, 8_000_000, 9_000_000, 10_000_000, 12_000_000, 14_000_000,
    15_000_000, 20_000_000, 24_000_000, 28_000_000, 0,
];
