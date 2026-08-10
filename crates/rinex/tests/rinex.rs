use pest::Parser;
use rinex::{
    Error, Rinex,
    rule::{RinexBuilder, RinexParser, Rule},
};

#[test]
fn parser_accepts_complete_fixture() -> Result<(), Error> {
    let data = complete_navigation_data('D');
    RinexParser::parse(Rule::rinex, &data).map_err(Box::new)?;
    Ok(())
}

#[test]
fn complete_header_preserves_exact_fields_and_source_order() -> Result<(), Error>
{
    let data =
        navigation_data(&complete_header('D'), &ephemeris_record(1, 'D'));
    let rinex = Rinex::read_string(&data)?;

    assert_eq!(rinex.version, "2.11");
    assert_eq!(rinex.type_, "NAVIGATION DATA");
    assert_eq!(rinex.program, "TEST PROGRAM");
    assert_eq!(rinex.agency, "TEST AGENCY");
    assert_eq!(rinex.update, "2026-07-13 00:00");
    assert_eq!(rinex.comments, ["first comment", "second comment"]);
    assert_eq!(rinex.ion_alpha, Some([1.0, -0.2, 0.03, -0.004]));
    assert_eq!(rinex.ion_beta, Some([10.0, 20.0, 30.0, 40.0]));
    let delta_utc = rinex
        .delta_utc
        .as_ref()
        .ok_or_else(|| Error::rule("missing DELTA-UTC test value"))?;
    assert!((delta_utc.a0 - -1.25e-9).abs() < f64::EPSILON);
    assert!((delta_utc.a1 - 2.5e-13).abs() < f64::EPSILON);
    assert_eq!(delta_utc.time, 4096);
    assert_eq!(delta_utc.week, 2317);
    assert_eq!(rinex.leap_seconds, Some(18));

    let ephemeris = first_ephemeris(&rinex)?;
    assert_eq!(ephemeris.prn, 1);
    assert_eq!(ephemeris.time_of_clock.year, 2024);
    assert!((ephemeris.sv_clock.bias - 2.665_151_841_94e-4).abs() < 1e-18);
    assert!((ephemeris.orbit1.crs - 77.656_25).abs() < f64::EPSILON);
    assert!((ephemeris.orbit2.ecc - 0.013_383_595_971_4).abs() < 1e-15);
    Ok(())
}

#[test]
fn minimal_header_accepts_no_optional_records() -> Result<(), Error> {
    let header = vec![version_line("2.11", "N: GPS NAV DATA"), program_line()];
    let rinex = Rinex::read_string(&navigation_data(
        &header,
        &ephemeris_record(1, 'D'),
    ))?;

    assert!(rinex.comments.is_empty());
    assert!(rinex.ion_alpha.is_none());
    assert!(rinex.ion_beta.is_none());
    assert!(rinex.delta_utc.is_none());
    assert!(rinex.leap_seconds.is_none());
    Ok(())
}

#[test]
fn every_optional_header_record_can_be_omitted_independently()
-> Result<(), Error> {
    let optional_records = [
        ("COMMENT", comment_line("only comment")),
        ("ION ALPHA", ion_alpha_line('D')),
        ("ION BETA", ion_beta_line('D')),
        ("DELTA-UTC", delta_utc_line('D')),
        ("LEAP SECONDS", leap_seconds_line()),
    ];

    for (omitted, _) in &optional_records {
        let mut header = vec![version_line("2.11", "N"), program_line()];
        header.extend(
            optional_records
                .iter()
                .filter(|(name, _)| name != omitted)
                .map(|(_, line)| line.clone()),
        );
        let rinex = Rinex::read_string(&navigation_data(
            &header,
            &ephemeris_record(1, 'D'),
        ))?;
        match *omitted {
            "COMMENT" => assert!(rinex.comments.is_empty()),
            "ION ALPHA" => assert!(rinex.ion_alpha.is_none()),
            "ION BETA" => assert!(rinex.ion_beta.is_none()),
            "DELTA-UTC" => assert!(rinex.delta_utc.is_none()),
            "LEAP SECONDS" => assert!(rinex.leap_seconds.is_none()),
            _ => return Err(Error::rule("unknown optional-record test case")),
        }
    }
    Ok(())
}

