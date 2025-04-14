// use clap::{Parser, Subcommand};
// pub mod sizer;
// pub mod xrd;
// #[derive(Parser, Debug)] // requires `derive` feature
// #[command(term_width = 0)] // Just to make testing across clap features easier
// #[command(name = "parser")]
// #[command(version, about="An experimental data parser", long_about = None)]
// #[command(propagate_version = true)]
// pub struct Cli {
//     #[command(subcommand)]
//     pub commands: Commands,
// }
//
// #[allow(clippy::upper_case_acronyms)]
// #[derive(Subcommand, Debug)]
// pub enum Commands {
//     Report,
//     #[command(arg_required_else_help = true)]
//     XRD(xrd::Commands),
//     #[command(arg_required_else_help = true)]
//     Sizer(sizer::Commands),
// }
