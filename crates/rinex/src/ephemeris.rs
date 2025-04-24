mod orbit;
pub use self::orbit::*;
use crate::error::Error;

/// Satellite vehicle clock parameters from the RINEX navigation message.
///
/// This structure contains the clock correction parameters for a GPS satellite.
/// These parameters are used to compute the satellite clock offset from GPS
/// time.
///
/// The satellite clock correction is calculated as:
/// Δt = a₀ + a₁(t - t₀) + a₂(t - t₀)²
///
/// Where:
/// - a₀ is the clock bias (seconds)
/// - a₁ is the clock drift (seconds/second)
/// - a₂ is the clock drift rate (seconds/second²)
/// - t is the current time
/// - t₀ is the reference time
#[derive(Debug, Clone, Default)]
pub struct SvClock {
    /// Clock bias term a₀ (seconds)
    pub bias: f64,

    /// Clock drift term a₁ (seconds/second)
    pub drift: f64,

    /// Clock drift rate term a₂ (seconds/second²)
    pub drift_rate: f64,
}

impl SvClock {
    /// Creates a new satellite vehicle clock parameter set.
    ///
    /// # Arguments
    /// * `bias` - Clock bias term a₀ (seconds)
    /// * `drift` - Clock drift term a₁ (seconds/second)
    /// * `drift_rate` - Clock drift rate term a₂ (seconds/second²)
    ///
    /// # Returns
    /// A new `SvClock` instance with the provided parameters
    pub fn new(bias: f64, drift: f64, drift_rate: f64) -> Self {
        Self {
            bias,
            drift,
            drift_rate,
        }
    }
}

/// Ephemeris data for a single GPS satellite from a RINEX navigation message.
///
/// This structure contains the complete set of orbital and clock parameters
/// for a GPS satellite, as broadcast in the navigation message. These
/// parameters allow the receiver to compute the satellite's position, velocity,
/// and clock correction at any time within the validity period of the
/// ephemeris.
///
/// The ephemeris data is organized into clock parameters and seven sets of
/// orbital parameters, following the standard RINEX navigation message format.
#[derive(Debug, Clone, Default)]
pub struct Ephemeris {
    /// Satellite PRN (Pseudo-Random Noise) number (1-32)
    pub prn: usize,

    /// Reference time for the clock parameters (Time of Clock)
    pub time_of_clock: jiff::Timestamp,

    /// Satellite clock correction parameters
    pub sv_clock: SvClock,

    /// First set of orbital parameters (IODE, Crs, Delta n, M0)
    pub orbit1: Orbit1,

    /// Second set of orbital parameters (Cuc, e, Cus, sqrt(A))
    pub orbit2: Orbit2,

    /// Third set of orbital parameters (Toe, Cic, OMEGA, Cis)
    pub orbit3: Orbit3,

    /// Fourth set of orbital parameters (i0, Crc, omega, OMEGA DOT)
    pub orbit4: Orbit4,

    /// Fifth set of orbital parameters (IDOT, L2 codes, GPS week, L2 P data
    /// flag)
    pub orbit5: Orbit5,

    /// Sixth set of orbital parameters (SV accuracy, SV health, TGD, IODC)
    pub orbit6: Orbit6,

    /// Seventh set of orbital parameters (Transmission time, spare fields)
    pub orbit7: Orbit7,
}

/// Builder for creating Ephemeris objects incrementally.
///
/// This builder pattern implementation allows for the gradual construction
/// of an Ephemeris object as data is parsed from a RINEX navigation file.
/// Each component of the ephemeris can be set individually, and the final
/// object is created only when all required components are present.
#[derive(Debug, Default)]
pub struct EphemerisBuilder {
    /// Satellite PRN number
    prn: Option<usize>,

    /// Reference time for the clock parameters
    time_of_clock: Option<jiff::Timestamp>,

    /// Satellite clock correction parameters
    sv_clock: Option<SvClock>,

    /// First set of orbital parameters
    orbit1: Option<Orbit1>,

    /// Second set of orbital parameters
    orbit2: Option<Orbit2>,

    /// Third set of orbital parameters
    orbit3: Option<Orbit3>,

    /// Fourth set of orbital parameters
    orbit4: Option<Orbit4>,

    /// Fifth set of orbital parameters
    orbit5: Option<Orbit5>,

    /// Sixth set of orbital parameters
    orbit6: Option<Orbit6>,

    /// Seventh set of orbital parameters
    orbit7: Option<Orbit7>,
}

impl EphemerisBuilder {
    /// Creates a new empty `EphemerisBuilder`.
    ///
    /// # Returns
    /// A new `EphemerisBuilder` instance with all fields set to None
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the satellite PRN number.
    ///
    /// # Arguments
    /// * `prn` - Satellite PRN (Pseudo-Random Noise) number (1-32)
    pub fn set_prn(&mut self, prn: usize) {
        self.prn.replace(prn);
    }

