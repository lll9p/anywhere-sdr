/// First set of orbital parameters from the GPS navigation message.
///
/// This structure contains the first set of parameters from the RINEX
/// navigation message that describe the satellite's orbit. These parameters
/// include the Issue of Data Ephemeris (IODE), the amplitude of the sine
/// harmonic correction term to the orbit radius (Crs), the mean motion
/// difference from computed value (Delta n), and the mean anomaly at reference
/// time (M0).
#[derive(Debug, Clone, Default)]
pub struct Orbit1 {
    /// Issue of Data Ephemeris (IODE)
    /// Used to match ephemeris sets and for timing
    pub iode: f64,

    /// Amplitude of the sine harmonic correction term to the orbit radius
    /// (meters)
    pub crs: f64,

    /// Mean motion difference from computed value (radians/sec)
    pub delta_n: f64,

    /// Mean anomaly at reference time (radians)
    pub m0: f64,
}

/// Second set of orbital parameters from the GPS navigation message.
///
/// This structure contains the second set of parameters from the RINEX
/// navigation message that describe the satellite's orbit. These parameters
/// include the amplitude of the cosine harmonic correction term to the argument
/// of latitude (Cuc), the eccentricity of the orbit (e), the amplitude of the
/// sine harmonic correction term to the argument of latitude (Cus), and the
/// square root of the semi-major axis (√A).
#[derive(Debug, Clone, Default)]
pub struct Orbit2 {
    /// Amplitude of the cosine harmonic correction term to the argument of
    /// latitude (radians)
    pub cuc: f64,

    /// Eccentricity of the orbit (dimensionless)
    pub ecc: f64,

    /// Amplitude of the sine harmonic correction term to the argument of
    /// latitude (radians)
    pub cus: f64,

    /// Square root of the semi-major axis (√meters)
    pub sqrta: f64,
}

/// Third set of orbital parameters from the GPS navigation message.
///
/// This structure contains the third set of parameters from the RINEX
/// navigation message that describe the satellite's orbit. These parameters
/// include the reference time for the ephemeris (TOE), the amplitude of the
/// cosine harmonic correction term to the angle of inclination (Cic), the
/// longitude of ascending node of orbit plane at weekly epoch (Ω), and the
/// amplitude of the sine harmonic correction term to the angle of inclination
/// (Cis).
#[derive(Debug, Clone, Default)]
pub struct Orbit3 {
    /// Reference time for the ephemeris (seconds of GPS week)
    pub toe: f64,

    /// Amplitude of the cosine harmonic correction term to the angle of
    /// inclination (radians)
    pub cic: f64,

    /// Longitude of ascending node of orbit plane at weekly epoch (radians)
    pub omega: f64,

    /// Amplitude of the sine harmonic correction term to the angle of
    /// inclination (radians)
    pub cis: f64,
}

/// Fourth set of orbital parameters from the GPS navigation message.
///
/// This structure contains the fourth set of parameters from the RINEX
/// navigation message that describe the satellite's orbit. These parameters
/// include the inclination angle at reference time (i0), the amplitude of the
/// cosine harmonic correction term to the orbit radius (Crc), the argument of
/// perigee (ω), and the rate of right ascension (Ω̇).
#[derive(Debug, Clone, Default)]
pub struct Orbit4 {
    /// Inclination angle at reference time (radians)
    pub i0: f64,

    /// Amplitude of the cosine harmonic correction term to the orbit radius
    /// (meters)
    pub crc: f64,

    /// Argument of perigee (radians)
    pub omega: f64,

    /// Rate of right ascension (radians/sec)
    pub omega_dot: f64,
}

/// Fifth set of orbital parameters from the GPS navigation message.
///
/// This structure contains the fifth set of parameters from the RINEX
/// navigation message that describe the satellite's orbit and signal
/// characteristics. These parameters include the rate of inclination angle (i̇),
/// the codes on the L2 channel, the GPS week number for the ephemeris reference
/// time, and the L2 P-code data flag.
#[derive(Debug, Clone, Default)]
pub struct Orbit5 {
    /// Rate of inclination angle (radians/sec)
    pub idot: f64,

