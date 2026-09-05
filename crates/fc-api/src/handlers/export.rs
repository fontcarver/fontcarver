//! Export handlers shared by the export dialog and the `fontcutter` command line.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use fc_compile::{ExportFormat as CompileFormat, ExportOptions as CompileOptions};
use fc_model::{Font, GlyphName, MasterId, Tag};

use crate::{
    Api, ApiError, ExportFormat, ExportOptions, ExportParams, ExportReport, ExportWarning,
    WRITE_STAGE,
};

/// Per-glyph invalidation generations captured with the font.
type Generations = HashMap<GlyphName, u64>;

/// Exports one document, always ending the progress stream.
///
/// Every failure after the request is accepted emits a terminal `failed` event
/// with the error message, so a progress bar in the UI never stays stuck at the
/// stage that broke.
pub(crate) fn export(api: &Api, params: ExportParams) -> Result<ExportReport, ApiError> {
    let doc = params.doc;
    run(api, params).inspect_err(|error| api.emit_failure(doc, error.message.clone()))
}

fn run(api: &Api, params: ExportParams) -> Result<ExportReport, ApiError> {
    let started = Instant::now();
    let path = PathBuf::from(&params.path);
    let (font, generations) = snapshot(api, params.doc)?;
    let master = match &params.options.master {
        Some(master) => master.clone(),
        None => font.default_master().id.clone(),
    };
    if font.master(&master).is_none() {
        return Err(ApiError::not_found(format!(
            "master `{master}` does not exist"
        )));
    }
    let touched_glyphs =
        u32::try_from(generations.values().filter(|value| **value > 0).count()).unwrap_or(u32::MAX);

    let mut warnings = Vec::new();
    let mut tables = Vec::new();
    match params.options.format {
        ExportFormat::Ttf | ExportFormat::Woff | ExportFormat::Woff2 => {
            let options = compile_options(&params.options, &master)?;
            let report = fc_compile::export_with_progress(
                &font,
                &generation_lookup(&generations),
                &options,
                &mut |stage, percent| api.emit_progress(params.doc, stage.as_str(), percent),
            )?;
            warnings.extend(report.warnings.iter().map(|warning| ExportWarning {
                code: warning.code.to_owned(),
                message: warning.message.clone(),
            }));
            tables = report
                .tables
                .iter()
                .map(|(tag, length)| (tag.to_string(), *length))
                .collect();
            write_file(&path, &report.bytes)?;
        }
        ExportFormat::Ufo => {
            api.emit_progress(params.doc, WRITE_STAGE, 0);
            fc_formats::ufo::write_ufo(
                &font,
                &master,
                &path,
                &fc_formats::ufo::UfoWriteOptions::default(),
            )?;
        }
        ExportFormat::Designspace => {
            api.emit_progress(params.doc, WRITE_STAGE, 0);
            fc_formats::ufo::write_designspace(&font, &path)?;
        }
        ExportFormat::Fcf => {
            api.emit_progress(params.doc, WRITE_STAGE, 0);
            fc_doc::fcf::write(&path, &font)?;
        }
    }
    api.emit_progress(params.doc, WRITE_STAGE, 100);

    Ok(ExportReport {
        path: params.path,
        format: params.options.format,
        bytes_written: written_bytes(&path),
        warnings,
        tables,
        touched_glyphs,
        timing_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    })
}

/// Copies the font and its per-glyph generations out of the session.
///
/// Exporting must not hold the session lock: compilation of a large font takes
/// seconds and every other request would block behind it.
fn snapshot(api: &Api, doc: fc_doc::DocId) -> Result<(Font, Generations), ApiError> {
    api.session
        .with_doc(doc, |document| {
            let generations = document
                .font
                .glyph_order()
                .map(|name| {
                    let generation = document
                        .glyph_id(name)
                        .map_or(0, |gid| document.glyph_generation(gid));
                    (name.clone(), generation)
                })
                .collect();
            (document.font.clone(), generations)
        })
        .map_err(ApiError::from)
}

/// Reports a glyph as edited since import when its generation moved.
///
/// Documents start every glyph at generation zero when they are opened or
/// imported, so a non-zero generation is exactly "edited in this session".
fn generation_lookup(generations: &Generations) -> impl Fn(&GlyphName) -> u64 + '_ {
    move |name: &GlyphName| generations.get(name).copied().unwrap_or_default()
}

fn compile_options(options: &ExportOptions, master: &MasterId) -> Result<CompileOptions, ApiError> {
    let table_subset = match &options.table_subset {
        Some(tags) => Some(
            tags.iter()
                .map(|tag| {
                    tag.parse::<Tag>()
                        .map_err(|error| ApiError::invalid_params(error.to_string()))
                })
                .collect::<Result<Vec<_>, _>>()?,
        ),
        None => None,
    };
    Ok(CompileOptions {
        format: match options.format {
            ExportFormat::Woff => CompileFormat::Woff,
            ExportFormat::Woff2 => CompileFormat::Woff2,
            _ => CompileFormat::Ttf,
        },
        master: Some(master.clone()),
        remove_overlap: options.remove_overlap,
        keep_tt_instructions: options.keep_tt_instructions,
        preserve_untouched_tables: options.preserve_untouched_tables,
        production_names: options.production_names,
        autohint: options.autohint,
        table_subset,
    })
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), ApiError> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|error| {
            ApiError::io(format!("failed to create `{}`: {error}", parent.display()))
        })?;
    }
    fs::write(path, bytes)
        .map_err(|error| ApiError::io(format!("failed to write `{}`: {error}", path.display())))
}

/// Returns the number of bytes an export wrote, walking package directories.
fn written_bytes(path: &Path) -> u64 {
    let Ok(metadata) = fs::metadata(path) else {
        return 0;
    };
    if metadata.is_file() {
        return metadata.len();
    }
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| written_bytes(&entry.path()))
        .sum()
}
