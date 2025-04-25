use thiserror::Error;

/// Custom error type for the GPS crate
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// Error when performing I/O operations
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Error when processing navigation file data
    #[error("Invalid navigation file: {0}")]
    NavigationFile(String),

    /// Error when no ephemeris data is available for a satellite
    #[error("No ephemeris available")]
    NoEphemeris,

    /// Error when no current set of ephemerides is found for the simulation
    /// time
    #[error("No current set of ephemerides found")]
    NoCurrentEphemerides,

    /// Error when an invalid sampling frequency is specified
    #[error("Invalid sampling frequency")]
    InvalidSamplingFrequency,

    /// Error when an invalid I/Q data format is specified
    #[error("Invalid I/Q data format")]
    InvalidDataFormat,

    /// Error when attempting to set position(s) more than once
    #[error("Cannot set position(s) more than once")]
    DuplicatePositionSetting,

    /// Error when an invalid simulation duration is specified
    #[error("Invalid duration")]
    InvalidDuration,

    /// Error when an invalid start time is specified
    #[error("Invalid start time")]
    InvalidStartTime,

    /// Error when an invalid GPS day number is specified
    #[error("Invalid GPS day number")]
    InvalidGpsDay,

    /// Error when an invalid GPS week number is specified
    #[error("Invalid GPS week number")]
    InvalidGpsWeek,

    /// Error when an invalid delta leap second value is specified
    #[error("Invalid delta leap second")]
    InvalidDeltaLeapSecond,

    /// Error when incorrect position data is provided
    #[error("Wrong positions")]
    WrongPositions,

    /// Error when data format is not set before simulation
    #[error("Data format not set")]
    DataFormatNotSet,

    /// Error when navigation data is not set before simulation
    #[error("Navigation data not set")]
    NavigationNotSet,

    /// Error when `IQWriter` is not properly initialized
    #[error("IQWriter not initialized")]
    IQWriterNotInitialized,

    /// Error when signal generator is not properly initialized
    #[error("Signal generator not initialized")]
    NotInitialized,

    /// Error from the RINEX parsing module
    #[error("RINEX error: {0}")]
    Rinex(#[from] rinex::error::Error),

    /// Error when parsing time values
    #[error("Time parsing error: {0}")]
    TimeParseError(#[from] jiff::Error),

    /// Error when converting between UTF-8 and other encodings
    #[error("UTF-8 conversion error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),

    /// Error when parsing floating-point values
    #[error("Float parsing error: {0}")]
    ParseFloatError(#[from] std::num::ParseFloatError),

    /// Error when parsing integer values
    #[error("Integer parsing error: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),

    /// General parsing error with a message
    #[error("Parsing error: {0}")]
    ParsingError(String),

    /// Unknown or unspecified error
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
