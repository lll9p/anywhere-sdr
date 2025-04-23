use std::{
    fs,
    num::{ParseFloatError, ParseIntError},
    path::Path,
};

use jiff::Timestamp;
use pest::{
    Parser,
    iterators::{Pair, Pairs},
};
use pest_derive::Parser;

use crate::{
    ephemeris::{
        Ephemeris, EphemerisBuilder, Orbit1, Orbit2, Orbit3, Orbit4, Orbit5,
        Orbit6, Orbit7, SvClock,
    },
    error::Error,
    utc::DeltaUtc,
};

#[derive(Parser)]
#[grammar = "rinex.pest"]
pub struct RinexParser;

#[derive(Debug)]
pub struct Rinex {
    /// Format version
    pub version: String,
    /// File type
    pub type_: String,
    /// Program name
    pub program: String,
    /// Agency
    pub agency: String,
    /// Date
    pub update: String,
    /// Comments
    pub comments: String,
    /// Ionosphere parameters A0-A3 of almanac
    pub ion_alpha: [f64; 4],
    /// Ionosphere parameters B0-B3 of almanac
    pub ion_beta: [f64; 4],
    /// Almanac parameters to compute time in UTC
    pub delta_utc: DeltaUtc,
    /// Delta time due to leap seconds
    pub leap_seconds: i32,
    /// Ephemeris data
    pub ephemerides: Vec<Ephemeris>,
}
impl Rinex {
    pub fn read_file(path: &dyn AsRef<Path>) -> Result<Self, Error> {
        let data = fs::read_to_string(path)?;
        Self::read_string(data.as_str())
    }

    pub fn read_string(data: &str) -> Result<Self, Error> {
        let mut parser = RinexParser::parse(Rule::rinex, data)
            .map_err(|e| Error::ParseFile(Box::new(e)))?;
        let top_pair = parser
            .next()
            .ok_or_else(|| Error::Rule("Empty parsing result".to_string()))?;

        // Ensure it's the correct rule if needed (optional check)
        if top_pair.as_rule() != Rule::rinex {
            return Err(Error::Rule(format!(
                "Expected Rule::rinex, found {:?}",
                top_pair.as_rule()
            )));
        }
        // Check if there are more top-level pairs than expected (optional
        // check)
        if parser.next().is_some() {
            return Err(Error::Rule(
                "Unexpected additional data after RINEX content".to_string(),
            ));
        }
        let mut builder = RinexBuilder::new();
        for line in top_pair.into_inner() {
            match line.as_rule() {
                Rule::header => {
                    read_header(&mut line.into_inner(), &mut builder)?;
                }
                Rule::ephemerides => {
                    read_ephemerides(&mut line.into_inner(), &mut builder)?;
                }
                Rule::EOI => {} // Expected end of input marker by pest
                _ => {
                    return Err(Error::Rule(format!(
                        "Unexpected rule at top level: {:?}",
                        line.as_rule()
                    )));
                }
            }
        }
        let rinex = builder.build()?;
        Ok(rinex)
    }
}

#[derive(Debug, Default)]
pub struct RinexBuilder {
    version: Option<String>,
    type_: Option<String>,
    program: Option<String>,
    agency: Option<String>,
    update: Option<String>,
    comments: Option<String>,
    ion_alpha: Option<[f64; 4]>,
    ion_beta: Option<[f64; 4]>,
    delta_utc: Option<DeltaUtc>,
    leap_seconds: Option<i32>,
    ephemerides: Option<Vec<Ephemeris>>,
}
impl RinexBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_version(&mut self, version: String) {
        self.version.replace(version);
    }

    pub fn set_type(&mut self, type_: String) {
        self.type_.replace(type_);
    }

    pub fn set_program(&mut self, program: String) {
        self.program.replace(program);
    }

    pub fn set_agency(&mut self, agency: String) {
        self.agency.replace(agency);
    }

    pub fn set_update(&mut self, update: String) {
        self.update.replace(update);
    }

    pub fn set_comments(&mut self, comments: String) {
        self.comments.replace(comments);
    }

    pub fn set_ion_alpha(&mut self, ion_alpha: [f64; 4]) {
        self.ion_alpha.replace(ion_alpha);
    }

    pub fn set_ion_beta(&mut self, ion_beta: [f64; 4]) {
        self.ion_beta.replace(ion_beta);
    }

    pub fn set_delta_utc(&mut self, delta_utc: DeltaUtc) {
        self.delta_utc.replace(delta_utc);
    }

    pub fn set_leap_seconds(&mut self, leap_seconds: i32) {
        self.leap_seconds.replace(leap_seconds);
    }

    pub fn set_ephemerides(&mut self, ephemerides: Vec<Ephemeris>) {
        self.ephemerides.replace(ephemerides);
    }

    pub fn build(&mut self) -> Result<Rinex, Error> {
        fn take<T>(v: &mut Option<T>, msg: &str) -> Result<T, Error> {
            v.take().ok_or_else(|| Error::RinexBuilder(msg.into()))
        }
        let rinex = Rinex {
            version: take(&mut self.version, "version is none")?,
            type_: take(&mut self.type_, "type is none")?,
            program: take(&mut self.program, "program is none")?,
            agency: take(&mut self.agency, "agency is none")?,
            update: take(&mut self.update, "update is none")?,
            comments: take(&mut self.comments, "comments is none")?,
            ion_alpha: take(&mut self.ion_alpha, "ion_alpha is none")?,
            ion_beta: take(&mut self.ion_beta, "ion_beta is none")?,
            delta_utc: take(&mut self.delta_utc, "delta_utc is none")?,
            leap_seconds: take(&mut self.leap_seconds, "leap_seconds is none")?,
            ephemerides: take(&mut self.ephemerides, "ephemerides is none")?,
        };
        Ok(rinex)
    }
}
/// Helper to get the next pair from an iterator or return an error.
fn next_pair<'a>(
    pairs: &mut Pairs<'a, Rule>, context: &'static str,
) -> Result<Pair<'a, Rule>, Error> {
    pairs.next().ok_or_else(|| {
        Error::Rule(format!("Missing expected rule in {context}"))
    })
}

