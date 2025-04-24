use thiserror::Error;

/// Custom error type for the GPS crate
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid navigation file: {0}")]
    NavigationFile(String),

    #[error("No ephemeris available")]
    NoEphemeris,

    #[error("No current set of ephemerides found")]
    NoCurrentEphemerides,

    #[error("Invalid sampling frequency")]
    InvalidSamplingFrequency,

    #[error("Invalid I/Q data format")]
    InvalidDataFormat,

    #[error("Cannot set position(s) more than once")]
    DuplicatePositionSetting,

    #[error("Invalid duration")]
    InvalidDuration,

    #[error("Invalid start time")]
    InvalidStartTime,

    #[error("Invalid GPS day number")]
    InvalidGpsDay,

    #[error("Invalid GPS week number")]
    InvalidGpsWeek,

    #[error("Invalid delta leap second")]
    InvalidDeltaLeapSecond,

    #[error("Wrong positions")]
    WrongPositions,

    #[error("Data format not set")]
    DataFormatNotSet,

    #[error("Navigation data not set")]
    NavigationNotSet,

    #[error("IQWriter not initialized")]
    IQWriterNotInitialized,

    #[error("Signal generator not initialized")]
    NotInitialized,

    #[error("RINEX error: {0}")]
    Rinex(#[from] rinex::error::Error),

    #[error("Time parsing error: {0}")]
    TimeParseError(#[from] jiff::Error),

    #[error("UTF-8 conversion error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),

    #[error("Float parsing error: {0}")]
    ParseFloatError(#[from] std::num::ParseFloatError),

    #[error("Integer parsing error: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),

    #[error("Parsing error: {0}")]
    ParsingError(String),

    #[error("Unknown error")]
    Unknown,
}

impl Error {
    /// Create a new error with a message
    #[inline]
    pub fn msg(message: impl Into<String>) -> Self {
        Error::ParsingError(message.into())
    }

    /// Create a new error for invalid navigation data
    #[inline]
    pub fn invalid_navigation(message: impl Into<String>) -> Self {
        Error::NavigationFile(message.into())
    }

    /// Create a new error for invalid data format
    #[inline]
    pub fn invalid_data_format() -> Self {
        Error::InvalidDataFormat
    }

    /// Create a new error for invalid sampling frequency
    #[inline]
    pub fn invalid_sampling_frequency() -> Self {
        Error::InvalidSamplingFrequency
    }

    /// Create a new error for duplicate position setting
    #[inline]
    pub fn duplicate_position() -> Self {
        Error::DuplicatePositionSetting
    }

    /// Create a new error for invalid duration
    #[inline]
    pub fn invalid_duration() -> Self {
        Error::InvalidDuration
    }

    /// Create a new error for invalid start time
    #[inline]
    pub fn invalid_start_time() -> Self {
        Error::InvalidStartTime
    }

    /// Create a new error for invalid GPS day number
    #[inline]
    pub fn invalid_gps_day() -> Self {
        Error::InvalidGpsDay
    }

    /// Create a new error for invalid GPS week number
    #[inline]
    pub fn invalid_gps_week() -> Self {
        Error::InvalidGpsWeek
    }

    /// Create a new error for invalid delta leap second
    #[inline]
    pub fn invalid_delta_leap_second() -> Self {
        Error::InvalidDeltaLeapSecond
    }

    /// Create a new error for wrong positions
    #[inline]
    pub fn wrong_positions() -> Self {
        Error::WrongPositions
    }

    /// Create a new error for data format not set
    #[inline]
    pub fn data_format_not_set() -> Self {
        Error::DataFormatNotSet
    }

    /// Create a new error for navigation data not set
    #[inline]
    pub fn navigation_not_set() -> Self {
        Error::NavigationNotSet
    }

    /// Create a new error for no current set of ephemerides
    #[inline]
    pub fn no_current_ephemerides() -> Self {
        Error::NoCurrentEphemerides
    }
}

// We'll implement From for specific parsing errors as needed
// This is removed since we no longer depend on anyhow
