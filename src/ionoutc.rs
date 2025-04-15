#[allow(non_snake_case)]
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct ionoutc_t {
    pub enable: bool,
    pub vflg: bool,
    pub alpha0: f64,
    pub alpha1: f64,
    pub alpha2: f64,
    pub alpha3: f64,
    pub beta0: f64,
    pub beta1: f64,
    pub beta2: f64,
    pub beta3: f64,
    pub A0: f64,
    pub A1: f64,
    pub dtls: i32,
    pub tot: i32,
    pub wnt: i32,
    pub dtlsf: i32,
    pub dn: i32,
    pub wnlsf: i32,
    // enable custom leap event
    pub leapen: i32,
}
