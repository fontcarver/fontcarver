//! Compilation of a document into a binary or source font.

use std::path::{Path, PathBuf};

use clap::Args as ClapArgs;
use fc_api::{DocSummary, ExportFormat, ExportOptions, ExportParams, ExportReport};
use fc_model::MasterId;

use super::{api_call, open_document, OpenedDocument};
use fc_cli::Error;

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Font or FontCarver document to compile.
    pub(crate) input: PathBuf,
    /// Destination compiled font path (TTF, OTF, WOFF, Type 1/42, CFF, TTX or TTC).
    pub(crate) output: PathBuf,
    /// Remove overlapping contours before compiling.
    #[arg(long)]
    pub(crate) remove_overlap: bool,
    /// Insert missing curve extrema in the static export copy.
    #[arg(long, conflicts_with = "variable")]
    pub(crate) add_extrema: bool,
    /// Round outline coordinates and metrics in the static export copy.
    #[arg(long, conflicts_with = "variable")]
    pub(crate) round_coordinates: bool,
    /// Expand components into contours in the static export copy.
    #[arg(long, conflicts_with = "variable")]
    pub(crate) decompose_components: bool,
    /// Use supplied production glyph names in compiled output.
    #[arg(long)]
    pub(crate) production_names: bool,
    /// Share repeated charstrings in static OTF or bare CFF output.
    #[arg(long)]
    pub(crate) subroutinize: bool,
    /// Automatically hint a static sfnt export (TrueType requires ttfautohint on PATH).
    #[arg(long, conflicts_with = "variable")]
    pub(crate) autohint: bool,
    /// Compile from the model alone, without imported tables or hinting.
    #[arg(long)]
    pub(crate) no_preserve: bool,
    /// Compile all masters into a variable TrueType or CFF2 font.
    #[arg(long, conflicts_with_all = ["master", "instance"])]
    pub(crate) variable: bool,
    /// Export a named static instance by zero-based index.
    #[arg(long, conflicts_with = "master")]
    pub(crate) instance: Option<usize>,
    /// Select variable source masters by name or identifier (repeatable).
    #[arg(long, requires = "variable", value_name = "NAME")]
    pub(crate) source_master: Vec<String>,
    /// Master to compile, by name or identifier.
    #[arg(long, value_name = "NAME")]
    pub(crate) master: Option<String>,
}

pub(crate) fn run(args: Args) -> Result<(), Error> {
    let format = binary_format(&args.output)?;
    let opened = open_document(&args.input, None)?;
    let options = ExportOptions {
        format,
        variable: args.variable,
        instance: args.instance,
        master_subset: (!args.source_master.is_empty())
            .then(|| {
                args.source_master
                    .iter()
                    .map(|name| {
                        master_id(&opened, Some(name))?
                            .ok_or_else(|| Error::UnknownMaster { name: name.clone() })
                    })
                    .collect::<Result<Vec<_>, Error>>()
            })
            .transpose()?,
        master: master_id(&opened, args.master.as_deref())?,
        add_extrema: args.add_extrema,
        round_coordinates: args.round_coordinates,
        decompose_components: args.decompose_components,
        production_names: args.production_names,
        subroutinize: args.subroutinize,
        remove_overlap: args.remove_overlap,
        autohint: args.autohint,
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
            collection_docs: Vec::new(),
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
        "otf" => Ok(ExportFormat::Otf),
        "pfa" => Ok(ExportFormat::Pfa),
        "pfb" => Ok(ExportFormat::Pfb),
        "woff" => Ok(ExportFormat::Woff),
        "woff2" => Ok(ExportFormat::Woff2),
        "ttx" => Ok(ExportFormat::Ttx),
        "ttc" => Ok(ExportFormat::Ttc),
        "cff" => Ok(ExportFormat::Cff),
        "t42" | "type42" => Ok(ExportFormat::Type42),
        extension => Err(Error::UnsupportedOutput {
            extension: extension.to_owned(),
            supported: "ttf, otf, woff, woff2, pfa, pfb, ttx, ttc, cff, t42",
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
