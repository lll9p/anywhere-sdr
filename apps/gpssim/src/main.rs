mod cli;
mod error;
mod utils;

use clap::Parser;
pub use error::Error;

pub fn main() -> Result<(), Error> {
    let _guard = utils::tracing_init();

    let cli = cli::Args::parse();
    cli.run()
}
