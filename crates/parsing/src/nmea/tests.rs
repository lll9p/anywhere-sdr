use std::path::PathBuf;

use geometry::Ecef;

use super::{parse_gga_line, parse_nmea_gga, read_nmea_gga, xor_checksum};
use crate::{Error, ecef_from_degrees};

const BASE_PAYLOAD: &[u8] = b"GPGGA,000000.00,4852.46626694,N,00217.58140440,E,1,05,2.87,+0.00,M,-21.3213,M,,";
const FIRST_PAYLOAD: &[u8] = b"GPGGA,000000.00,4852.46626694,N,00217.58140440,E,1,05,2.87,+0.00,M,-21.3213,M,,";
const FIRST_LINE: &[u8] = b"$GPGGA,000000.00,4852.46626694,N,00217.58140440,E,1,05,2.87,+0.00,M,-21.3213,M,,*5E";

const FORMATTER: usize = 0;
const UTC: usize = 1;
const LATITUDE: usize = 2;
const LATITUDE_DIRECTION: usize = 3;
const LONGITUDE: usize = 4;
const LONGITUDE_DIRECTION: usize = 5;
const QUALITY: usize = 6;
const SATELLITE_COUNT: usize = 7;
const HDOP: usize = 8;
const ALTITUDE: usize = 9;
const ALTITUDE_UNIT: usize = 10;
const UNDULATION: usize = 11;
const UNDULATION_UNIT: usize = 12;
const DIFFERENTIAL_AGE: usize = 13;
const STATION_ID: usize = 14;

type TestResult = Result<(), Error>;

fn test_checksum(payload: &[u8]) -> u8 {
    payload.iter().fold(0, |checksum, byte| checksum ^ byte)
}

fn uppercase_hex_digit(value: u8) -> u8 {
    match value {
        0..=9 => b'0' + value,
        _ => b'A' + value - 10,
    }
}

fn sentence_from_fields(fields: &[&[u8]]) -> Vec<u8> {
    let mut payload = Vec::new();
    for (field_index, field) in fields.iter().enumerate() {
        if field_index != 0 {
            payload.push(b',');
        }
        payload.extend_from_slice(field);
    }

    let checksum = test_checksum(&payload);
    let mut sentence = Vec::with_capacity(payload.len() + 4);
    sentence.push(b'$');
    sentence.extend_from_slice(&payload);
    sentence.push(b'*');
    sentence.push(uppercase_hex_digit(checksum >> 4));
    sentence.push(uppercase_hex_digit(checksum & 0x0f));
    sentence
}

fn sentence_with_fields(
    replacements: &[(usize, &[u8])],
) -> Result<Vec<u8>, Error> {
    let mut fields =
        BASE_PAYLOAD.split(|byte| *byte == b',').collect::<Vec<_>>();
    for (field_index, value) in replacements {
        let field = fields.get_mut(*field_index).ok_or_else(|| {
            Error::invalid_nmea("invalid test field replacement")
        })?;
        *field = value;
    }
    Ok(sentence_from_fields(&fields))
}

fn sentence_with_field(
    field_index: usize, value: &[u8],
) -> Result<Vec<u8>, Error> {
    sentence_with_fields(&[(field_index, value)])
}

fn parse_one(sentence: &[u8]) -> Result<Ecef, Error> {
    let positions = parse_nmea_gga(sentence)?;
    positions
        .first()
        .copied()
        .ok_or_else(|| Error::invalid_nmea("missing test position"))
}

fn invalid_message(content: &[u8]) -> Option<String> {
    match parse_nmea_gga(content) {
        Err(Error::InvalidNmeaFormat(message)) => Some(message),
        _ => None,
    }
}

fn assert_invalid_contains(content: &[u8], expected: &str) {
    let message = invalid_message(content);
    assert!(
        message
            .as_deref()
            .is_some_and(|message| message.starts_with("line ")),
        "expected physical line context, got {message:?}"
    );
    assert!(
        message
            .as_deref()
            .is_some_and(|message| message.contains(expected)),
        "expected NMEA error containing {expected:?}, got {message:?}"
    );
}

