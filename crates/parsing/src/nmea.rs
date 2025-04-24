use std::{fs, path::PathBuf};

use constants::R2D;
use geometry::{Ecef, Location};

use crate::Error;

/// Parses a string into a floating-point number.
///
/// This is a helper function that converts a string representation of a number
/// into an f64 value, with appropriate error handling.
///
/// # Arguments
/// * `num_string` - String containing a floating-point number
///
/// # Returns
/// * `Ok(f64)` - Successfully parsed floating-point value
/// * `Err(Error)` - If the string cannot be parsed as a valid number
///
/// # Errors
/// * Returns a `ParseFloatError` wrapped in the crate's Error type if parsing
///   fails
#[inline]
pub fn parse_f64(num_string: &str) -> Result<f64, Error> {
    num_string.parse().map_err(Error::from)
}

/// Reads NMEA GGA sentences from a file and converts them to ECEF coordinates.
///
/// This function parses a file containing NMEA GGA sentences (Global
/// Positioning System Fix Data), extracts the position information, and
/// converts it to Earth-Centered, Earth-Fixed (ECEF) coordinates for use in the
/// simulation.
///
/// # NMEA GGA Format
/// Each GGA sentence has the following format:
/// ```
/// $GPGGA,time,lat,lat_dir,lon,lon_dir,quality,num_sats,hdop,alt,alt_units,undulation,und_units,age,station_id*checksum
/// ```
/// Where:
/// - `time` is UTC time in HHMMSS.SS format
/// - `lat` is latitude in DDMM.MMMM format (degrees + minutes)
/// - `lat_dir` is N (north) or S (south)
/// - `lon` is longitude in DDDMM.MMMM format (degrees + minutes)
/// - `lon_dir` is E (east) or W (west)
/// - `quality` is fix quality (0=invalid, 1=GPS fix, 2=DGPS fix)
/// - `num_sats` is number of satellites in use
/// - `hdop` is horizontal dilution of precision
/// - `alt` is altitude above mean sea level
/// - `alt_units` is units of altitude (usually 'M' for meters)
/// - `undulation` is height of geoid above WGS84 ellipsoid
/// - `und_units` is units of undulation (usually 'M' for meters)
/// - `age` is time since last DGPS update
/// - `station_id` is DGPS station ID
///
/// # Arguments
/// * `filename` - Path to the file containing NMEA GGA sentences
///
/// # Returns
/// * `Ok(Vec<Ecef>)` - Vector of ECEF coordinates converted from the NMEA data
/// * `Err(Error)` - If the file cannot be read or contains invalid data
///
/// # Errors
/// * Returns an error if the file cannot be opened
/// * Returns an error if the NMEA format is invalid
/// * Returns an error if latitude or longitude values cannot be parsed
/// * Returns an error if latitude or longitude are outside valid ranges
/// * Returns an error if the file contains no valid NMEA records
pub fn read_nmea_gga(filename: &PathBuf) -> Result<Vec<Ecef>, Error> {
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
        if record.len() < 15 {
            return Err(Error::invalid_nmea(format!(
                "Expected at least 15 fields, got {}",
                record.len()
            )));
        }

        // Extract fields
        let lat = record
            .get(2)
            .ok_or_else(|| Error::missing_field("latitude"))?;
        let lat_dir = record
            .get(3)
            .ok_or_else(|| Error::missing_field("latitude direction"))?;
        let lon = record
            .get(4)
            .ok_or_else(|| Error::missing_field("longitude"))?;
        let lon_dir = record
            .get(5)
            .ok_or_else(|| Error::missing_field("longitude direction"))?;
        let alt = record
            .get(9)
            .ok_or_else(|| Error::missing_field("altitude"))?;
        let undulation = record
            .get(11)
            .ok_or_else(|| Error::missing_field("undulation"))?;

        // Parse coordinates
        let mut llh = [0.0f64; 3];

        // Parse latitude: format is DDMM.MMMM (degrees + minutes)
        if lat.len() < 3 {
            return Err(Error::invalid_nmea(format!(
                "Invalid latitude format: {lat}"
            )));
        }
        llh[0] = parse_f64(&lat[..2])? + parse_f64(&lat[2..])? / 60.0;

        // Apply direction
        if lat_dir == "S" {
            llh[0] *= -1.0;
        }
        llh[0] /= R2D; // Convert to radians

        // Parse longitude: format is DDDMM.MMMM (degrees + minutes)
        if lon.len() < 4 {
            return Err(Error::invalid_nmea(format!(
                "Invalid longitude format: {lon}"
            )));
        }
        llh[1] = parse_f64(&lon[..3])? + parse_f64(&lon[3..])? / 60.0;

        // Apply direction
        if lon_dir == "W" {
            llh[1] *= -1.0;
        }
        llh[1] /= R2D; // Convert to radians

        // Parse altitude and undulation
        llh[2] = parse_f64(alt)? + parse_f64(undulation)?;

        // Validate coordinates
        if llh[0] < -90.0 || llh[0] > 90.0 || llh[1] < -180.0 || llh[1] > 180.0
        {
            return Err(Error::invalid_coordinates(llh[0], llh[1]));
        }

        // Convert to ECEF
        let pos = Ecef::from(&Location::from(&llh));
        xyz.push(pos);
    }

    if xyz.is_empty() {
        return Err(Error::invalid_nmea(
            "No valid NMEA GGA records found".to_string(),
        ));
    }

    Ok(xyz)
}
