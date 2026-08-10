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
#[derive(Clone, Debug, Default)]
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
        let [alpha0, alpha1, alpha2, alpha3] =
            rinex.ion_alpha.unwrap_or_default();
        self.alpha0 = alpha0;
        self.alpha1 = alpha1;
        self.alpha2 = alpha2;
        self.alpha3 = alpha3;

        let [beta0, beta1, beta2, beta3] = rinex.ion_beta.unwrap_or_default();
        self.beta0 = beta0;
        self.beta1 = beta1;
        self.beta2 = beta2;
        self.beta3 = beta3;

        if let Some(delta_utc) = &rinex.delta_utc {
            self.A0 = delta_utc.a0;
            self.A1 = delta_utc.a1;
            self.tot = delta_utc.time;
            self.week_number = delta_utc.week;
        } else {
            self.A0 = 0.0;
            self.A1 = 0.0;
            self.tot = 0;
            self.week_number = 0;
        }
        self.dtls = rinex.leap_seconds.unwrap_or_default();

        self.vflg = rinex.ion_alpha.is_some()
            && rinex.ion_beta.is_some()
            && rinex.delta_utc.is_some()
            && self.tot % 4096 == 0;
    }
}

#[cfg(test)]
mod tests {
    use rinex::{Rinex, ephemeris::Ephemeris, utc::DeltaUtc};

    use super::IonoUtc;

    #[test]
    fn missing_optional_rinex_corrections_keep_defaults_and_are_invalid() {
        let mut iono_utc = IonoUtc {
            alpha0: 1.0,
            beta0: 2.0,
            A0: 3.0,
            dtls: 18,
            tot: 4096,
            week_number: 2_317,
            vflg: true,
            ..IonoUtc::default()
        };
        iono_utc.read_from_rinex(&rinex(None, None, None, None));

        assert_close(iono_utc.alpha0, 0.0);
        assert_close(iono_utc.beta0, 0.0);
        assert_close(iono_utc.A0, 0.0);
        assert_eq!(iono_utc.dtls, 0);
        assert_eq!(iono_utc.tot, 0);
        assert_eq!(iono_utc.week_number, 0);
        assert!(!iono_utc.vflg);
    }

    #[test]
    fn complete_rinex_corrections_copy_exact_values_and_set_validity() {
        let mut iono_utc = IonoUtc::default();
        iono_utc.read_from_rinex(&rinex(
            Some([1.0, 2.0, 3.0, 4.0]),
            Some([5.0, 6.0, 7.0, 8.0]),
            Some(DeltaUtc::new(9.0, 10.0, 4096, 2_317)),
            Some(18),
        ));

        for (actual, expected) in [
            iono_utc.alpha0,
            iono_utc.alpha1,
            iono_utc.alpha2,
            iono_utc.alpha3,
        ]
        .into_iter()
        .zip([1.0, 2.0, 3.0, 4.0])
        {
            assert_close(actual, expected);
        }
        for (actual, expected) in [
            iono_utc.beta0,
            iono_utc.beta1,
            iono_utc.beta2,
            iono_utc.beta3,
        ]
        .into_iter()
        .zip([5.0, 6.0, 7.0, 8.0])
        {
            assert_close(actual, expected);
        }
        assert_close(iono_utc.A0, 9.0);
        assert_close(iono_utc.A1, 10.0);
        assert_eq!(iono_utc.tot, 4096);
        assert_eq!(iono_utc.week_number, 2_317);
        assert_eq!(iono_utc.dtls, 18);
        assert!(iono_utc.vflg);
    }

    #[test]
    fn validity_requires_model_records_but_not_leap_seconds() {
        for (has_alpha, has_beta, has_delta_utc, expected_valid) in [
            (false, true, true, false),
            (true, false, true, false),
            (true, true, false, false),
            (true, true, true, true),
        ] {
            let mut iono_utc = IonoUtc::default();
            iono_utc.read_from_rinex(&rinex(
                has_alpha.then_some([1.0; 4]),
                has_beta.then_some([2.0; 4]),
                has_delta_utc.then(|| DeltaUtc::new(3.0, 4.0, 4096, 2_317)),
                None,
            ));
            assert_eq!(iono_utc.vflg, expected_valid);
            assert_eq!(iono_utc.dtls, 0);
        }

        let mut iono_utc = IonoUtc::default();
        iono_utc.read_from_rinex(&rinex(
            Some([1.0; 4]),
            Some([2.0; 4]),
            Some(DeltaUtc::new(3.0, 4.0, 4095, 2_317)),
            Some(18),
        ));
        assert!(!iono_utc.vflg);
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < f64::EPSILON);
    }

    fn rinex(
        ion_alpha: Option<[f64; 4]>, ion_beta: Option<[f64; 4]>,
        delta_utc: Option<DeltaUtc>, leap_seconds: Option<i32>,
    ) -> Rinex {
        Rinex {
            version: "2.11".to_string(),
            type_: "N".to_string(),
            program: "test".to_string(),
            agency: "test".to_string(),
            update: "test".to_string(),
            comments: Vec::new(),
            ion_alpha,
            ion_beta,
            delta_utc,
            leap_seconds,
            ephemerides: Vec::<Ephemeris>::new(),
        }
    }
}
