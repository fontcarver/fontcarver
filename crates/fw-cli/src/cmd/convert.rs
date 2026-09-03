use std::path::{Path, PathBuf};

use clap::Args as ClapArgs;

use super::open_document;
use fw_cli::Error;

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Font or FontWorkshop document to read.
    pub(crate) input: PathBuf,
    /// Destination `.fwf` or `.fwf.gz` path.
    pub(crate) output: PathBuf,
}

pub(crate) fn run(args: Args) -> Result<(), Error> {
    validate_output(&args.output)?;
    let opened = open_document(&args.input, None)?;
    opened
        .session
        .with_doc_mut(opened.doc, |document| document.save(&args.output))
        .map_err(|source| Error::Save {
            path: args.output.clone(),
            source,
        })?
        .map_err(|source| Error::Save {
            path: args.output,
            source,
        })
}

fn validate_output(path: &Path) -> Result<(), Error> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let normalized = file_name.to_ascii_lowercase();
    if normalized.ends_with(".fwf") || normalized.ends_with(".fwf.gz") {
        return Ok(());
    }
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_owned();
    Err(Error::UnsupportedOutput { extension })
}
