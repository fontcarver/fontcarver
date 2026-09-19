//! Export handlers shared by the export dialog and the `fontcarver` command line.

use std::{
    collections::HashMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::Instant,
};

use fc_compile::{ExportFormat as CompileFormat, ExportOptions as CompileOptions};
use fc_model::{Font, GlyphName, LayerKind, MasterId, Tag};

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
    check_features_for_export(&font, params.options.format, &master)?;
    let touched_glyphs =
        u32::try_from(generations.values().filter(|value| **value > 0).count()).unwrap_or(u32::MAX);

    let mut warnings = Vec::new();
    let mut tables = Vec::new();
    match params.options.format {
        ExportFormat::Ttf | ExportFormat::Otf | ExportFormat::Woff | ExportFormat::Woff2 => {
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
        ExportFormat::Glyphs => {
            api.emit_progress(params.doc, WRITE_STAGE, 0);
            fc_formats::glyphs::write_glyphs(&font, &path)?;
        }
        ExportFormat::Sfd => {
            api.emit_progress(params.doc, WRITE_STAGE, 0);
            fc_formats::sfd::write_sfd(&font, &master, &path)?;
        }
        ExportFormat::Pfa | ExportFormat::Pfb => {
            api.emit_progress(params.doc, WRITE_STAGE, 0);
            for (requested, code, message) in [
                (params.options.remove_overlap, "remove-overlap-unavailable", "Overlap removal is not applied by the Type 1 exporter."),
                (params.options.autohint, "autohint-unavailable", "Automatic hinting is not applied by the Type 1 exporter."),
                (params.options.production_names, "production-names-unavailable", "Type 1 export retains source glyph names; production-name substitution was not applied."),
            ] {
                if requested {
                    warnings.push(ExportWarning { code: code.into(), message: message.into() });
                }
            }
            warnings.extend(
                fc_formats::type1::loss_warnings(&font, &master)
                    .into_iter()
                    .map(|warning| ExportWarning {
                        code: warning.code.into(),
                        message: warning.message,
                    }),
            );
            fc_formats::type1::write_type1(
                &font,
                &master,
                &path,
                params.options.format == ExportFormat::Pfb,
            )?;
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

fn check_features_for_export(
    font: &Font,
    format: ExportFormat,
    master: &MasterId,
) -> Result<(), ApiError> {
    if format == ExportFormat::Fcf {
        return Ok(());
    }
    if font.has_unapplied_feature_draft() {
        return Err(ApiError {
            code: "unapplied_feature_draft".to_owned(),
            message: "Apply or discard the feature draft before exporting a font.".to_owned(),
        });
    }
    if matches!(
        format,
        ExportFormat::Ufo | ExportFormat::Designspace | ExportFormat::Glyphs | ExportFormat::Sfd
    ) {
        if font.features.source_of_truth == fc_model::FeaturesSource::ImportedTables {
            return Err(ApiError {
                code: "layout_conversion_required".to_owned(),
                message: "Imported layout requires validated conversion before source export."
                    .to_owned(),
            });
        }
        // Static SFD contains only the selected master. A work-in-progress
        // alternate master must not prevent validating that selected output.
        let selected = (format == ExportFormat::Sfd).then(|| {
            let mut selected = font.clone();
            selected.masters.retain(|candidate| &candidate.id == master);
            for glyph in selected.glyphs.values_mut() {
                glyph
                    .layers
                    .retain(|layer| matches!(&layer.kind, LayerKind::Master(id) if id == master));
            }
            selected
        });
        let validation_font = selected.as_ref().unwrap_or(font);
        if let Some(blocker) = fc_doc::problems::compilation_blocker(validation_font) {
            return Err(ApiError {
                code: "invalid_font_geometry".to_owned(),
                message: format!("{}: {}", blocker.code, blocker.message),
            });
        }
        let validation = fc_compile::validate_feature_candidate(
            validation_font,
            &validation_font.features,
            &validation_font.feature_settings,
        );
        if !validation.is_valid() {
            let message = validation
                .diagnostics
                .iter()
                .filter(|diagnostic| {
                    diagnostic.severity == fc_compile::FeatureDiagnosticSeverity::Error
                })
                .map(|diagnostic| diagnostic.message.as_str())
                .collect::<Vec<_>>()
                .join("; ");
            return Err(ApiError {
                code: "invalid_feature_source".to_owned(),
                message: format!("Active feature source cannot compile: {message}"),
            });
        }
    }
    Ok(())
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
            ExportFormat::Otf => CompileFormat::Otf,
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
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .map_err(|error| ApiError::io(format!("failed to stage `{}`: {error}", path.display())))?;
    temporary
        .write_all(bytes)
        .and_then(|()| temporary.as_file().sync_all())
        .map_err(|error| ApiError::io(format!("failed to write `{}`: {error}", path.display())))?;
    temporary.persist(path).map_err(|error| {
        ApiError::io(format!("failed to replace `{}`: {error}", path.display()))
    })?;
    Ok(())
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