#[test]
fn optional_singleton_records_reject_duplicates() -> Result<(), Error> {
    for record in [
        ion_alpha_line('D'),
        ion_beta_line('D'),
        delta_utc_line('D'),
        leap_seconds_line(),
    ] {
        let header = vec![
            version_line("2.11", "N"),
            program_line(),
            record.clone(),
            record,
        ];
        assert_error_contains(
            Rinex::read_string(&navigation_data(
                &header,
                &ephemeris_record(1, 'D'),
            )),
            "duplicate",
        )?;
    }
    Ok(())
}

#[test]
fn core_header_cardinality_and_version_order_are_validated() -> Result<(), Error>
{
    for duplicate in [version_line("2.11", "N"), program_line()] {
        let header = vec![version_line("2.11", "N"), program_line(), duplicate];
        assert_error_contains(
            Rinex::read_string(&navigation_data(
                &header,
                &ephemeris_record(1, 'D'),
            )),
            "duplicate",
        )?;
    }

    let header = vec![program_line(), version_line("2.11", "N")];
    assert_error_contains(
        Rinex::read_string(&navigation_data(
            &header,
            &ephemeris_record(1, 'D'),
        )),
        "must be the first header record",
    )?;
    Ok(())
}

#[test]
fn repeated_comments_retain_order_when_interleaved() -> Result<(), Error> {
    let header = vec![
        version_line("2.11", "NAVIGATION DATA"),
        comment_line("one"),
        ion_alpha_line('D'),
        comment_line("two"),
        program_line(),
        comment_line("three"),
    ];
    let rinex = Rinex::read_string(&navigation_data(
        &header,
        &ephemeris_record(1, 'D'),
    ))?;
    assert_eq!(rinex.comments, ["one", "two", "three"]);
    Ok(())
}

#[test]
fn mandatory_core_header_records_are_required() -> Result<(), Error> {
    let cases = [
        (vec![program_line()], "version is missing"),
        (vec![version_line("2.11", "N")], "program is missing"),
    ];
    for (header, expected) in cases {
        assert_error_contains(
            Rinex::read_string(&navigation_data(
                &header,
                &ephemeris_record(1, 'D'),
            )),
            expected,
        )?;
    }

    let mut missing_end = String::new();
    missing_end.push_str(&version_line("2.11", "N"));
    missing_end.push_str(&program_line());
    missing_end.push_str(&ephemeris_record(1, 'D'));
    assert_error_contains(Rinex::read_string(&missing_end), "Cannot parse")?;
    Ok(())
}

#[test]
fn supported_rinex_2_versions_are_numeric() -> Result<(), Error> {
    for version in ["2", "2.00", "2.11", "2.99"] {
        let header = vec![version_line(version, "N"), program_line()];
        let rinex = Rinex::read_string(&navigation_data(
            &header,
            &ephemeris_record(1, 'D'),
        ))?;
        assert_eq!(rinex.version, version);
    }
    for version in ["1.99", "3.00"] {
        let header = vec![version_line(version, "N"), program_line()];
        assert_error_contains(
            Rinex::read_string(&navigation_data(
                &header,
                &ephemeris_record(1, 'D'),
            )),
            "unsupported RINEX version",
        )?;
    }
    Ok(())
}

#[test]
fn navigation_file_type_requires_a_supported_gps_form() -> Result<(), Error> {
    for file_type in [
        "N",
        "NAVIGATION DATA",
        "N: GPS NAV DATA",
        "N:   GPS NAV DATA",
    ] {
        let header = vec![version_line("2.11", file_type), program_line()];
        let rinex = Rinex::read_string(&navigation_data(
            &header,
            &ephemeris_record(1, 'D'),
        ))?;
        assert_eq!(rinex.type_, file_type);
    }
    for file_type in [
        "",
        "O: OBSERVATION DATA",
        "NONSENSE",
        "N: GLONASS NAV DATA",
        "N: GPS NAV DATA EXTRA",
        "n: GPS NAV DATA",
    ] {
        let header = vec![version_line("2.11", file_type), program_line()];
        assert_error_contains(
            Rinex::read_string(&navigation_data(
                &header,
                &ephemeris_record(1, 'D'),
            )),
            "unsupported RINEX file type",
        )?;
    }
    Ok(())
}