fn assert_ecef_equal(left: &Ecef, right: &Ecef) {
    assert_eq!(left.x.to_bits(), right.x.to_bits());
    assert_eq!(left.y.to_bits(), right.y.to_bits());
    assert_eq!(left.z.to_bits(), right.z.to_bits());
}

fn assert_positions_equal(left: &[Ecef], right: &[Ecef]) {
    assert_eq!(left.len(), right.len());
    for (left, right) in left.iter().zip(right) {
        assert_ecef_equal(left, right);
    }
}

#[test]
fn known_literal_checksum_is_5e() -> TestResult {
    assert_eq!(xor_checksum(FIRST_PAYLOAD), 0x5e);
    parse_gga_line(FIRST_LINE, 1)?;
    Ok(())
}

#[test]
fn accepts_gp_gn_line_endings_and_exact_blank_lines() -> TestResult {
    let gp = sentence_with_fields(&[])?;
    let gn = sentence_with_field(FORMATTER, b"GNGGA")?;
    let mut content = b"\n\r\n".to_vec();
    content.extend_from_slice(&gp);
    content.extend_from_slice(b"\r\n\n");
    content.extend_from_slice(&gn);

    let positions = parse_nmea_gga(&content)?;
    assert_eq!(positions.len(), 2);
    let mut positions = positions.iter();
    let first = positions
        .next()
        .ok_or_else(|| Error::invalid_nmea("missing first test position"))?;
    let second = positions
        .next()
        .ok_or_else(|| Error::invalid_nmea("missing second test position"))?;
    assert_ecef_equal(first, second);
    Ok(())
}

#[test]
fn accepts_every_supported_quality() -> TestResult {
    let qualities: &[&[u8]] = &[b"1", b"2", b"3", b"4", b"5", b"6", b"7", b"8"];
    for quality in qualities {
        parse_one(&sentence_with_field(QUALITY, quality)?)?;
    }
    Ok(())
}

#[test]
fn accepts_coordinate_forms_hemispheres_endpoints_and_signed_heights()
-> TestResult {
    let decimal = sentence_with_fields(&[
        (LATITUDE_DIRECTION, b"S"),
        (LONGITUDE_DIRECTION, b"W"),
        (ALTITUDE, b"+10.5"),
        (UNDULATION, b"-20.5"),
    ])?;
    let actual = parse_one(&decimal)?;
    let expected = ecef_from_degrees(
        -(48.0 + 52.466_266_94 / 60.0),
        -(2.0 + 17.581_404_40 / 60.0),
        -10.0,
    )?;
    assert_ecef_equal(&actual, &expected);

    parse_one(&sentence_with_fields(&[
        (LATITUDE, b"4852"),
        (LONGITUDE, b"00217"),
    ])?)?;
    parse_one(&sentence_with_fields(&[
        (LATITUDE, b"9000.000"),
        (LONGITUDE, b"18000.000"),
    ])?)?;
    parse_one(&sentence_with_fields(&[
        (LATITUDE, b"0000.0"),
        (LONGITUDE, b"00000.0"),
        (LATITUDE_DIRECTION, b"S"),
        (LONGITUDE_DIRECTION, b"W"),
    ])?)?;
    Ok(())
}

#[test]
fn rejects_non_ascii_controls_and_whitespace_with_physical_lines() {
    assert_invalid_contains(b"\n\xc3\xa9", "line 2: record contains");
    assert_invalid_contains(b"\n\xff", "line 2: record contains");
    assert_invalid_contains(b"\n\t", "line 2: record contains");
    assert_invalid_contains(b"\n   \n", "line 2: record must contain");
}

