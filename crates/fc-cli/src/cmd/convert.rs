//! Conversion of a font between the supported document and export formats.

use std::path::{Path, PathBuf};

use clap::Args as ClapArgs;
use fc_api::{ExportFormat, ExportOptions};

use super::{
    build::{export, extension, master_id, print_report},
    open_document,
};
use fc_cli::Error;

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Font, UFO, designspace, or Font Cutter document to read.
    pub(crate) input: PathBuf,
    /// Destination path; the extension selects the output format.
    pub(crate) output: PathBuf,
    /// Master to write, by name or identifier.
    #[arg(long, value_name = "NAME")]
    pub(crate) master: Option<String>,
}

pub(crate) fn run(args: Args) -> Result<(), Error> {
    let opened = open_document(&args.input, None)?;
    if is_fcf_path(&args.output) {
        return opened
            .session
            .with_doc_mut(opened.doc, |document| document.save(&args.output))
            .map_err(|source| Error::Save {
                path: args.output.clone(),
                source,
            })?
            .map_err(|source| Error::Save {
                path: args.output,
                source,
            });
    }
    let format = output_format(&args.output)?;
    let options = ExportOptions {
        format,
        master: master_id(&opened, args.master.as_deref())?,
        ..ExportOptions::default()
    };
    let report = export(&opened, &args.output, options)?;
    print_report(&report);
    Ok(())
}

fn output_format(path: &Path) -> Result<ExportFormat, Error> {
    match extension(path).as_str() {
        "ttf" => Ok(ExportFormat::Ttf),
        "woff" => Ok(ExportFormat::Woff),
        "woff2" => Ok(ExportFormat::Woff2),
        "ufo" => Ok(ExportFormat::Ufo),
        "designspace" => Ok(ExportFormat::Designspace),
        extension => Err(Error::UnsupportedOutput {
            extension: extension.to_owned(),
            supported: "fcf, fcf.gz, ttf, woff, woff2, ufo, designspace",
        }),
    }
}

fn is_fcf_path(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    name.ends_with(".fcf")
        || name.ends_with(".fcf.gz")
        // Documents written before the `.fwf` extension was renamed.
        || name.ends_with(".fwf")
        || name.ends_with(".fwf.gz")
}