#[test]
fn at_least_one_ephemeris_record_is_required() -> Result<(), Error> {
    let data = navigation_data(&minimal_header(), "");
    assert_error_contains(Rinex::read_string(&data), "Cannot parse")?;

    let mut builder = RinexBuilder::new();
    builder.set_version("2.11".to_string());
    builder.set_type("N".to_string());
    builder.set_program("test".to_string());
    builder.set_agency("test".to_string());
    builder.set_update("test".to_string());
    builder.set_ephemerides(Vec::new());
    match builder.build() {
        Ok(_) => Err(Error::rule("empty ephemeris builder unexpectedly built")),
        Err(error) => {
            assert!(
                error.to_string().contains("ephemerides is empty"),
                "unexpected error: {error}"
            );
            Ok(())
        }
    }
}

#[test]
fn gps_prn_domain_is_validated_before_construction() -> Result<(), Error> {
    for prn in [1, 32] {
        let rinex = Rinex::read_string(&navigation_data(
            &minimal_header(),
            &ephemeris_record(prn, 'D'),
        ))?;
        assert_eq!(first_ephemeris(&rinex)?.prn, prn);
    }
    for prn in [0, 33] {
        assert_error_contains(
            Rinex::read_string(&navigation_data(
                &minimal_header(),
                &ephemeris_record(prn, 'D'),
            )),
            &format!("GPS PRN {prn}"),
        )?;
    }

    let oversized_prn = "184467440737095516160";
    let record = ephemeris_record(1, 'D').replacen(
        " 1 24",
        &format!("{oversized_prn} 24"),
        1,
    );
    assert_error_contains(
        Rinex::read_string(&navigation_data(&minimal_header(), &record)),
        oversized_prn,
    )?;
    Ok(())
}

#[test]
fn all_supported_exponent_markers_parse_in_headers_and_records()
-> Result<(), Error> {
    for exponent in ['D', 'd', 'E', 'e'] {
        let rinex = Rinex::read_string(&complete_navigation_data(exponent))?;
        assert_eq!(rinex.ion_alpha, Some([1.0, -0.2, 0.03, -0.004]));
        let ephemeris = first_ephemeris(&rinex)?;
        assert!((ephemeris.sv_clock.bias - 2.665_151_841_94e-4).abs() < 1e-18);
        assert!((ephemeris.orbit1.delta_n - 5.858_815_471_75e-9).abs() < 1e-21);
    }
    Ok(())
}

#[test]
fn truncated_header_and_record_return_errors() -> Result<(), Error> {
    let valid = complete_navigation_data('D');
    let missing_end = valid.replace(&header_line("", "END OF HEADER"), "");
    assert_error_contains(Rinex::read_string(&missing_end), "Cannot parse")?;

    let trimmed = valid.trim_end();
    let truncated = trimmed
        .rsplit_once('\n')
        .map(|(prefix, _)| prefix)
        .ok_or_else(|| Error::rule("fixture has no record line to truncate"))?;
    assert_error_contains(Rinex::read_string(truncated), "Cannot parse")?;
    Ok(())
}

#[test]
fn rinex_epoch_year_uses_rinex_2_pivot() -> Result<(), Error> {
    for (encoded_year, expected_year) in
        [("79", 2079), ("80", 1980), ("99", 1999), ("00", 2000)]
    {
        let record = ephemeris_record(1, 'D').replacen(
            " 1 24  6  1",
            &format!(" 1 {encoded_year}  6  1"),
            1,
        );
        let rinex =
            Rinex::read_string(&navigation_data(&minimal_header(), &record))?;
        assert_eq!(first_ephemeris(&rinex)?.time_of_clock.year, expected_year);
    }
    Ok(())
}

#[test]
fn rinex_epoch_year_outside_two_digit_domain_is_contextual_error()
-> Result<(), Error> {
    for encoded_year in ["100", "2147483647"] {
        let record = ephemeris_record(1, 'D').replacen(
            " 1 24  6  1",
            &format!(" 1 {encoded_year}  6  1"),
            1,
        );
        assert_error_contains(
            Rinex::read_string(&navigation_data(&minimal_header(), &record)),
            encoded_year,
        )?;
    }
    Ok(())
}

