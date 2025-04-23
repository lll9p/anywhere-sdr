/// Almanac parameters to compute time in UTC
#[derive(Debug)]
pub struct DeltaUtc {
    /// terms of polynomial A0
    pub a0: f64,
    /// terms of polynomial A1
    pub a1: f64,
    /// reference time for UTC data
    pub time: i32,
    /// UTC reference week number
    pub week: i32,
}
impl DeltaUtc {
    pub fn new(a0: f64, a1: f64, time: i32, week: i32) -> Self {
        Self { a0, a1, time, week }
    }
}
