use thiserror::Error;

use crate::rule::Rule;
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error("rinex file can not read")]
    ReadRinex(#[from] std::io::Error),
    #[error("can not parse to float")]
    ParseFloat(#[from] std::num::ParseFloatError),
    #[error("can not parse to int")]
    IntFloat(#[from] std::num::ParseIntError),
    #[error("cannot parse")]
    ParseFile(#[from] Box<pest::error::Error<Rule>>),
    #[error("cannot parse rule")]
    Rule(String),
    #[error("rinex builder error{0}")]
    RinexBuilder(String),
    #[error("ephemeris builder error{0}")]
    EphemerisBuilder(String),
    #[error("jiff error")]
    Jiff(#[from] jiff::Error),
    #[error("unknown Rinex error")]
    Unknown,
}