    /// Sets the reference time for the clock parameters.
    ///
    /// # Arguments
    /// * `time_of_clock` - Time of Clock (TOC) timestamp
    pub fn set_time_of_clock(&mut self, time_of_clock: jiff::Timestamp) {
        self.time_of_clock.replace(time_of_clock);
    }

    /// Sets the satellite clock correction parameters.
    ///
    /// # Arguments
    /// * `sv_clock` - Satellite vehicle clock parameters
    pub fn set_sv_clock(&mut self, sv_clock: SvClock) {
        self.sv_clock.replace(sv_clock);
    }

    /// Sets the first set of orbital parameters.
    ///
    /// # Arguments
    /// * `orbit1` - First set of orbital parameters (IODE, Crs, Delta n, M0)
    pub fn set_orbit1(&mut self, orbit1: Orbit1) {
        self.orbit1.replace(orbit1);
    }

    /// Sets the second set of orbital parameters.
    ///
    /// # Arguments
    /// * `orbit2` - Second set of orbital parameters (Cuc, e, Cus, sqrt(A))
    pub fn set_orbit2(&mut self, orbit2: Orbit2) {
        self.orbit2.replace(orbit2);
    }

    /// Sets the third set of orbital parameters.
    ///
    /// # Arguments
    /// * `orbit3` - Third set of orbital parameters (Toe, Cic, OMEGA, Cis)
    pub fn set_orbit3(&mut self, orbit3: Orbit3) {
        self.orbit3.replace(orbit3);
    }

    /// Sets the fourth set of orbital parameters.
    ///
    /// # Arguments
    /// * `orbit4` - Fourth set of orbital parameters (i0, Crc, omega, OMEGA
    ///   DOT)
    pub fn set_orbit4(&mut self, orbit4: Orbit4) {
        self.orbit4.replace(orbit4);
    }

    /// Sets the fifth set of orbital parameters.
    ///
    /// # Arguments
    /// * `orbit5` - Fifth set of orbital parameters (IDOT, L2 codes, GPS week,
    ///   L2 P data flag)
    pub fn set_orbit5(&mut self, orbit5: Orbit5) {
        self.orbit5.replace(orbit5);
    }

    /// Sets the sixth set of orbital parameters.
    ///
    /// # Arguments
    /// * `orbit6` - Sixth set of orbital parameters (SV accuracy, SV health,
    ///   TGD, IODC)
    pub fn set_orbit6(&mut self, orbit6: Orbit6) {
        self.orbit6.replace(orbit6);
    }

    /// Sets the seventh set of orbital parameters.
    ///
    /// # Arguments
    /// * `orbit7` - Seventh set of orbital parameters (Transmission time, spare
    ///   fields)
    pub fn set_orbit7(&mut self, orbit7: Orbit7) {
        self.orbit7.replace(orbit7);
    }

    /// Builds an Ephemeris object from the collected parameters.
    ///
    /// This method consumes all the parameters that have been set in the
    /// builder and creates a complete Ephemeris object. It will return an
    /// error if any required parameter is missing.
    ///
    /// # Returns
    /// * `Ok(Ephemeris)` - A complete Ephemeris object with all required
    ///   parameters
    /// * `Err(Error)` - If any required parameter is missing
    ///
    /// # Errors
    /// * Returns an error if any of the required parameters has not been set
    pub fn build(&mut self) -> Result<Ephemeris, Error> {
        // Helper function to take a value from an Option and return an error if
        // None
        fn take<T>(v: &mut Option<T>, msg: &str) -> Result<T, Error> {
            v.take().ok_or_else(|| Error::EphemerisBuilder(msg.into()))
        }

        let ephemeris = Ephemeris {
            prn: take(&mut self.prn, "prn is none")?,
            time_of_clock: take(
                &mut self.time_of_clock,
                "time_of_clock is none",
            )?,
            sv_clock: take(&mut self.sv_clock, "sv_clock is none")?,
            orbit1: take(&mut self.orbit1, "orbit1 is none")?,
            orbit2: take(&mut self.orbit2, "orbit2 is none")?,
            orbit3: take(&mut self.orbit3, "orbit3 is none")?,
            orbit4: take(&mut self.orbit4, "orbit4 is none")?,
            orbit5: take(&mut self.orbit5, "orbit5 is none")?,
            orbit6: take(&mut self.orbit6, "orbit6 is none")?,
            orbit7: take(&mut self.orbit7, "orbit7 is none")?,
        };
        Ok(ephemeris)
    }
}
