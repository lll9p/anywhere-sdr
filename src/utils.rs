#[inline]
pub fn cos(value: f64) -> f64 {
    value.cos()
}

#[inline]
pub fn atan2(y: f64, x: f64) -> f64 {
    y.atan2(x)
}

#[inline]
pub fn sin(value: f64) -> f64 {
    value.sin()
}

#[inline]
pub fn pow(base: f64, exponent: f64) -> f64 {
    base.powf(exponent)
}

#[inline]
pub fn sqrt(value: f64) -> f64 {
    value.sqrt()
}

#[inline]
pub fn fabs(value: f64) -> f64 {
    value.abs()
}

#[inline]
pub fn floor(value: f64) -> f64 {
    value.floor()
}

#[inline]
pub fn round(value: f64) -> f64 {
    value.round()
}

#[inline]
pub fn iqbuf_to_u8(buf: &[i16]) -> Vec<u8> {
    buf.iter().map(|v| *v as u8).collect()
}
#[inline]
pub fn iq8buf_to_u8(buf: &[i8]) -> Vec<u8> {
    buf.iter().map(|v| *v as u8).collect()
}