    /// Codes on L2 channel
    /// Indicates which codes are transmitted on the L2 channel
    pub code_l2: f64,

    /// GPS Week number for the ephemeris reference time (TOE)
    pub week: f64,

    /// L2 P-code data flag
    /// Indicates whether P-code is being transmitted on L2
    pub l2_pseudorange: f64,
}

/// Sixth set of orbital parameters from the GPS navigation message.
///
/// This structure contains the sixth set of parameters from the RINEX
/// navigation message that describe the satellite's health and timing
/// characteristics. These parameters include the satellite vehicle accuracy
/// (URA), the satellite health status, the Total Group Delay (TGD) for timing
/// correction, and the Issue of Data Clock (IODC) for timing reference.
#[derive(Debug, Clone, Default)]
pub struct Orbit6 {
    /// Satellite vehicle accuracy (URA) in meters
    /// User Range Accuracy index for position error estimation
    pub sv_accuracy: f64,

    /// Satellite health status (MSB only)
    /// Indicates whether the satellite is operational
    pub sv_health: f64,

    /// Total Group Delay (TGD) in seconds
    /// Correction term for timing offset between L1 and L2 frequencies
    pub tgd: f64,

    /// Issue of Data Clock (IODC)
    /// Used to match clock parameters and for timing reference
    pub iodc: f64,
}

/// Seventh set of orbital parameters from the GPS navigation message.
///
/// This structure contains the seventh and final set of parameters from the
/// RINEX navigation message. It includes the transmission time of the message
/// and three spare fields that are reserved for future use or system-specific
/// parameters.
#[derive(Debug, Clone, Default)]
pub struct Orbit7 {
    /// Transmission time of message (seconds of GPS week)
    /// Derived from the Z-count in the Hand Over Word (HOW)
    pub tom: f64,

    /// Spare field 1 (reserved for future use)
    pub spare1: f64,

    /// Spare field 2 (reserved for future use)
    pub spare2: f64,

    /// Spare field 3 (reserved for future use)
    pub spare3: f64,
}

/// Converts an array of 4 floating-point values to Orbit1 parameters.
///
/// This implementation allows for easy creation of Orbit1 from raw RINEX data.
impl From<[f64; 4]> for Orbit1 {
    /// Creates an Orbit1 from an array of 4 floating-point values.
    ///
    /// # Arguments
    /// * `data` - Array containing [`iode`, `crs`, `delta_n`, `m0`]
    ///
    /// # Returns
    /// An `Orbit1` instance with the provided parameters
    fn from(data: [f64; 4]) -> Self {
        Self {
            iode: data[0],
            crs: data[1],
            delta_n: data[2],
            m0: data[3],
        }
    }
}
impl From<[f64; 4]> for Orbit2 {
    fn from(data: [f64; 4]) -> Self {
        Self {
            cuc: data[0],
            ecc: data[1],
            cus: data[2],
            sqrta: data[3],
        }
    }
}
impl From<[f64; 4]> for Orbit3 {
    fn from(data: [f64; 4]) -> Self {
        Self {
            toe: data[0],
            cic: data[1],
            omega: data[2],
            cis: data[3],
        }
    }
}
impl From<[f64; 4]> for Orbit4 {
    fn from(data: [f64; 4]) -> Self {
        Self {
            i0: data[0],
            crc: data[1],
            omega: data[2],
            omega_dot: data[3],
        }
    }
}
impl From<[f64; 4]> for Orbit5 {
    fn from(data: [f64; 4]) -> Self {
        Self {
            idot: data[0],
            code_l2: data[1],
            week: data[2],
            l2_pseudorange: data[3],
        }
    }
}
impl From<[f64; 4]> for Orbit6 {
    fn from(data: [f64; 4]) -> Self {
        Self {
            sv_accuracy: data[0],
            sv_health: data[1],
            tgd: data[2],
            iodc: data[3],
        }
    }
}
impl From<[f64; 4]> for Orbit7 {
    fn from(data: [f64; 4]) -> Self {
        Self {
            tom: data[0],
            spare1: data[1],
            spare2: data[2],
            spare3: data[3],
        }
    }
}
