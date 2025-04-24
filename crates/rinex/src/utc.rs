/// Parameters for converting GPS time to UTC time.
///
/// This structure contains the almanac parameters needed to compute the
/// conversion between GPS time and Coordinated Universal Time (UTC).
/// GPS time is a continuous time scale that doesn't include leap seconds,
/// while UTC does include leap seconds to stay synchronized with Earth's
/// rotation.
///
/// The conversion formula is:
/// UTC = GPS time - `ΔtUTC`
/// where `ΔtUTC` = A0 + A1 × (t - t0)
///
/// These parameters are typically extracted from the navigation message
/// broadcast by GPS satellites.
#[derive(Debug)]
pub struct DeltaUtc {
    /// Constant term of the UTC offset polynomial (seconds)
    pub a0: f64,

    /// First-order term of the UTC offset polynomial (seconds/second)
    pub a1: f64,

    /// Reference time for UTC data (seconds of GPS week)
    pub time: i32,

    /// UTC reference week number (GPS week)
    pub week: i32,
}

impl DeltaUtc {
    /// Creates a new `DeltaUtc` instance with the specified parameters.
    ///
    /// # Arguments
    /// * `a0` - Constant term of the UTC offset polynomial (seconds)
    /// * `a1` - First-order term of the UTC offset polynomial (seconds/second)
    /// * `time` - Reference time for UTC data (seconds of GPS week)
    /// * `week` - UTC reference week number (GPS week)
    ///
    /// # Returns
    /// A new `DeltaUtc` instance with the provided parameters
    pub fn new(a0: f64, a1: f64, time: i32, week: i32) -> Self {
        Self { a0, a1, time, week }
    }
}
