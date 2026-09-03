//! FontWorkshop command-line entry point.

mod cmd;

use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "fw", version, about = "FontWorkshop command-line tools")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Show document metadata and preserved font tables.
    Inspect(cmd::inspect::Args),
    /// Convert a supported font to a FontWorkshop document.
    Convert(cmd::convert::Args),
    /// List and filter glyphs in a font.
    Glyphs(cmd::glyphs::Args),
}

impl Command {
    fn run(self) -> Result<(), fw_cli::Error> {
        match self {
            Self::Inspect(args) => cmd::inspect::run(args),
            Self::Convert(args) => cmd::convert::run(args),
            Self::Glyphs(args) => cmd::glyphs::run(args),
        }
    }
}

fn main() -> ExitCode {
    match Cli::parse().command.run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(error.exit_code())
        }
    }
}
