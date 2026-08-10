use pest::iterators::Pairs;

use super::{Rule, next_str, to_float, to_int};
use crate::{ephemeris::Ephemeris, error::Error, utc::DeltaUtc};

/// Parsed RINEX 2 GPS navigation data.
#[derive(Debug)]
pub struct Rinex {
    /// Format version as written in the header.
    pub version: String,
    /// Navigation file type as written in the header.
    pub type_: String,
    /// Program name.
    pub program: String,
    /// Agency name.
    pub agency: String,
    /// Header update date.
    pub update: String,
    /// Comments in source order.
    pub comments: Vec<String>,
    /// Ionospheric alpha correction parameters when present.
    pub ion_alpha: Option<[f64; 4]>,
    /// Ionospheric beta correction parameters when present.
    pub ion_beta: Option<[f64; 4]>,
    /// GPS-to-UTC correction parameters when present.
    pub delta_utc: Option<DeltaUtc>,
    /// GPS-to-UTC leap-second offset when present.
    pub leap_seconds: Option<i32>,
    /// GPS satellite ephemeris records.
    pub ephemerides: Vec<Ephemeris>,
}

/// Incremental builder used by the RINEX parser.
#[derive(Debug, Default)]
pub struct RinexBuilder {
    /// Format version.
    version: Option<String>,
    /// Navigation file type.
    type_: Option<String>,
    /// Producing program.
    program: Option<String>,
    /// Producing agency.
    agency: Option<String>,
    /// Header update date.
    update: Option<String>,
    /// Comments in source order.
    comments: Vec<String>,
    /// Optional ionospheric alpha parameters.
    ion_alpha: Option<[f64; 4]>,
    /// Optional ionospheric beta parameters.
    ion_beta: Option<[f64; 4]>,
    /// Optional UTC correction parameters.
    delta_utc: Option<DeltaUtc>,
    /// Optional leap-second offset.
    leap_seconds: Option<i32>,
    /// Parsed ephemeris records.
    ephemerides: Option<Vec<Ephemeris>>,
}

impl RinexBuilder {
    /// Creates an empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the RINEX version.
    pub fn set_version(&mut self, version: String) {
        self.version = Some(version);
    }

    /// Sets the RINEX file type.
    pub fn set_type(&mut self, type_: String) {
        self.type_ = Some(type_);
    }

    /// Sets the producing program.
    pub fn set_program(&mut self, program: String) {
        self.program = Some(program);
    }

    /// Sets the producing agency.
    pub fn set_agency(&mut self, agency: String) {
        self.agency = Some(agency);
    }

    /// Sets the header update date.
    pub fn set_update(&mut self, update: String) {
        self.update = Some(update);
    }

    /// Replaces all comments with one comment.
    pub fn set_comments(&mut self, comments: String) {
        self.comments.clear();
        self.comments.push(comments);
    }

    /// Appends one comment while preserving source order.
    pub fn add_comment(&mut self, comment: String) {
        self.comments.push(comment);
    }

    /// Sets the ionospheric alpha parameters.
    pub fn set_ion_alpha(&mut self, ion_alpha: [f64; 4]) {
        self.ion_alpha = Some(ion_alpha);
    }

    /// Sets the ionospheric beta parameters.
    pub fn set_ion_beta(&mut self, ion_beta: [f64; 4]) {
        self.ion_beta = Some(ion_beta);
    }

    /// Sets the UTC correction parameters.
    pub fn set_delta_utc(&mut self, delta_utc: DeltaUtc) {
        self.delta_utc = Some(delta_utc);
    }

    /// Sets the leap-second offset.
    pub fn set_leap_seconds(&mut self, leap_seconds: i32) {
        self.leap_seconds = Some(leap_seconds);
    }

    /// Sets the parsed ephemeris records.
    pub fn set_ephemerides(&mut self, ephemerides: Vec<Ephemeris>) {
        self.ephemerides = Some(ephemerides);
    }

    /// Builds a validated RINEX 2 GPS navigation model.
    ///
    /// # Errors
    /// Returns an error when a mandatory header/record section is absent or
    /// the version/type metadata is unsupported.
    pub fn build(&mut self) -> Result<Rinex, Error> {
        let version = required_ref(self.version.as_ref(), "version")?;
        let type_ = required_ref(self.type_.as_ref(), "type")?;
        validate_metadata(version, type_)?;
        required_ref(self.program.as_ref(), "program")?;
        required_ref(self.agency.as_ref(), "agency")?;
        required_ref(self.update.as_ref(), "update")?;
        let ephemerides =
            required_ref(self.ephemerides.as_ref(), "ephemerides")?;
        if ephemerides.is_empty() {
            return Err(Error::rinex_builder("ephemerides is empty"));
        }

        Ok(Rinex {
            version: take_required(&mut self.version, "version")?,
            type_: take_required(&mut self.type_, "type")?,
            program: take_required(&mut self.program, "program")?,
            agency: take_required(&mut self.agency, "agency")?,
            update: take_required(&mut self.update, "update")?,
            comments: std::mem::take(&mut self.comments),
            ion_alpha: self.ion_alpha.take(),
            ion_beta: self.ion_beta.take(),
            delta_utc: self.delta_utc.take(),
            leap_seconds: self.leap_seconds.take(),
            ephemerides: take_required(&mut self.ephemerides, "ephemerides")?,
        })
    }
}

/// Returns a mandatory builder field by reference.
fn required_ref<'a, T>(
    value: Option<&'a T>, field: &'static str,
) -> Result<&'a T, Error> {
    value.ok_or_else(|| Error::rinex_builder(format!("{field} is missing")))
}

