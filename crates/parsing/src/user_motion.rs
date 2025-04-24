use std::{fs, path::PathBuf};

use constants::R2D;
use geometry::{Ecef, Location};

use crate::Error;

/// Reads user motion data from a CSV file in ECEF coordinate format.
///
/// This function parses a CSV file containing user motion data in
/// Earth-Centered, Earth-Fixed (ECEF) coordinates. Each line in the file
/// represents a position at a specific time point.
///
/// # File Format
/// The file should be in CSV format with each line containing:
/// ```text
/// time, x, y, z
/// ```
/// Where:
/// - `time` is the time in seconds
/// - `x`, `y`, `z` are ECEF coordinates in meters
///
/// # Arguments
/// * `filename` - Path to the CSV file containing user motion data
///
/// # Returns
/// * `Ok(Vec<Ecef>)` - Vector of ECEF coordinates parsed from the file
/// * `Err(Error)` - If the file cannot be read or contains invalid data
///
/// # Errors
/// * Returns an error if the file cannot be opened
/// * Returns an error if the CSV format is invalid
/// * Returns an error if any coordinate values cannot be parsed
/// * Returns an error if the file contains no valid motion records
pub fn read_user_motion(filename: &PathBuf) -> Result<Vec<Ecef>, Error> {
    let mut xyz = Vec::new();
    let content = fs::read_to_string(filename)?;

    // Create a CSV reader with comma delimiter
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b',')
        .from_reader(content.as_bytes());

    for result in rdr.records() {
        let record = result?;

        // Ensure we have enough fields
        if record.len() < 4 {
            return Err(Error::invalid_user_motion(format!(
                "Expected at least 4 fields (time,x,y,z), got {}",
                record.len()
            )));
        }

        // Extract and parse ECEF coordinates
        let x = record
            .get(1)
            .ok_or_else(|| Error::missing_field("x coordinate"))?
            .trim()
            .parse()?;

        let y = record
            .get(2)
            .ok_or_else(|| Error::missing_field("y coordinate"))?
            .trim()
            .parse()?;

        let z = record
            .get(3)
            .ok_or_else(|| Error::missing_field("z coordinate"))?
            .trim()
            .parse()?;

        xyz.push(Ecef::from(&[x, y, z]));
    }

    if xyz.is_empty() {
        return Err(Error::invalid_user_motion(
            "No valid motion records found".to_string(),
        ));
    }

    Ok(xyz)
}

/// Reads user motion data from a CSV file in LLH coordinate format and converts
/// to ECEF.
///
/// This function parses a CSV file containing user motion data in Latitude,
/// Longitude, Height (LLH) coordinates and converts each position to
/// Earth-Centered, Earth-Fixed (ECEF) coordinates for use in the simulation.
///
/// # File Format
/// The file should be in CSV format with each line containing:
/// ```text
/// time, latitude, longitude, height
/// ```
/// Where:
/// - `time` is the time in seconds
/// - `latitude` is in degrees (-90 to 90)
/// - `longitude` is in degrees (-180 to 180)
/// - `height` is in meters above the WGS-84 ellipsoid
///
/// # Arguments
/// * `filename` - Path to the CSV file containing user motion data in LLH
///   format
///
/// # Returns
/// * `Ok(Vec<Ecef>)` - Vector of ECEF coordinates converted from the LLH data
/// * `Err(Error)` - If the file cannot be read or contains invalid data
///
/// # Errors
/// * Returns an error if the file cannot be opened
/// * Returns an error if the CSV format is invalid
/// * Returns an error if any coordinate values cannot be parsed
/// * Returns an error if latitude or longitude are outside valid ranges
/// * Returns an error if the file contains no valid motion records
///
/// # Credit
/// Originally added by romalvarezllorens@gmail.com
pub fn read_user_motion_llh(filename: &PathBuf) -> Result<Vec<Ecef>, Error> {
    let mut xyz = Vec::new();
    let content = fs::read_to_string(filename)?;

    // Create a CSV reader with comma delimiter
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b',')
        .from_reader(content.as_bytes());

    for result in rdr.records() {
        let record = result?;

        // Ensure we have enough fields
        if record.len() < 4 {
            return Err(Error::invalid_user_motion(format!(
                "Expected at least 4 fields (time,lat,lon,height), got {}",
                record.len()
            )));
        }

        // Extract and parse LLH coordinates
        let lat = record
            .get(1)
            .ok_or_else(|| Error::missing_field("latitude"))?
            .trim()
            .parse()?;

        let lon = record
            .get(2)
            .ok_or_else(|| Error::missing_field("longitude"))?
            .trim()
            .parse()?;

        let height = record
            .get(3)
            .ok_or_else(|| Error::missing_field("height"))?
            .trim()
            .parse()?;

        let mut llh = Location::from(&[lat, lon, height]);

        // Validate coordinates
        if llh.latitude > 90.0
            || llh.latitude < -90.0
            || llh.longitude > 180.0
            || llh.longitude < -180.0
        {
            return Err(Error::invalid_coordinates(
                llh.latitude,
                llh.longitude,
            ));
        }

        // Convert to radians
        llh.latitude /= R2D;
        llh.longitude /= R2D;

        // Convert to ECEF
        xyz.push(Ecef::from(&llh));
    }

    if xyz.is_empty() {
        return Err(Error::invalid_user_motion(
            "No valid motion records found".to_string(),
        ));
    }

    Ok(xyz)
}
