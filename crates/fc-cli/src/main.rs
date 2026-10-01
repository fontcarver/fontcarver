//! FontCarver command-line entry point.

mod cmd;

use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "fontcarver", version, about = "FontCarver command-line tools")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Show document metadata and preserved font tables.
    Inspect(cmd::inspect::Args),
    /// Find document problems (exit 1 when errors are detected).
    Check(cmd::check::Args),
    /// Convert a font between document, source, and binary formats.
    Convert(cmd::convert::Args),
    /// Compile a document into a binary font.
    ///
    /// Boxed because `build` takes every export and hinting option, which makes
    /// its argument record much larger than any other subcommand's.
    Build(Box<cmd::build::Args>),
    /// List the export profiles and variable-font configurations a project stores.
    Profiles(cmd::profiles::Args),
    /// Shape text with the document's own compiled font.
    Shape(cmd::shape::Args),
    /// List and filter glyphs in a font.
    Glyphs(cmd::glyphs::Args),
    /// Compile multiple inputs into a TrueType/OpenType collection.
    Collection(cmd::collection::Args),
}

impl Command {
    fn run(self) -> Result<(), fc_cli::Error> {
        match self {
            Self::Inspect(args) => cmd::inspect::run(args),
            Self::Check(args) => cmd::check::run(args),
            Self::Convert(args) => cmd::convert::run(args),
            Self::Build(args) => cmd::build::run(*args),
            Self::Profiles(args) => cmd::profiles::run(args),
            Self::Shape(args) => cmd::shape::run(args),
            Self::Glyphs(args) => cmd::glyphs::run(args),
            Self::Collection(args) => cmd::collection::run(args),
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