/// Takes a mandatory builder field.
fn take_required<T>(
    value: &mut Option<T>, field: &'static str,
) -> Result<T, Error> {
    value
        .take()
        .ok_or_else(|| Error::rinex_builder(format!("{field} is missing")))
}

/// Validates supported RINEX version and navigation type metadata.
fn validate_metadata(version: &str, type_: &str) -> Result<(), Error> {
    let numeric_version = to_float(version).map_err(|error| {
        Error::rule(format!("invalid RINEX version {version:?}: {error}"))
    })?;
    if !(2.0..3.0).contains(&numeric_version) {
        return Err(Error::rule(format!(
            "unsupported RINEX version {version:?}; expected 2.x"
        )));
    }

    let type_ = type_.trim();
    let normalized_type =
        type_.split_ascii_whitespace().collect::<Vec<_>>().join(" ");
    if !matches!(
        normalized_type.as_str(),
        "N" | "NAVIGATION DATA" | "N: GPS NAV DATA"
    ) {
        return Err(Error::rule(format!(
            "unsupported RINEX file type {type_:?}; expected GPS navigation \
             data"
        )));
    }
    Ok(())
}

/// Parses the RINEX header into the builder.
///
/// # Errors
/// Returns an error for malformed values, duplicate singleton records, or an
/// unexpected grammar rule.
pub fn read_header(
    header_rules: &mut Pairs<Rule>, builder: &mut RinexBuilder,
) -> Result<(), Error> {
    let mut header_record_seen = false;
    for header_rule in header_rules {
        match header_rule.as_rule() {
            Rule::header_version => {
                reject_duplicate(
                    builder.version.is_some() || builder.type_.is_some(),
                    "RINEX VERSION / TYPE",
                )?;
                if header_record_seen {
                    return Err(Error::rule(
                        "RINEX VERSION / TYPE must be the first header record",
                    ));
                }
                let mut rules = header_rule.into_inner();
                builder.set_version(
                    next_str(&mut rules, "header version")?.trim().to_string(),
                );
                builder.set_type(
                    next_str(&mut rules, "header type")?.trim().to_string(),
                );
            }
            Rule::header_program => {
                reject_duplicate(
                    builder.program.is_some()
                        || builder.agency.is_some()
                        || builder.update.is_some(),
                    "PGM / RUN BY / DATE",
                )?;
                let mut rules = header_rule.into_inner();
                builder.set_program(
                    next_str(&mut rules, "header program")?.trim().to_string(),
                );
                builder.set_agency(
                    next_str(&mut rules, "header agency")?.trim().to_string(),
                );
                builder.set_update(
                    next_str(&mut rules, "header date")?.trim().to_string(),
                );
            }
            Rule::header_comment => {
                let mut rules = header_rule.into_inner();
                builder.add_comment(
                    next_str(&mut rules, "header comment")?.trim().to_string(),
                );
            }
            Rule::header_ion_alpha => {
                reject_duplicate(builder.ion_alpha.is_some(), "ION ALPHA")?;
                builder.set_ion_alpha(read_ion_values(
                    &mut header_rule.into_inner(),
                    "ION ALPHA",
                )?);
            }
            Rule::header_ion_beta => {
                reject_duplicate(builder.ion_beta.is_some(), "ION BETA")?;
                builder.set_ion_beta(read_ion_values(
                    &mut header_rule.into_inner(),
                    "ION BETA",
                )?);
            }
            Rule::header_delta_utc => {
                reject_duplicate(builder.delta_utc.is_some(), "DELTA-UTC")?;
                builder.set_delta_utc(read_delta_utc(
                    &mut header_rule.into_inner(),
                )?);
            }
            Rule::header_leap_secs => {
                reject_duplicate(
                    builder.leap_seconds.is_some(),
                    "LEAP SECONDS",
                )?;
                let mut rules = header_rule.into_inner();
                builder.set_leap_seconds(to_int(next_str(
                    &mut rules,
                    "header leap seconds",
                )?)?);
            }
            rule => {
                return Err(Error::rule(format!(
                    "unexpected header rule: {rule:?}"
                )));
            }
        }
        header_record_seen = true;
    }
    Ok(())
}

/// Rejects duplicate singleton header records.
fn reject_duplicate(
    duplicate: bool, record: &'static str,
) -> Result<(), Error> {
    if duplicate {
        return Err(Error::rule(format!("duplicate {record} header record")));
    }
    Ok(())
}

/// Parses one four-value ionospheric correction record.
fn read_ion_values(
    rules: &mut Pairs<Rule>, context: &'static str,
) -> Result<[f64; 4], Error> {
    let mut values = [0.0; 4];
    for value in &mut values {
        *value = to_float(next_str(rules, context)?)?;
    }
    if rules.next().is_some() {
        return Err(Error::rule(format!(
            "unexpected extra value in {context} header record"
        )));
    }
    Ok(values)
}

/// Parses one DELTA-UTC record.
fn read_delta_utc(rules: &mut Pairs<Rule>) -> Result<DeltaUtc, Error> {
    let a0 = to_float(next_str(rules, "DELTA-UTC A0")?)?;
    let a1 = to_float(next_str(rules, "DELTA-UTC A1")?)?;
    let time = to_int(next_str(rules, "DELTA-UTC time")?)?;
    let week = to_int(next_str(rules, "DELTA-UTC week")?)?;
    Ok(DeltaUtc::new(a0, a1, time, week))
}