#[test]
fn enforces_framing_and_checksum_priority() -> TestResult {
    let cases: &[(&[u8], &str)] = &[
        (b"\xffGPGGA", "record contains non-printable ASCII"),
        (b"GPGGA*00", "exactly one leading '$'"),
        (b"$GP$GGA*00", "exactly one leading '$'"),
        (b"$GPGGA", "exactly one '*'"),
        (b"$GPGGA*00*00", "exactly one '*'"),
        (b"$GPGGA*", "exactly two uppercase"),
        (b"$GPGGA*0", "exactly two uppercase"),
        (b"$GPGGA*000", "exactly two uppercase"),
        (b"$GPGGA*5e", "exactly two uppercase"),
        (b"$GPGGA*GG", "exactly two uppercase"),
        (b"$GPGGA*00", "checksum mismatch"),
    ];
    for (content, expected) in cases {
        assert_invalid_contains(content, expected);
    }

    let mut unsupported = sentence_with_field(FORMATTER, b"GLGGA")?;
    if let Some(last) = unsupported.last_mut() {
        *last = if *last == b'0' { b'1' } else { b'0' };
    }
    assert_invalid_contains(&unsupported, "checksum mismatch");

    let short_fields = BASE_PAYLOAD
        .split(|byte| *byte == b',')
        .take(14)
        .collect::<Vec<_>>();
    let mut short = sentence_from_fields(&short_fields);
    if let Some(last) = short.last_mut() {
        *last = if *last == b'0' { b'1' } else { b'0' };
    }
    assert_invalid_contains(&short, "checksum mismatch");
    assert_invalid_contains(
        &sentence_with_field(FORMATTER, b"GLGGA")?,
        "unsupported sentence formatter",
    );
    Ok(())
}

#[test]
fn requires_exact_field_count_and_supported_formatter() {
    let fields = BASE_PAYLOAD.split(|byte| *byte == b',').collect::<Vec<_>>();
    let fourteen = sentence_from_fields(
        &fields.iter().copied().take(14).collect::<Vec<_>>(),
    );
    let mut unsupported_fourteen_fields =
        fields.iter().copied().take(14).collect::<Vec<_>>();
    if let Some(formatter) = unsupported_fourteen_fields.first_mut() {
        *formatter = b"GLGGA";
    }
    let unsupported_fourteen =
        sentence_from_fields(&unsupported_fourteen_fields);
    let mut sixteen_fields = fields;
    sixteen_fields.push(b"extra");
    let sixteen = sentence_from_fields(&sixteen_fields);
    assert_invalid_contains(&fourteen, "exactly 15 fields, got 14");
    assert_invalid_contains(&unsupported_fourteen, "exactly 15 fields, got 14");
    assert_invalid_contains(&sixteen, "exactly 15 fields, got 16");

    for formatter in [b"GPRMC".as_slice(), b"GLGGA", b"GGA"] {
        let sentence = sentence_with_field(FORMATTER, formatter);
        assert!(sentence.is_ok());
        if let Ok(sentence) = sentence {
            assert_invalid_contains(&sentence, "unsupported sentence");
        }
    }
}

#[test]
fn validates_quality_directions_and_units() -> TestResult {
    let cases: &[(usize, &[u8], &str)] = &[
        (QUALITY, b"0", "fix quality"),
        (QUALITY, b"9", "fix quality"),
        (QUALITY, b"", "fix quality"),
        (QUALITY, b"01", "fix quality"),
        (QUALITY, b"X", "fix quality"),
        (LATITUDE_DIRECTION, b"", "latitude direction"),
        (LATITUDE_DIRECTION, b"n", "latitude direction"),
        (LATITUDE_DIRECTION, b"E", "latitude direction"),
        (LONGITUDE_DIRECTION, b"", "longitude direction"),
        (LONGITUDE_DIRECTION, b"w", "longitude direction"),
        (LONGITUDE_DIRECTION, b"N", "longitude direction"),
        (ALTITUDE_UNIT, b"", "altitude unit"),
        (ALTITUDE_UNIT, b"m", "altitude unit"),
        (UNDULATION_UNIT, b"", "geoid undulation unit"),
        (UNDULATION_UNIT, b"FT", "geoid undulation unit"),
    ];
    for (field_index, value, expected) in cases {
        assert_invalid_contains(
            &sentence_with_field(*field_index, value)?,
            expected,
        );
    }
    Ok(())
}

