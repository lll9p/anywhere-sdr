// use clap::{ArgAction, Args};
// use parser::sizer::runner;
// use std::path::PathBuf;
//
// #[derive(Debug, Args)]
// #[command(args_conflicts_with_subcommands = true)]
// #[command(flatten_help = true)]
// pub struct Commands {
//     /// Run the interactive mode, this is the default mode
//     #[arg(short, long,
//         action = ArgAction::SetFalse)]
//     interact: Option<bool>,
//
//     /// Get list of run in apkw
//     #[arg(short, long, action = ArgAction::SetTrue,
//         conflicts_with_all = &["interact"])]
//     list: Option<bool>,
//     /// Output file
//     #[arg(short, long,
//         conflicts_with_all = &["interact"])]
//     file: Option<PathBuf>,
//     /// Output file
//     #[arg(short, long,
//         conflicts_with_all = &["interact", "list"])]
//     output: Option<PathBuf>,
// }
// impl Commands {
//     pub fn run(&self) -> anyhow::Result<()> {
//         runner::run_interactive().unwrap();
//         Ok(())
//     }
// }
