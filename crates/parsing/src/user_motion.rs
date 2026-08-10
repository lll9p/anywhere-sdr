use std::{fs, path::PathBuf};

use csv::StringRecord;
use geometry::Ecef;

use crate::{Error, ecef_from_degrees, validate_ecef};

/// One user-motion position at a normalized elapsed time.
#[derive(Clone, Copy, Debug)]
pub struct UserMotionSample {
    /// Seconds elapsed since the first record in the motion file.
    pub elapsed_seconds: f64,
    /// Receiver position in Earth-Centered, Earth-Fixed coordinates.
    pub position_ecef: Ecef,
}

/// Reads user motion data from a CSV file in ECEF coordinate format.
///
/// Each record has the form `time,x,y,z`, where time is finite and records are
/// strictly increasing. The first timestamp is normalized to zero.
///
/// # Errors
/// Returns an error for I/O or CSV failures, malformed or non-monotonic
/// timestamps, invalid coordinates, or an empty file.
pub fn read_user_motion(
    filename: &PathBuf,
) -> Result<Vec<UserMotionSample>, Error> {
    read_motion_records(filename, "time,x,y,z", |record| {
        let x = parse_coordinate(record, 1, "x coordinate")?;
        let y = parse_coordinate(record, 2, "y coordinate")?;
        let z = parse_coordinate(record, 3, "z coordinate")?;
        validate_ecef(Ecef::from(&[x, y, z]))
    })
}

/// Reads user motion data from a CSV file in LLH coordinate format.
///
/// Each record has the form `time,latitude,longitude,height`. Time is finite
/// and strictly increasing, latitude and longitude are in degrees, and the
/// first timestamp is normalized to zero.
///
/// # Errors
/// Returns an error for I/O or CSV failures, malformed or non-monotonic
/// timestamps, invalid coordinates, or an empty file.
pub fn read_user_motion_llh(
    filename: &PathBuf,
) -> Result<Vec<UserMotionSample>, Error> {
    read_motion_records(filename, "time,lat,lon,height", |record| {
        let latitude = parse_coordinate(record, 1, "latitude")?;
        let longitude = parse_coordinate(record, 2, "longitude")?;
        let height = parse_coordinate(record, 3, "height")?;
        ecef_from_degrees(latitude, longitude, height)
    })
}

/// Reads and validates the shared timestamped CSV record structure.
fn read_motion_records<F>(
    filename: &PathBuf, fields: &str, mut parse_position: F,
) -> Result<Vec<UserMotionSample>, Error>
where
    F: FnMut(&StringRecord) -> Result<Ecef, Error>,
{
    let content = fs::read_to_string(filename)?;
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b',')
        .from_reader(content.as_bytes());
    let mut samples = Vec::new();
    let mut first_timestamp = None;
    let mut previous_timestamp = None;

    for result in reader.records() {
        let record = result?;
        let line = record.position().map_or(1, csv::Position::line);
        if record.len() < 4 {
            return Err(Error::invalid_user_motion(format!(
                "line {line}: expected at least 4 fields ({fields}), got {}",
                record.len()
            )));
        }

        let timestamp_text = record
            .get(0)
            .ok_or_else(|| Error::missing_field("timestamp"))?
            .trim();
        let timestamp = timestamp_text.parse::<f64>().map_err(|error| {
            Error::invalid_user_motion(format!(
                "line {line}: invalid timestamp `{timestamp_text}`: {error}"
            ))
        })?;
        if !timestamp.is_finite() {
            return Err(Error::invalid_user_motion(format!(
                "line {line}: timestamp must be finite, got {timestamp}"
            )));
        }
        if let Some(previous) = previous_timestamp
            && timestamp <= previous
        {
            return Err(Error::invalid_user_motion(format!(
                "line {line}: timestamp {timestamp} must be greater than the \
                 previous timestamp {previous}"
            )));
        }

        let origin = *first_timestamp.get_or_insert(timestamp);
        let elapsed_seconds = timestamp - origin;
        if !elapsed_seconds.is_finite() {
            return Err(Error::invalid_user_motion(format!(
                "line {line}: normalized elapsed timestamp is not finite"
            )));
        }
        let position_ecef = parse_position(&record)?;
        samples.push(UserMotionSample {
            elapsed_seconds,
            position_ecef,
        });
        previous_timestamp = Some(timestamp);
    }

    if samples.is_empty() {
        return Err(Error::invalid_user_motion(
            "No valid motion records found".to_string(),
        ));
    }
    Ok(samples)
}

/// Parses one required coordinate field while preserving numeric errors.
fn parse_coordinate(
    record: &StringRecord, index: usize, field: &str,
) -> Result<f64, Error> {
    record
        .get(index)
        .ok_or_else(|| Error::missing_field(field))?
        .trim()
        .parse()
        .map_err(Into::into)
}

#[cfg(test)]
#[path = "user_motion/tests.rs"]
mod tests;
