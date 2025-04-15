#![allow(non_camel_case_types, non_snake_case, clippy::missing_safety_doc)]

pub fn tracing_init() -> tracing_appender::non_blocking::WorkerGuard {
    let file_appender = tracing_appender::rolling::daily("./", "app.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_file(true)
        .with_line_number(true)
        .with_writer(non_blocking)
        .with_ansi(false)
        .init();
    guard
}

mod cli;
mod constants;
mod datetime;
mod eph;
mod ionoutc;
mod process;
mod read_nmea_gga;
mod read_rinex;
mod read_user_motion;
mod table;
mod utils;

use clap::Parser;
use datetime::{datetime_t, gpstime_t};
use eph::ephem_t;
use ionoutc::ionoutc_t;
use process::process;

pub fn main() -> anyhow::Result<()> {
    let _guard = tracing_init();

    let params = cli::Args::parse().get_params();
    ::std::process::exit(process(params))
}
