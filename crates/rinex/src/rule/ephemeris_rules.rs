use pest::iterators::Pairs;

use super::{RinexBuilder, Rule, next_str, to_float, to_int, to_usize};
use crate::{
    Error,
    ephemeris::{Ephemeris, EphemerisBuilder, GpsCalendarDateTime, SvClock},
};

/// Parses the ephemerides section and populates the RINEX builder.
pub(super) fn read_ephemerides(
    eph_rules: &mut Pairs<Rule>, builder: &mut RinexBuilder,
) -> Result<(), Error> {
    let mut ephemerides: Vec<Ephemeris> = Vec::new();
    for eph_rule in eph_rules {
        match eph_rule.as_rule() {
            Rule::ephemeris => {
                let mut eph_builder = EphemerisBuilder::new();
                let mut rules = eph_rule.into_inner();
                read_ephemeris(&mut rules, &mut eph_builder)?;
                ephemerides.push(eph_builder.build()?);
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

/// Parses one satellite navigation record into its ephemeris builder.
#[allow(clippy::similar_names)]
fn read_ephemeris(
    rules: &mut Pairs<Rule>, builder: &mut EphemerisBuilder,
) -> Result<(), Error> {
    for rule in rules {
        match rule.as_rule() {
            Rule::prn => {
                let prn_text = rule.as_str().trim();
                let prn = to_usize(prn_text).map_err(|error| {
                    Error::rule(format!(
                        "invalid GPS PRN {prn_text:?}: {error}"
                    ))
                })?;
                if !(1..=32).contains(&prn) {
                    return Err(Error::rule(format!(
                        "GPS PRN {prn} is outside the supported range 1..=32"
                    )));
                }
                builder.set_prn(prn);
            }
            Rule::epoch => {
                let mut epoch_rules = rule.into_inner();
                let year = to_int(next_str(&mut epoch_rules, "epoch year")?)?;
                let month = to_int(next_str(&mut epoch_rules, "epoch month")?)?;
                let day = to_int(next_str(&mut epoch_rules, "epoch day")?)?;
                let hour = to_int(next_str(&mut epoch_rules, "epoch hour")?)?;
                let minute =
                    to_int(next_str(&mut epoch_rules, "epoch minutes")?)?;
                let second =
                    to_float(next_str(&mut epoch_rules, "epoch seconds")?)?;
                let year = expand_rinex_2_year(year)?;
                builder.set_time_of_clock(GpsCalendarDateTime::new(
                    year, month, day, hour, minute, second,
                )?);
            }
            Rule::sv_clk => {
                let mut clock_rules = rule.into_inner();
                let bias =
                    to_float(next_str(&mut clock_rules, "sv_clk bias")?)?;
                let drift =
                    to_float(next_str(&mut clock_rules, "sv_clk drift")?)?;
                let drift_rate =
                    to_float(next_str(&mut clock_rules, "sv_clk drift_rate")?)?;
                builder.set_sv_clock(SvClock::new(bias, drift, drift_rate));
            }
            Rule::orbit_1 => {
                builder.set_orbit1(read_orbit(rule.into_inner(), "orbit_1")?);
            }
            Rule::orbit_2 => {
                builder.set_orbit2(read_orbit(rule.into_inner(), "orbit_2")?);
            }
            Rule::orbit_3 => {
                builder.set_orbit3(read_orbit(rule.into_inner(), "orbit_3")?);
            }
            Rule::orbit_4 => {
                builder.set_orbit4(read_orbit(rule.into_inner(), "orbit_4")?);
            }
            Rule::orbit_5 => {
                builder.set_orbit5(read_orbit(rule.into_inner(), "orbit_5")?);
            }
            Rule::orbit_6 => {
                builder.set_orbit6(read_orbit(rule.into_inner(), "orbit_6")?);
            }
            Rule::orbit_7 => {
                builder.set_orbit7(read_orbit(rule.into_inner(), "orbit_7")?);
            }
            _ => {
                return Err(Error::Rule(format!(
                    "Unexpected rule in ephemeris record: {:?}",
                    rule.as_rule()
                )));
            }
        }
    }
    Ok(())
}

/// Expands a RINEX 2 two-digit navigation epoch year to the full year.
fn expand_rinex_2_year(year: i32) -> Result<i32, Error> {
    match year {
        0..=79 => Ok(2_000 + year),
        80..=99 => Ok(1_900 + year),
        _ => Err(Error::rule(format!(
            "RINEX 2 epoch year {year} is outside the two-digit range 0..=99"
        ))),
    }
}

/// Parses the four floating-point values in one RINEX orbit row.
fn read_orbit<O: From<[f64; 4]>>(
    mut rules: Pairs<Rule>, context: &'static str,
) -> Result<O, Error> {
    let mut values = [0.0; 4];
    for item in &mut values {
        *item = to_float(next_str(&mut rules, context)?)?;
    }
    Ok(O::from(values))
}
