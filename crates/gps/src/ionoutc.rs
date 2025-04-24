/// Ionospheric and UTC parameters from the GPS navigation message.
///
/// This structure contains parameters for the Klobuchar ionospheric model and
/// UTC time conversion. These parameters are broadcast in the GPS navigation
/// message and are used to:
///
/// 1. Calculate ionospheric delay corrections for single-frequency receivers
/// 2. Convert between GPS time and UTC time
///
/// The ionospheric model uses alpha and beta parameters to estimate the delay
/// caused by the ionosphere, which varies with time of day, receiver location,
/// and satellite elevation angle.
#[allow(non_snake_case)]
#[derive(Debug, Default)]
pub struct IonoUtc {
    /// Flag to enable/disable ionospheric corrections
    pub enable: bool,

    /// Validity flag for UTC parameters
    pub vflg: bool,

    /// Alpha parameter constant term (seconds)
    /// Used in the Klobuchar ionospheric model
    pub alpha0: f64,

    /// Alpha parameter 1st order term (seconds/semi-circle)
    /// Used in the Klobuchar ionospheric model
    pub alpha1: f64,

    /// Alpha parameter 2nd order term (seconds/semi-circle²)
    /// Used in the Klobuchar ionospheric model
    pub alpha2: f64,

    /// Alpha parameter 3rd order term (seconds/semi-circle³)
    /// Used in the Klobuchar ionospheric model
    pub alpha3: f64,

    /// Beta parameter constant term (seconds)
    /// Used in the Klobuchar ionospheric model
    pub beta0: f64,

    /// Beta parameter 1st order term (seconds/semi-circle)
    /// Used in the Klobuchar ionospheric model
    pub beta1: f64,

    /// Beta parameter 2nd order term (seconds/semi-circle²)
    /// Used in the Klobuchar ionospheric model
    pub beta2: f64,

    /// Beta parameter 3rd order term (seconds/semi-circle³)
    /// Used in the Klobuchar ionospheric model
    pub beta3: f64,

    /// UTC constant term of polynomial (seconds)
    /// Used for GPS to UTC time conversion
    pub A0: f64,

    /// UTC 1st order term of polynomial (seconds/second)
    /// Used for GPS to UTC time conversion
    pub A1: f64,

    /// Delta time due to leap seconds (seconds)
    /// Current difference between GPS time and UTC
    pub dtls: i32,

    /// Reference time of UTC parameters (seconds of GPS week)
    pub tot: i32,

    /// UTC reference week number (GPS week)
    pub week_number: i32,

    /// Future delta time due to leap seconds (seconds)
    /// For upcoming leap second changes
    pub dtlsf: i32,

    /// Day number when the future leap second becomes effective
    /// (1-7, where 1=Sunday and 7=Saturday)
    pub day_number: i32,

    /// Future week number when the leap second becomes effective (GPS week)
    pub wnlsf: i32,

    /// Flag to enable custom leap second event
    pub leapen: i32,
}
impl IonoUtc {
    /// Populates ionospheric and UTC parameters from a RINEX navigation file.
    ///
    /// This method extracts the ionospheric model parameters (alpha, beta) and
    /// UTC conversion parameters from a parsed RINEX navigation file. It also
    /// sets the validity flag based on the reference time.
    ///
    /// # Arguments
    /// * `rinex` - A reference to a parsed RINEX navigation file
    pub fn read_from_rinex(&mut self, rinex: &rinex::Rinex) {
        // Extract ionospheric model parameters (Klobuchar model)
        self.alpha0 = rinex.ion_alpha[0];
        self.alpha1 = rinex.ion_alpha[1];
        self.alpha2 = rinex.ion_alpha[2];
        self.alpha3 = rinex.ion_alpha[3];
        self.beta0 = rinex.ion_beta[0];
        self.beta1 = rinex.ion_beta[1];
        self.beta2 = rinex.ion_beta[2];
        self.beta3 = rinex.ion_beta[3];

        // Extract UTC parameters
        self.A0 = rinex.delta_utc.a0;
        self.A1 = rinex.delta_utc.a1;
        self.tot = rinex.delta_utc.time;
        self.week_number = rinex.delta_utc.week;
        self.dtls = rinex.leap_seconds;

        // Set validity flag (tot should be a multiple of 4096 seconds)
        self.vflg = self.tot % 4096 == 0;
    }
}