/// Helper to get the next pair's inner text or return an error.
fn next_str<'a>(
    pairs: &mut Pairs<'a, Rule>, context: &'static str,
) -> Result<&'a str, Error> {
    Ok(next_pair(pairs, context)?.as_str())
}

fn to_float(num: &str) -> Result<f64, ParseFloatError> {
    num.replace('D', "E").trim().parse()
}

fn to_int(num: &str) -> Result<i32, ParseIntError> {
    num.trim().parse()
}
fn to_usize(num: &str) -> Result<usize, ParseIntError> {
    num.trim().parse()
}

pub fn read_header(
    header_rules: &mut Pairs<Rule>, builder: &mut RinexBuilder,
) -> Result<(), Error> {
    for header_rule in header_rules {
        match header_rule.as_rule() {
            Rule::header_version => {
                let mut rules = header_rule.into_inner();
                let version =
                    next_str(&mut rules, "header_version")?.trim().to_string();
                let type_ =
                    next_str(&mut rules, "header_version")?.trim().to_string();
                builder.set_version(version);
                builder.set_type(type_);
            }
            Rule::header_program => {
                let mut rules = header_rule.into_inner();
                let program =
                    next_str(&mut rules, "header_program")?.trim().to_string();
                let agency =
                    next_str(&mut rules, "header_program")?.trim().to_string();
                let update =
                    next_str(&mut rules, "header_program")?.trim().to_string();
                builder.set_program(program);
                builder.set_agency(agency);
                builder.set_update(update);
            }
            Rule::header_comment => {
                let mut rules = header_rule.into_inner();
                // Comments might span multiple inner parts depending on
                // grammar, but original code took only the
                // first. Assuming simple structure.
                let comments =
                    next_str(&mut rules, "header_comment")?.trim().to_string();
                builder.set_comments(comments);
            }
            Rule::header_ion_alpha => {
                let mut rules = header_rule.into_inner();
                let ion_alpha = read_ion_values(&mut rules)?;
                builder.set_ion_alpha(ion_alpha);
            }
            Rule::header_ion_beta => {
                let mut rules = header_rule.into_inner();
                let ion_beta = read_ion_values(&mut rules)?;
                builder.set_ion_beta(ion_beta);
            }
            Rule::header_delta_utc => {
                let mut rules = header_rule.into_inner();
                let delta_utc = read_delta_utc(&mut rules)?;
                builder.set_delta_utc(delta_utc);
            }
            Rule::header_leap_secs => {
                let mut rules = header_rule.into_inner();
                let leap_seconds_str =
                    next_str(&mut rules, "header_leap_secs")?;
                let leap_seconds = to_int(leap_seconds_str)?;
                builder.set_leap_seconds(leap_seconds);
            }
            _ => unreachable!(),
        }
    }
    Ok(())
}
fn read_ion_values(rule: &mut Pairs<Rule>) -> Result<[f64; 4], Error> {
    let mut values: [f64; 4] = [0.0, 0.0, 0.0, 0.0];
    for (i, line) in rule.enumerate() {
        match line.as_rule() {
            Rule::ion_a0
            | Rule::ion_a1
            | Rule::ion_a2
            | Rule::ion_a3
            | Rule::ion_b0
            | Rule::ion_b1
            | Rule::ion_b2
            | Rule::ion_b3 => {
                let a = to_float(line.as_str())?;
                values[i] = a;
            }
            _ => unreachable!(),
        }
    }
    Ok(values)
}
fn read_delta_utc(rule: &mut Pairs<Rule>) -> Result<DeltaUtc, Error> {
    let a0 = to_float(next_str(rule, "delta_utc a0")?)?;
    let a1 = to_float(next_str(rule, "delta_utc a1")?)?;
    let time = to_int(next_str(rule, "delta_utc time")?)?;
    let week = to_int(next_str(rule, "delta_utc week")?)?;
    Ok(DeltaUtc::new(a0, a1, time, week))
}

