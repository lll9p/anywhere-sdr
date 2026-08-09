use std::{fs, path::PathBuf, str};

use geometry::Ecef;

use crate::{Error, ecef_from_degrees};

/// Reads NMEA GGA sentences from a file and converts them to ECEF coordinates.
///
/// Empty physical lines are ignored. Every nonempty line must be a printable
/// ASCII `GPGGA` or `GNGGA` sentence with a valid checksum and fix.
///
/// # Errors
///
/// Returns an I/O error when the file cannot be read, an NMEA format error with
/// physical line context when a record is invalid, or a geometry error when
/// coordinate conversion fails.
pub fn read_nmea_gga(filename: &PathBuf) -> Result<Vec<Ecef>, Error> {
    parse_nmea_gga(&fs::read(filename)?)
}

/// Parses a complete byte stream containing physical NMEA lines.
fn parse_nmea_gga(content: &[u8]) -> Result<Vec<Ecef>, Error> {
    let mut positions = Vec::new();

    for (line_index, physical_line) in
        content.split(|byte| *byte == b'\n').enumerate()
    {
        let line = match physical_line.strip_suffix(b"\r") {
            Some(line) => line,
            None => physical_line,
        };
        if line.is_empty() {
            continue;
        }

        positions.push(parse_gga_line(line, line_index + 1)?);
    }

    if positions.is_empty() {
        return Err(Error::invalid_nmea("No valid NMEA GGA records found"));
    }

    Ok(positions)
}

/// Parses one validated physical line into an ECEF position.
fn parse_gga_line(line: &[u8], line_number: usize) -> Result<Ecef, Error> {
    let payload = validate_framing(line, line_number)?;
    let fields = payload.split(|byte| *byte == b',').collect::<Vec<_>>();
    let fields: [&[u8]; 15] =
        fields.try_into().map_err(|fields: Vec<&[u8]>| {
            invalid_line(
                line_number,
                format!("expected exactly 15 fields, got {}", fields.len()),
            )
        })?;
    let [
        formatter,
        _utc,
        latitude,
        latitude_direction,
        longitude,
        longitude_direction,
        quality,
        _satellite_count,
        _hdop,
        altitude,
        altitude_unit,
        undulation,
        undulation_unit,
        _differential_age,
        _station_id,
    ] = fields;

    if formatter != b"GPGGA" && formatter != b"GNGGA" {
        return Err(invalid_line(
            line_number,
            "unsupported sentence formatter",
        ));
    }
    if !matches!(quality, [b'1'..=b'8']) {
        return Err(invalid_line(line_number, "invalid fix quality"));
    }

    let latitude_sign = match latitude_direction {
        b"N" => 1.0,
        b"S" => -1.0,
        _ => {
            return Err(invalid_line(
                line_number,
                "invalid latitude direction",
            ));
        }
    };
    let longitude_sign = match longitude_direction {
        b"E" => 1.0,
        b"W" => -1.0,
        _ => {
            return Err(invalid_line(
                line_number,
                "invalid longitude direction",
            ));
        }
    };
    if altitude_unit != b"M" {
        return Err(invalid_line(line_number, "invalid altitude unit"));
    }
    if undulation_unit != b"M" {
        return Err(invalid_line(line_number, "invalid geoid undulation unit"));
    }

    let latitude = parse_coordinate(latitude, 2, 90, "latitude", line_number)?
        * latitude_sign;
    let longitude =
        parse_coordinate(longitude, 3, 180, "longitude", line_number)?
            * longitude_sign;
    let altitude = parse_finite_number(altitude, "altitude", line_number)?;
    let undulation =
        parse_finite_number(undulation, "geoid undulation", line_number)?;
    let height = altitude + undulation;
    if !height.is_finite() {
        return Err(invalid_line(
            line_number,
            "altitude plus geoid undulation must be finite",
        ));
    }

    ecef_from_degrees(latitude, longitude, height)
}

