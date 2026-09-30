//! Compilation of a document into a binary or source font.

use std::path::{Path, PathBuf};

use clap::Args as ClapArgs;
use fc_api::{
    DocSummary, ExportFormat, ExportOptions, ExportParams, ExportProfilesResult, ExportReport,
    ResolveExportProfileParams, ResolvedExportProfile,
};
use fc_model::MasterId;

use super::{api_call, open_document, preflight_report, OpenedDocument};
use fc_cli::Error;

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Font or FontCarver document to compile.
    pub(crate) input: PathBuf,
    /// Destination compiled font path; omit it when `--profile` and `--out-dir` derive one.
    pub(crate) output: Option<PathBuf>,
    /// Stored export profile to use, by name, exactly as the export dialog saved it.
    #[arg(long, value_name = "NAME")]
    pub(crate) profile: Option<String>,
    /// Stored variable-font configuration to use, by name.
    #[arg(long, value_name = "NAME", requires = "profile")]
    pub(crate) variable_config: Option<String>,
    /// Named static instances to build: `all`, or a comma-separated list of names.
    #[arg(long, value_name = "SPEC", requires = "profile")]
    pub(crate) instances: Option<String>,
    /// Destination directory for profile-derived file names.
    #[arg(long, value_name = "DIR", requires = "profile")]
    pub(crate) out_dir: Option<PathBuf>,
    /// Print the document's preflight report before writing anything.
    #[arg(long)]
    pub(crate) preflight: bool,
    /// Stop before writing when preflight reports an error.
    #[arg(long, requires = "preflight")]
    pub(crate) fail_on_preflight: bool,
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
    #[arg(long, conflicts_with_all = ["master", "instance_name"])]
    pub(crate) instance: Option<usize>,
    /// Export a named static instance by name.
    #[arg(long, value_name = "NAME", conflicts_with_all = ["master", "variable"])]
    pub(crate) instance_name: Option<String>,
    /// Select variable source masters by name or identifier (repeatable).
    #[arg(long, requires = "variable", value_name = "NAME")]
    pub(crate) source_master: Vec<String>,
    /// Master to compile, by name or identifier.
    #[arg(long, value_name = "NAME")]
    pub(crate) master: Option<String>,
}

/// Flags that describe an ad-hoc export and may not be mixed with a profile.
fn ad_hoc_flags(args: &Args) -> Vec<&'static str> {
    [
        ("--remove-overlap", args.remove_overlap),
        ("--add-extrema", args.add_extrema),
        ("--round-coordinates", args.round_coordinates),
        ("--decompose-components", args.decompose_components),
        ("--production-names", args.production_names),
        ("--subroutinize", args.subroutinize),
        ("--autohint", args.autohint),
        ("--no-preserve", args.no_preserve),
        ("--variable", args.variable),
        ("--instance", args.instance.is_some()),
        ("--instance-name", args.instance_name.is_some()),
        ("--source-master", !args.source_master.is_empty()),
        ("--master", args.master.is_some()),
    ]
    .into_iter()
    .filter_map(|(name, present)| present.then_some(name))
    .collect()
}

pub(crate) fn run(args: Args) -> Result<(), Error> {
    let opened = open_document(&args.input, None)?;
    if args.preflight {
        let errors = preflight_report(&opened)?;
        if errors && args.fail_on_preflight {
            return Err(Error::ProblemsFound);
        }
    }
    match &args.profile {
        Some(profile) => run_profile(&args, &opened, profile),
        None => run_ad_hoc(&args, &opened),
    }
}

