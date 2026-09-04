//! Compilation of a document into a binary or source font.

use std::path::{Path, PathBuf};

use clap::Args as ClapArgs;
use fw_api::{DocSummary, ExportFormat, ExportOptions, ExportParams, ExportReport};
use fw_model::MasterId;

use super::{api_call, open_document, OpenedDocument};
use fw_cli::Error;

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Font or FontWorkshop document to compile.
    pub(crate) input: PathBuf,
    /// Destination `.ttf`, `.woff`, or `.woff2` path.
    pub(crate) output: PathBuf,
    /// Remove overlapping contours before compiling.
    #[arg(long)]
    pub(crate) remove_overlap: bool,
    /// Compile from the model alone, without imported tables or hinting.
    #[arg(long)]
    pub(crate) no_preserve: bool,
    /// Master to compile, by name or identifier.
    #[arg(long, value_name = "NAME")]
    pub(crate) master: Option<String>,
}

pub(crate) fn run(args: Args) -> Result<(), Error> {
    let format = binary_format(&args.output)?;
    let opened = open_document(&args.input, None)?;
    let options = ExportOptions {
        format,
        master: master_id(&opened, args.master.as_deref())?,
        remove_overlap: args.remove_overlap,
        keep_tt_instructions: !args.no_preserve,
        preserve_untouched_tables: !args.no_preserve,
        ..ExportOptions::default()
    };
    let report = export(&opened, &args.output, options)?;
    print_report(&report);
    Ok(())
}

/// Exports one open document and returns the kernel's report.
pub(crate) fn export(
    opened: &OpenedDocument,
    output: &Path,
    options: ExportOptions,
) -> Result<ExportReport, Error> {
    api_call(
        &opened.api,
        "session.export",
        ExportParams {
            doc: opened.doc,
            path: output.to_string_lossy().into_owned(),
            options,
        },
    )
}

/// Resolves a master name or identifier against the open document.
pub(crate) fn master_id(
    opened: &OpenedDocument,
    requested: Option<&str>,
) -> Result<Option<MasterId>, Error> {
    let Some(requested) = requested else {
        return Ok(None);
    };
    let summary: DocSummary = api_call(
        &opened.api,
        "doc.summary",
        serde_json::json!({ "doc": opened.doc }),
    )?;
    summary
        .masters
        .into_iter()
        .find(|master| master.name == requested || master.id.to_string() == requested)
        .map(|master| Some(master.id))
        .ok_or_else(|| Error::UnknownMaster {
            name: requested.to_owned(),
        })
}

/// Prints the written path, its size, and any export diagnostics.
pub(crate) fn print_report(report: &ExportReport) {
    for warning in &report.warnings {
        eprintln!("warning: {}: {}", warning.code, warning.message);
    }
    println!(
        "wrote {} ({} bytes, {} ms)",
        report.path, report.bytes_written, report.timing_ms
    );
}

/// Maps a binary output extension to its export format.
pub(crate) fn binary_format(path: &Path) -> Result<ExportFormat, Error> {
    match extension(path).as_str() {
        "ttf" => Ok(ExportFormat::Ttf),
        "woff" => Ok(ExportFormat::Woff),
        "woff2" => Ok(ExportFormat::Woff2),
        extension => Err(Error::UnsupportedOutput {
            extension: extension.to_owned(),
            supported: "ttf, woff, woff2",
        }),
    }
}

/// Returns the lowercase extension of `path`, or an empty string.
pub(crate) fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}