/// Validates ASCII sentence framing and returns the checksummed payload.
fn validate_framing(line: &[u8], line_number: usize) -> Result<&[u8], Error> {
    if !line.iter().all(|byte| matches!(byte, b' '..=b'~')) {
        return Err(invalid_line(
            line_number,
            "record contains non-printable ASCII",
        ));
    }

    if line.first() != Some(&b'$')
        || line.iter().skip(1).any(|byte| *byte == b'$')
    {
        return Err(invalid_line(
            line_number,
            "record must contain exactly one leading '$'",
        ));
    }

    let mut stars = line.iter().enumerate().filter(|(_, byte)| **byte == b'*');
    let star_index = match stars.next() {
        Some((index, _)) if stars.next().is_none() => index,
        _ => {
            return Err(invalid_line(
                line_number,
                "record must contain exactly one '*'",
            ));
        }
    };

    let checksum_start = star_index
        .checked_add(1)
        .ok_or_else(|| invalid_line(line_number, "invalid checksum framing"))?;
    let checksum_bytes = line
        .get(checksum_start..)
        .ok_or_else(|| invalid_line(line_number, "invalid checksum framing"))?;
    let [checksum_high, checksum_low] = checksum_bytes else {
        return Err(invalid_line(
            line_number,
            "checksum must be exactly two uppercase hexadecimal digits",
        ));
    };
    let checksum_high =
        uppercase_hex_value(*checksum_high).ok_or_else(|| {
            invalid_line(
                line_number,
                "checksum must be exactly two uppercase hexadecimal digits",
            )
        })?;
    let checksum_low = uppercase_hex_value(*checksum_low).ok_or_else(|| {
        invalid_line(
            line_number,
            "checksum must be exactly two uppercase hexadecimal digits",
        )
    })?;
    let expected_checksum = (checksum_high << 4) | checksum_low;

    let payload = line
        .get(1..star_index)
        .ok_or_else(|| invalid_line(line_number, "invalid sentence framing"))?;
    if xor_checksum(payload) != expected_checksum {
        return Err(invalid_line(line_number, "checksum mismatch"));
    }

    Ok(payload)
}

/// Parses one unsigned `DDMM` or `DDDMM` coordinate field.
fn parse_coordinate(
    field: &[u8], degree_width: usize, maximum_degrees: u16, label: &str,
    line_number: usize,
) -> Result<f64, Error> {
    let mut components = field.split(|byte| *byte == b'.');
    let Some(integer) = components.next() else {
        return Err(invalid_line(line_number, invalid_format(label)));
    };
    let fraction = components.next();
    if components.next().is_some()
        || fraction.is_some_and(<[u8]>::is_empty)
        || integer.len() != degree_width + 2
        || !integer.iter().all(u8::is_ascii_digit)
        || fraction
            .is_some_and(|fraction| !fraction.iter().all(u8::is_ascii_digit))
    {
        return Err(invalid_line(line_number, invalid_format(label)));
    }

    let degree_bytes = integer
        .get(..degree_width)
        .ok_or_else(|| invalid_line(line_number, invalid_format(label)))?;
    let minute_bytes = field
        .get(degree_width..)
        .ok_or_else(|| invalid_line(line_number, invalid_format(label)))?;
    let degrees = parse_unsigned_integer(degree_bytes, label, line_number)?;
    let minutes = parse_number(minute_bytes, label, line_number)?;

    if minutes >= 60.0 {
        return Err(invalid_line(
            line_number,
            format!("{label} minutes must be less than 60"),
        ));
    }
    if degrees > maximum_degrees {
        return Err(invalid_line(
            line_number,
            format!("{label} degrees exceed the valid range"),
        ));
    }
    if degrees == maximum_degrees && minutes != 0.0 {
        return Err(invalid_line(
            line_number,
            format!("{label} endpoint requires zero minutes"),
        ));
    }

    Ok(f64::from(degrees) + minutes / 60.0)
}

/// Parses a finite floating-point field with line and field context.
fn parse_finite_number(
    field: &[u8], label: &str, line_number: usize,
) -> Result<f64, Error> {
    let value = parse_number(field, label, line_number)?;
    if !value.is_finite() {
        return Err(invalid_line(
            line_number,
            format!("{label} must be finite"),
        ));
    }
    Ok(value)
}

/// Parses a floating-point field without leaking a context-free parse error.
fn parse_number(
    field: &[u8], label: &str, line_number: usize,
) -> Result<f64, Error> {
    str::from_utf8(field)
        .map_err(|_| invalid_line(line_number, invalid_format(label)))?
        .parse::<f64>()
        .map_err(|_| invalid_line(line_number, invalid_format(label)))
}

/// Parses an unsigned integer field with line and field context.
fn parse_unsigned_integer(
    field: &[u8], label: &str, line_number: usize,
) -> Result<u16, Error> {
    str::from_utf8(field)
        .map_err(|_| invalid_line(line_number, invalid_format(label)))?
        .parse::<u16>()
        .map_err(|_| invalid_line(line_number, invalid_format(label)))
}

/// Formats the shared lexical error for one semantic field.
fn invalid_format(label: &str) -> String {
    format!("invalid {label} format")
}

/// Converts one uppercase hexadecimal byte to its nibble value.
fn uppercase_hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Computes the NMEA XOR checksum over a sentence payload.
fn xor_checksum(payload: &[u8]) -> u8 {
    payload.iter().fold(0, |checksum, byte| checksum ^ byte)
}

/// Creates an NMEA format error with physical line context.
fn invalid_line(line_number: usize, message: impl Into<String>) -> Error {
    Error::invalid_nmea(format!("line {line_number}: {}", message.into()))
}

#[cfg(test)]
#[path = "nmea/tests.rs"]
mod tests;
