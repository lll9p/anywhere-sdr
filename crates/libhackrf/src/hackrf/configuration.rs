/// Pure validation and wire preparation for RF and sample-rate parameters.
mod parameters;

use self::parameters::{
    PreparedSampleRate, prepare_rf_frequency, prepare_sample_rate_auto,
    prepare_sample_rate_manual,
};
use super::{HackRF, control::decode_gain_ack};
use crate::{enums::Request, error::Error};

impl HackRF {
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
    /// Returns [`Error::Argument`] unless `hz` is within the supported
    /// 1 MHz through 6 GHz range. USB communication failures are also
    /// returned.
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
        let prepared = prepare_rf_frequency(hz)?;
        self.write_control(Request::SetFreq, 0, 0, &prepared.payload)
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
    /// Returns [`Error::Argument`] unless `divider` is within `1..=31` and
    /// the exact rational rate `freq_hz / divider` is within 2 MHz through
    /// 20 MHz. USB communication failures are also returned.
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
        let prepared = prepare_sample_rate_manual(freq_hz, divider)?;
        self.submit_sample_rate(prepared)
    }

    /// For anti-aliasing, the baseband filter bandwidth is automatically set to
    /// the widest available setting that is no more than 75% of the sample
    /// rate, except when the minimum 1.75 MHz setting is required. This happens
    /// every time the sample rate is set. If you want to
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
    /// Returns [`Error::Argument`] unless `freq` is finite and within 2 MHz
    /// through 20 MHz. USB communication failures are also returned.
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
        let prepared = prepare_sample_rate_auto(freq)?;
        self.submit_sample_rate(prepared)
    }

    /// Submits a validated sample-rate request before its prepared filter.
    fn submit_sample_rate(
        &mut self, prepared: PreparedSampleRate,
    ) -> Result<(), Error> {
        self.write_control(Request::SampleRateSet, 0, 0, &prepared.payload)?;
        self.set_baseband_filter_bandwidth(prepared.baseband_filter_hz)
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
            let acknowledgement = self.read_control_exact::<1>(
                Request::SetLnaGain,
                0,
                value & !0x07,
            )?;
            decode_gain_ack(acknowledgement)
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
            let acknowledgement = self.read_control_exact::<1>(
                Request::SetVgaGain,
                0,
                value & !0b1,
            )?;
            decode_gain_ack(acknowledgement)
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
            let acknowledgement =
                self.read_control_exact::<1>(Request::SetTxvgaGain, 0, value)?;
            decode_gain_ack(acknowledgement)
        }
    }
}
