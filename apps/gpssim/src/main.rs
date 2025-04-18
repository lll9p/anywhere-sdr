mod cli;
mod utils;
use clap::Parser;
pub fn main() -> anyhow::Result<()> {
    let _guard = utils::tracing_init();

    let cli = cli::Args::parse();
    cli.run()
}
