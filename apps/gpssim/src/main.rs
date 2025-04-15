mod cli;
mod utils;
use clap::Parser;
use gps::process;
pub fn main() -> anyhow::Result<()> {
    let _guard = utils::tracing_init();

    let params = cli::Args::parse().get_params();
    ::std::process::exit(process(params))
}
