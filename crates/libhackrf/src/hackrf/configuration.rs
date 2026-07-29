use super::{HackRF, control::decode_gain_ack};
use crate::{
    constants::{MAX2837, MHZ},
    enums::Request,
    error::Error,
};

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
