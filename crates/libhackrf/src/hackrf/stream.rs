use nusb::Endpoint;

use super::HackRF;
use crate::{
    constants::{HACKRF_RX_ENDPOINT_ADDRESS, HACKRF_TX_ENDPOINT_ADDRESS},
    enums::{DeviceMode, Request, TransceiverMode},
    error::Error,
};

impl HackRF {
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

    /// Gets an endpoint for receiving data from the device
    ///
    /// This method returns a `nusb::Endpoint` that can be used to receive data
    /// from the device in receive mode.
    ///
    /// # Returns
    ///
    /// A `nusb::Endpoint` for bulk IN transfers.
    pub fn rx_queue(
        &mut self,
    ) -> Result<Endpoint<nusb::transfer::Bulk, nusb::transfer::In>, Error> {
        Ok(self.interface.endpoint(HACKRF_RX_ENDPOINT_ADDRESS)?)
    }

    /// Gets an endpoint for sending data to the device
    ///
    /// This method returns a `nusb::Endpoint` that can be used to send data to
    /// the device in transmit mode.
    ///
    /// # Returns
    ///
    /// A `nusb::Endpoint` for bulk OUT transfers.
    pub fn tx_queue(
        &mut self,
    ) -> Result<Endpoint<nusb::transfer::Bulk, nusb::transfer::Out>, Error>
    {
        Ok(self.interface.endpoint(HACKRF_TX_ENDPOINT_ADDRESS)?)
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