#[test]
fn validates_latitude_grammar_minutes_degrees_and_endpoint() -> TestResult {
    let invalid: &[&[u8]] = &[
        b"",
        b"123",
        b"12345",
        b"+4852.1",
        b"-4852.1",
        b"485e1",
        b"852.1",
        b"4852.",
        b"4852..1",
        b"48A2.1",
        b"4860",
        b"9100",
        b"9000.0001",
    ];
    for value in invalid {
        assert_invalid_contains(
            &sentence_with_field(LATITUDE, value)?,
            "latitude",
        );
    }
    Ok(())
}

#[test]
fn validates_longitude_grammar_minutes_degrees_and_endpoint() -> TestResult {
    let invalid: &[&[u8]] = &[
        b"",
        b"0123",
        b"001234",
        b"+00217.1",
        b"-00217.1",
        b"002e1",
        b"0217.1",
        b"00217.",
        b"00217..1",
        b"00A17.1",
        b"00260",
        b"18100",
        b"18000.0001",
    ];
    for value in invalid {
        assert_invalid_contains(
            &sentence_with_field(LONGITUDE, value)?,
            "longitude",
        );
    }
    Ok(())
}

#[test]
fn validates_altitude_undulation_and_finite_sum() -> TestResult {
    for (field_index, value, expected) in [
        (ALTITUDE, b"".as_slice(), "invalid altitude format"),
        (ALTITUDE, b"NaN", "altitude must be finite"),
        (ALTITUDE, b"inf", "altitude must be finite"),
        (UNDULATION, b"-inf", "geoid undulation must be finite"),
    ] {
        assert_invalid_contains(
            &sentence_with_field(field_index, value)?,
            expected,
        );
    }
    assert_invalid_contains(
        &sentence_with_fields(&[(ALTITUDE, b"1e308"), (UNDULATION, b"1e308")])?,
        "plus geoid undulation must be finite",
    );
    Ok(())
}

#[test]
fn ignored_fields_preserve_ecef_values_and_record_order() -> TestResult {
    let first = sentence_with_fields(&[])?;
    let second = sentence_with_fields(&[
        (LATITUDE, b"4900.0000"),
        (LONGITUDE, b"00300.0000"),
    ])?;
    let mut baseline_content = first.clone();
    baseline_content.push(b'\n');
    baseline_content.extend_from_slice(&second);
    let baseline = parse_nmea_gga(&baseline_content)?;

    let ignored_fields: &[(usize, &[u8])] = &[
        (UTC, b"ignored time"),
        (SATELLITE_COUNT, b"many"),
        (HDOP, b"not-a-number"),
        (DIFFERENTIAL_AGE, b"unknown"),
        (STATION_ID, b"station text"),
    ];
    for (field_index, value) in ignored_fields {
        let mut mutated_content = sentence_with_field(*field_index, value)?;
        mutated_content.push(b'\n');
        mutated_content.extend_from_slice(&second);
        let mutated = parse_nmea_gga(&mutated_content)?;
        assert_positions_equal(&baseline, &mutated);
    }
    Ok(())
}

#[test]
fn invalid_record_fails_whole_file_with_line_context() -> TestResult {
    let valid = sentence_with_fields(&[])?;
    let invalid = sentence_with_field(QUALITY, b"0")?;
    let mut content = valid.clone();
    content.push(b'\n');
    content.extend_from_slice(&invalid);
    content.push(b'\n');
    content.extend_from_slice(&valid);
    assert_invalid_contains(&content, "line 2: invalid fix quality");
    Ok(())
}

#[test]
fn empty_and_exactly_blank_inputs_have_only_lineless_no_record_error() {
    for content in [b"".as_slice(), b"\n\n", b"\r\n\r\n"] {
        assert_eq!(
            invalid_message(content).as_deref(),
            Some("No valid NMEA GGA records found")
        );
    }
}

#[test]
fn bundled_triumph_file_has_1561_positions() -> TestResult {
    let path = PathBuf::from(env!("CARGO_WORKSPACE_DIR"))
        .join("resources/triumphv3.txt");
    let positions = read_nmea_gga(&path)?;
    assert_eq!(positions.len(), 1_561);
    Ok(())
}