fn first_ephemeris(
    rinex: &Rinex,
) -> Result<&rinex::ephemeris::Ephemeris, Error> {
    rinex
        .ephemerides
        .first()
        .ok_or_else(|| Error::rule("missing test ephemeris"))
}

fn assert_error_contains(
    result: Result<Rinex, Error>, expected: &str,
) -> Result<(), Error> {
    match result {
        Ok(_) => Err(Error::rule(format!(
            "expected error containing {expected:?}"
        ))),
        Err(error) => {
            assert!(
                error.to_string().contains(expected),
                "expected {expected:?} in {error}"
            );
            Ok(())
        }
    }
}

fn complete_navigation_data(exponent: char) -> String {
    navigation_data(&complete_header(exponent), &ephemeris_record(1, exponent))
}

fn navigation_data(header: &[String], record: &str) -> String {
    let mut data = header.concat();
    data.push_str(&header_line("", "END OF HEADER"));
    data.push_str(record);
    data
}

fn minimal_header() -> Vec<String> {
    vec![version_line("2.11", "N"), program_line()]
}

fn complete_header(exponent: char) -> Vec<String> {
    vec![
        version_line("2.11", "NAVIGATION DATA"),
        ion_alpha_line(exponent),
        comment_line("first comment"),
        program_line(),
        ion_beta_line(exponent),
        comment_line("second comment"),
        delta_utc_line(exponent),
        leap_seconds_line(),
    ]
}

fn version_line(version: &str, file_type: &str) -> String {
    header_line(
        &format!("{version:>9}           {file_type}"),
        "RINEX VERSION / TYPE",
    )
}

fn program_line() -> String {
    header_line(
        &format!(
            "{:<20}{:<20}{:<20}",
            "TEST PROGRAM", "TEST AGENCY", "2026-07-13 00:00"
        ),
        "PGM / RUN BY / DATE",
    )
}

fn comment_line(comment: &str) -> String {
    header_line(comment, "COMMENT")
}

fn ion_alpha_line(exponent: char) -> String {
    header_line(
        &format!(
            "    0.1000{exponent}+01 -0.2000{exponent}+00  \
             0.3000{exponent}-01 -0.4000{exponent}-02"
        ),
        "ION ALPHA",
    )
}

fn ion_beta_line(exponent: char) -> String {
    header_line(
        &format!(
            "    0.1000{exponent}+02  0.2000{exponent}+02  \
             0.3000{exponent}+02  0.4000{exponent}+02"
        ),
        "ION BETA",
    )
}

fn delta_utc_line(exponent: char) -> String {
    header_line(
        &format!(
            "   -0.1250{exponent}-08 0.2500{exponent}-12     4096     2317"
        ),
        "DELTA-UTC: A0,A1,T,W",
    )
}

fn leap_seconds_line() -> String {
    header_line("    18", "LEAP SECONDS")
}

fn header_line(content: &str, label: &str) -> String {
    format!("{content:<60}{label}\n")
}

fn ephemeris_record(prn: usize, exponent: char) -> String {
    EPHEMERIS_RECORD
        .replacen(" 1 24  6  1", &format!("{prn:2} 24  6  1"), 1)
        .replace('D', &exponent.to_string())
}

const EPHEMERIS_RECORD: &str = r" 1 24  6  1  0  0  0.0 0.266515184194D-03-0.557065504836D-11 0.000000000000D+00
    0.390000000000D+02 0.776562500000D+02 0.585881547175D-08 0.301270077574D+01
    0.397302210331D-05 0.133835959714D-01 0.659003853798D-05 0.515376377487D+04
    0.518400000000D+06 0.109896063805D-06-0.590134072653D-01-0.540167093277D-07
    0.954779198592D+00 0.248281250000D+03 0.103214963985D+01-0.799497587998D-08
    0.231081054025D-09 0.100000000000D+01 0.231600000000D+04 0.000000000000D+00
    0.280000000000D+01 0.630000000000D+02-0.195577740669D-07 0.390000000000D+02
    0.511218000000D+06 0.400000000000D+01 0.000000000000D+00 0.000000000000D+00
";
