#[derive(Copy, Clone)]
#[repr(C)]
pub struct ionoutc_t {
    pub enable: i32,
    pub vflg: i32,
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
    pub leapen: i32,
}