pub fn read_ephemerides(
    eph_rules: &mut Pairs<Rule>, builder: &mut RinexBuilder,
) -> Result<(), Error> {
    let mut ephemerides: Vec<Ephemeris> = Vec::new();
    for eph_rule in eph_rules {
        match eph_rule.as_rule() {
            Rule::ephemeris => {
                let mut eph_builder = EphemerisBuilder::new();
                let mut rules = eph_rule.into_inner();
                read_ephemeris(&mut rules, &mut eph_builder)?;
                let ephemeris = eph_builder.build()?;
                ephemerides.push(ephemeris);
            }
            _ => {
                return Err(Error::Rule(format!(
                    "Unexpected rule in ephemerides section: {:?}",
                    eph_rule.as_rule()
                )));
            }
        }
    }
    builder.set_ephemerides(ephemerides);
    Ok(())
}
#[allow(clippy::similar_names)]
fn read_ephemeris(
    rules: &mut Pairs<Rule>, builder: &mut EphemerisBuilder,
) -> Result<(), Error> {
    // Expect a specific sequence of rules based on the grammar
    // PRN + Epoch + SV Clock + 7 Orbit lines
    for rule in rules {
        match rule.as_rule() {
            Rule::prn => {
                let prn = to_usize(rule.as_str())?;
                builder.set_prn(prn);
            }
            Rule::epoch => {
                let mut epoch_rules = rule.into_inner();
                let year = to_int(next_str(&mut epoch_rules, "epoch year")?)?;
                let month = to_int(next_str(&mut epoch_rules, "epoch month")?)?;
                let day = to_int(next_str(&mut epoch_rules, "epoch day")?)?;
                let hour = to_int(next_str(&mut epoch_rules, "epoch hour")?)?;
                let minutes =
                    to_int(next_str(&mut epoch_rules, "epoch minutes")?)?;
                let seconds =
                    to_float(next_str(&mut epoch_rules, "epoch seconds")?)?;
                let datetime = format!(
                    "20{year}-{month:02}-{day:02}T{hour:02}:{minutes:02}:00Z"
                );
                let time_of_clock: Timestamp =
                    datetime.parse::<Timestamp>()?.checked_add(
                        std::time::Duration::from_secs_f64(seconds),
                    )?;
                builder.set_time_of_clock(time_of_clock);
            }
            Rule::sv_clk => {
                let mut sv_clk_rules = rule.into_inner();
                let bias =
                    to_float(next_str(&mut sv_clk_rules, "sv_clk bias")?)?;
                let drift =
                    to_float(next_str(&mut sv_clk_rules, "sv_clk drift")?)?;
                let drift_rate = to_float(next_str(
                    &mut sv_clk_rules,
                    "sv_clk drift_rate",
                )?)?;
                let sv_clock = SvClock::new(bias, drift, drift_rate);
                builder.set_sv_clock(sv_clock);
            }
            Rule::orbit_1 => {
                let mut rules = rule.into_inner();
                let orbit: Orbit1 = to_orbit_values(&mut rules, "orbit_1")?;
                builder.set_orbit1(orbit);
            }
            Rule::orbit_2 => {
                let mut rules = rule.into_inner();
                let orbit: Orbit2 = to_orbit_values(&mut rules, "orbit_2")?;
                builder.set_orbit2(orbit);
            }
            Rule::orbit_3 => {
                let mut rules = rule.into_inner();
                let orbit: Orbit3 = to_orbit_values(&mut rules, "orbit_3")?;
                builder.set_orbit3(orbit);
            }
            Rule::orbit_4 => {
                let mut rules = rule.into_inner();
                let orbit: Orbit4 = to_orbit_values(&mut rules, "orbit_4")?;
                builder.set_orbit4(orbit);
            }
            Rule::orbit_5 => {
                let mut rules = rule.into_inner();
                let orbit: Orbit5 = to_orbit_values(&mut rules, "orbit_5")?;
                builder.set_orbit5(orbit);
            }
            Rule::orbit_6 => {
                let mut rules = rule.into_inner();
                let orbit: Orbit6 = to_orbit_values(&mut rules, "orbit_6")?;
                builder.set_orbit6(orbit);
            }
            Rule::orbit_7 => {
                let mut rules = rule.into_inner();
                let orbit: Orbit7 = to_orbit_values(&mut rules, "orbit_7")?;
                builder.set_orbit7(orbit);
            }
            _ => unreachable!(),
        }
    }
    Ok(())
}
fn to_orbit_values<O: From<[f64; 4]>>(
    rules: &mut Pairs<Rule>, context: &'static str,
) -> Result<O, Error> {
    let mut values = [0.0; 4];
    for item in &mut values {
        let val_str = next_str(rules, context)?;
        *item = to_float(val_str)?;
    }
    let orbit = O::from(values);
    Ok(orbit)
}
