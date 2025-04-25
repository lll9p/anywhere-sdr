/// Current operating mode of the `HackRF` device
#[derive(Debug)]
pub enum DeviceMode {
    /// Device is not transmitting or receiving
    Off,
    /// Device is in transmit mode
    Tx,
    /// Device is in receive mode
    Rx,
}

/// Hardware synchronization mode for the `HackRF` device
#[repr(u8)]
pub enum SyncMode {
    /// Synchronization is disabled
    Off = 0,
    /// Synchronization is enabled
    On = 1,
}

impl From<SyncMode> for u8 {
    fn from(tm: SyncMode) -> Self {
        tm as u8
    }
}

/// Transceiver operating mode for the `HackRF` device
#[repr(u8)]
pub enum TransceiverMode {
    /// Transceiver is disabled
    Off = 0,
    /// Transceiver is in receive mode
    Receive = 1,
    /// Transceiver is in transmit mode
    Transmit = 2,
    /// Transceiver is in sweep spectrum mode
    Ss = 3,
    /// Transceiver is in CPLD update mode
    CpldUpdate = 4,
    /// Transceiver is in receive sweep mode
    RxSweep = 5,
}

impl From<TransceiverMode> for u8 {
    fn from(tm: TransceiverMode) -> Self {
        tm as u8
    }
}

impl From<TransceiverMode> for u16 {
    fn from(tm: TransceiverMode) -> Self {
        tm as u16
    }
}

/// USB control request codes for `HackRF` device operations
///
/// These values correspond to the vendor-specific USB control requests
/// used to communicate with the `HackRF` device.
#[allow(dead_code)]
#[repr(u8)]
pub enum Request {
    /// Set the transceiver mode (off, receive, transmit, etc.)
    SetTransceiverMode = 1,
    /// Write to the MAX2837 RF transceiver chip
    Max2837Write = 2,
    /// Read from the MAX2837 RF transceiver chip
    Max2837Read = 3,
    /// Write to the `Si5351C` clock generator chip
    Si5351CWrite = 4,
    /// Read from the `Si5351C` clock generator chip
    Si5351CRead = 5,
    /// Set the sample rate
    SampleRateSet = 6,
    /// Set the baseband filter bandwidth
    BasebandFilterBandwidthSet = 7,
    /// Write to the RFFC5071 mixer/synthesizer chip
    Rffc5071Write = 8,
    /// Read from the RFFC5071 mixer/synthesizer chip
    Rffc5071Read = 9,
    /// Erase the SPI flash memory
    SpiflashErase = 10,
    /// Write to the SPI flash memory
    SpiflashWrite = 11,
    /// Read from the SPI flash memory
    SpiflashRead = 12,
    /// Read the board ID
    BoardIdRead = 14,
    /// Read the firmware version string
    VersionStringRead = 15,
    /// Set the frequency
    SetFreq = 16,
    /// Enable/disable the RF amplifier
    AmpEnable = 17,
    /// Read the board part ID and serial number
    BoardPartidSerialnoRead = 18,
    /// Set the LNA gain
    SetLnaGain = 19,
    /// Set the VGA gain
    SetVgaGain = 20,
    /// Set the TX VGA gain
    SetTxvgaGain = 21,
    /// Enable/disable the antenna port power
    AntennaEnable = 23,
    /// Set the frequency with explicit parameters
    SetFreqExplicit = 24,
    /// USB WCID vendor request
    UsbWcidVendorReq = 25,
    /// Initialize frequency sweep
    InitSweep = 26,
    /// Get Operacake boards
    OperacakeGetBoards = 27,
    /// Set Operacake ports
    OperacakeSetPorts = 28,
    /// Set hardware sync mode
    SetHwSyncMode = 29,
    /// Reset the device
    Reset = 30,
    /// Set Operacake frequency ranges
    OperacakeSetRanges = 31,
    /// Enable/disable the clock output
    ClkoutEnable = 32,
    /// Get SPI flash status
    SpiflashStatus = 33,
    /// Clear SPI flash status
    SpiflashClearStatus = 34,
    /// Test Operacake GPIO
    OperacakeGpioTest = 35,
    /// Get CPLD checksum
    CpldChecksum = 36,
    /// Enable/disable the user interface
    UiEnable = 37,
}

impl From<Request> for u8 {
    fn from(r: Request) -> Self {
        r as u8
    }
}
