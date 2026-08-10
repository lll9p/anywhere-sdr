use std::{fs, num::ParseIntError, path::Path};

use pest::{
    Parser,
    iterators::{Pair, Pairs},
};
use pest_derive::Parser;

use crate::error::Error;

/// Ephemeris record parsing.
mod ephemeris_rules;
/// Header model and parsing.
mod header;

use ephemeris_rules::read_ephemerides;
pub use header::{Rinex, RinexBuilder, read_header};

pub(super) use crate::utils::{
    parse_i32 as to_int, parse_rinex_f64 as to_float,
};

/// Parser implementation for RINEX files using pest grammar.
#[derive(Parser)]
#[grammar = "rinex.pest"]
pub struct RinexParser;

impl Rinex {
    /// Reads a RINEX navigation file from the filesystem.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or parsed.
    pub fn read_file(path: &dyn AsRef<Path>) -> Result<Self, Error> {
        let data = fs::read_to_string(path)?;
        Self::read_string(&data)
    }

    /// Parses a RINEX navigation file from a string.
    ///
    /// # Errors
    /// Returns an error if the input is not supported RINEX 2 GPS navigation
    /// data.
    pub fn read_string(data: &str) -> Result<Self, Error> {
        let mut parser = RinexParser::parse(Rule::rinex, data)
            .map_err(|error| Error::ParseFile(Box::new(error)))?;
        let top_pair = parser
            .next()
            .ok_or_else(|| Error::rule("empty parsing result"))?;
        if top_pair.as_rule() != Rule::rinex {
            return Err(Error::rule(format!(
                "expected RINEX root rule, found {:?}",
                top_pair.as_rule()
            )));
        }
        if parser.next().is_some() {
            return Err(Error::rule(
                "unexpected additional data after RINEX content",
            ));
        }

        let mut builder = RinexBuilder::new();
        for section in top_pair.into_inner() {
            match section.as_rule() {
                Rule::header => {
                    read_header(&mut section.into_inner(), &mut builder)?;
                }
                Rule::ephemerides => {
                    read_ephemerides(&mut section.into_inner(), &mut builder)?;
                }
                Rule::EOI => {}
                rule => {
                    return Err(Error::rule(format!(
                        "unexpected top-level rule: {rule:?}"
                    )));
                }
            }
        }
        builder.build()
    }
}

/// Returns the next grammar pair with contextual missing-field diagnostics.
fn next_pair<'a>(
    pairs: &mut Pairs<'a, Rule>, context: &'static str,
) -> Result<Pair<'a, Rule>, Error> {
    pairs.next().ok_or_else(|| {
        Error::rule(format!("missing expected rule in {context}"))
    })
}

/// Returns the source text of the next grammar pair.
pub(super) fn next_str<'a>(
    pairs: &mut Pairs<'a, Rule>, context: &'static str,
) -> Result<&'a str, Error> {
    Ok(next_pair(pairs, context)?.as_str())
}

/// Parses a trimmed unsigned integer field.
pub(super) fn to_usize(num: &str) -> Result<usize, ParseIntError> {
    num.trim().parse()
}
