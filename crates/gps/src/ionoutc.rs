#[allow(non_snake_case)]
// #[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct IonoUtc {
    pub enable: bool,
    pub vflg: bool,
    /// Alpha parameter constant term
    pub alpha0: f64,

    /// Alpha parameter 1st order term
    pub alpha1: f64,

    /// Alpha parameter 2nd order term
    pub alpha2: f64,

    /// Alpha parameter 3rd order term
    pub alpha3: f64,

    /// Beta parameter constant term
    pub beta0: f64,

    /// Beta parameter 1st order term
    pub beta1: f64,

    /// Beta parameter 2nd order term
    pub beta2: f64,

    /// Beta parameter 3rd order term
    pub beta3: f64,

    /// UTC constant term of polynomial (s)
    pub A0: f64,

    /// UTC 1st order term of polynomial (s)
    pub A1: f64,

    /// Delta time due to leap seconds
    pub dtls: i32,

    /// Reference time of UTC parameters (s)
    pub tot: i32,

    /// UTC reference week number
    pub wnt: i32,

    /// Future delta time due to leap seconds
    pub dtlsf: i32,

    /// Day number (the range is 1 to 7 where Sunday = 1 and Saturday = 7)
    pub dn: i32,

    /// Future week number
    pub wnlsf: i32,

    // enable custom leap event
    pub leapen: i32,
}
