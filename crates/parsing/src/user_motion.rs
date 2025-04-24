use std::{fs, path::PathBuf};

use constants::R2D;
use geometry::{Ecef, Location};

use crate::Error;

/// Read the list of user motions from the input file in ECEF format
///
/// The file should be in CSV format with each line containing:
/// time, x, y, z
///
/// Returns a vector of ECEF coordinates
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

/// Read the list of user motions from the input file in LLH format
///
/// The file should be in CSV format with each line containing:
/// time, latitude, longitude, height
///
/// Returns a vector of ECEF coordinates converted from LLH
///
/// Added by romalvarezllorens@gmail.com
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