/// Builds every requested output from one stored profile.
fn run_profile(args: &Args, opened: &OpenedDocument, profile: &str) -> Result<(), Error> {
    let conflicting = ad_hoc_flags(args);
    if !conflicting.is_empty() {
        return Err(Error::ProfileConflict {
            flags: conflicting.join(", "),
        });
    }
    if args.instances.is_some() && args.variable_config.is_some() {
        return Err(Error::ProfileConflict {
            flags: "--instances, --variable-config".to_owned(),
        });
    }
    let instances = match &args.instances {
        None => vec![None],
        Some(spec) => named_instances(opened, spec)?
            .into_iter()
            .map(Some)
            .collect(),
    };
    if instances.len() > 1 && args.output.is_some() {
        return Err(Error::ProfileConflict {
            flags: "--instances with an explicit output path".to_owned(),
        });
    }
    let mut failures = 0usize;
    for instance in instances {
        let resolved: ResolvedExportProfile = api_call(
            &opened.api,
            "export.resolve_profile",
            ResolveExportProfileParams {
                doc: opened.doc,
                profile: profile.to_owned(),
                variable_config: args.variable_config.clone(),
                instance: instance.clone(),
            },
        )?;
        let path = match (&args.output, &args.out_dir) {
            (Some(output), _) => output.clone(),
            (None, Some(directory)) => directory.join(&resolved.relative_path),
            (None, None) => return Err(Error::MissingDestination),
        };
        match export(opened, &path, resolved.options) {
            Ok(report) => print_report(&report),
            Err(error) => {
                failures += 1;
                eprintln!(
                    "error: {}{}: {error}",
                    path.display(),
                    instance
                        .as_deref()
                        .map(|name| format!(" ({name})"))
                        .unwrap_or_default()
                );
            }
        }
    }
    if failures > 0 {
        return Err(Error::ExportsFailed { count: failures });
    }
    Ok(())
}

/// Resolves `all` or a comma-separated list against the document's instances.
fn named_instances(opened: &OpenedDocument, spec: &str) -> Result<Vec<String>, Error> {
    let stored: fc_api::VariationState = api_call(
        &opened.api,
        "variation.state",
        serde_json::json!({ "doc": opened.doc }),
    )?;
    let available: Vec<String> = stored
        .instances
        .iter()
        .filter(|instance| instance.active)
        .map(|instance| instance.name.clone())
        .collect();
    if spec.trim() == "all" {
        if available.is_empty() {
            return Err(Error::NoInstances);
        }
        return Ok(available);
    }
    let mut selected = Vec::new();
    for name in spec
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        if !available.iter().any(|candidate| candidate == name) {
            return Err(Error::UnknownInstance {
                name: name.to_owned(),
            });
        }
        if !selected.iter().any(|candidate: &String| candidate == name) {
            selected.push(name.to_owned());
        }
    }
    if selected.is_empty() {
        return Err(Error::NoInstances);
    }
    Ok(selected)
}

/// Builds one output from explicit command-line options.
fn run_ad_hoc(args: &Args, opened: &OpenedDocument) -> Result<(), Error> {
    let output = args.output.as_ref().ok_or(Error::MissingDestination)?;
    let format = binary_format(output)?;
    let instance = match &args.instance_name {
        Some(name) => Some(instance_index(opened, name)?),
        None => args.instance,
    };
    let options = ExportOptions {
        format,
        variable: args.variable,
        instance,
        master_subset: (!args.source_master.is_empty())
            .then(|| {
                args.source_master
                    .iter()
                    .map(|name| {
                        master_id(opened, Some(name))?
                            .ok_or_else(|| Error::UnknownMaster { name: name.clone() })
                    })
                    .collect::<Result<Vec<_>, Error>>()
            })
            .transpose()?,
        master: master_id(opened, args.master.as_deref())?,
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
    let report = export(opened, output, options)?;
    print_report(&report);
    Ok(())
}

/// Resolves an instance name against the open document.
fn instance_index(opened: &OpenedDocument, name: &str) -> Result<usize, Error> {
    let stored: fc_api::VariationState = api_call(
        &opened.api,
        "variation.state",
        serde_json::json!({ "doc": opened.doc }),
    )?;
    stored
        .instances
        .iter()
        .position(|instance| instance.name == name)
        .ok_or_else(|| Error::UnknownInstance {
            name: name.to_owned(),
        })
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

/// Reads the export configurations stored in the open document.
pub(crate) fn stored_profiles(opened: &OpenedDocument) -> Result<ExportProfilesResult, Error> {
    api_call(
        &opened.api,
        "export.profiles_get",
        serde_json::json!({ "doc": opened.doc }),
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
