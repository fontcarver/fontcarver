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
    if (params.options.variable || params.options.instance.is_some())
        && !matches!(
            params.options.format,
            ExportFormat::Ttf | ExportFormat::Otf | ExportFormat::Woff | ExportFormat::Woff2
        )
    {
        return Err(ApiError::invalid_params(
            "variable or interpolated instance compilation requires TTF, OTF, WOFF or WOFF2 output",
        ));
    }
    let path = PathBuf::from(&params.path);
    let (mut font, generations) = snapshot(api, params.doc)?;
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
    if matches!(
        params.options.format,
        ExportFormat::Cbdt | ExportFormat::Sbix
    ) {
        if font.bitmap_strikes.is_empty() || font.bitmap_strikes.iter().any(|s| s.depth != 32) {
            return Err(ApiError::invalid_params(
                "CBDT and sbix outputs require RGBA strikes; use TTF or OTB for coverage strikes",
            ));
        }
        for strike in &mut font.bitmap_strikes {
            strike.properties.insert(
                "FONTCARVER_BITMAP_TABLE".into(),
                fc_model::BdfValue::Str(
                    if params.options.format == ExportFormat::Sbix {
                        "sbix"
                    } else {
                        "CBDT"
                    }
                    .into(),
                ),
            );
        }
    }
    let math_authored = font.math.is_some();
    let color_authored = font.color.is_some();
    if params.options.format == ExportFormat::Otb {
        if font.bitmap_strikes.is_empty() {
            return Err(ApiError::invalid_params(
                "OTB requires at least one bitmap strike",
            ));
        }
        // OTB is bitmap-only; source outlines and layout remain in the native document.
        font.extra = Default::default();
        font.features = Default::default();
        font.color = None;
        font.color_tables_owned = true;
        font.math = None;
        font.math_table_owned = true;
        for glyph in font.glyphs.values_mut() {
            for layer in &mut glyph.layers {
                layer.contours.clear();
                layer.components.clear();
            }
        }
    }
    let touched_glyphs =
        u32::try_from(generations.values().filter(|value| **value > 0).count()).unwrap_or(u32::MAX);

    let mut warnings = Vec::new();
    let mut tables = Vec::new();
    if color_authored && !carries_math(params.options.format) {
        warnings.push(ExportWarning {
            code: "color-not-exported".into(),
            message: "This format does not store editable COLR/CPAL data; keep the native document or export an OpenType font to retain it.".into(),
        });
    }
    if math_authored && !carries_math(params.options.format) {
        warnings.push(ExportWarning {
            code: "math-not-exported".into(),
            message: "This format does not store the editable MATH table; keep the native document or export an OpenType font to retain it.".into(),
        });
    }
    match params.options.format {
        ExportFormat::Bdf
        | ExportFormat::Pcf
        | ExportFormat::Psf
        | ExportFormat::Psf1
        | ExportFormat::Fnt
        | ExportFormat::Fon => {
            api.emit_progress(params.doc, WRITE_STAGE, 0);
            let strike = fc_formats::bitmap::select_strike(&font, params.options.bitmap_strike);
            let bytes = match params.options.format {
                ExportFormat::Bdf => fc_formats::bitmap::bdf::encode(&font, strike?)?,
                ExportFormat::Pcf => fc_formats::bitmap::pcf::encode(&font, strike?)?,
                ExportFormat::Psf | ExportFormat::Psf1 => fc_formats::bitmap::psf::encode(
                    &font,
                    strike?,
                    params.options.format == ExportFormat::Psf1,
                )?,
                ExportFormat::Fnt => fc_formats::bitmap::fnt::encode(&font, strike?)?,
                ExportFormat::Fon => fc_formats::bitmap::fnt::encode_fon(&font)?,
                _ => unreachable!(),
            };
            fc_formats::bitmap::write_bytes(&path, &bytes)?;
            warnings.push(ExportWarning { code: "bitmap-source-loss".into(), message: "Standalone bitmap formats retain supported bitmap data (one strike, or all strikes for FON). Keep the native document for outlines, layout and source metadata. PSF uses fixed cells without baseline metadata.".into() });
        }
        ExportFormat::Ttf
        | ExportFormat::Otf
        | ExportFormat::Woff
        | ExportFormat::Woff2
        | ExportFormat::Otb
        | ExportFormat::Cbdt
        | ExportFormat::Sbix => {
            let options = compile_options(&params.options, &master)?;
            let report = fc_compile::export_with_progress(
                &font,
                &generation_lookup(&generations),
                &options,
                &mut |stage, percent| api.emit_progress(params.doc, stage.as_str(), percent),
            )?;
            if params.options.format == ExportFormat::Otb {
                warnings.push(ExportWarning{code:"bitmap-source-loss".into(),message:"OTB contains bitmap strikes; source outlines and layout remain in the native document.".into()});
            }
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
            if params.options.production_names {
                warnings.push(ExportWarning {
                    code: "production-names-unavailable".into(),
                    message: "Type 1 export retains source glyph names; production-name substitution was not applied.".into(),
                });
            }
            if params.options.remove_overlap
                && !params.options.autohint
                && font
                    .glyphs
                    .values()
                    .filter(|g| g.export)
                    .any(|g| g.master_layer(&master).is_some_and(|l| !l.hints.is_empty()))
            {
                warnings.push(ExportWarning {
                    code: "ps-hints-cleared-overlap".into(),
                    message: "Overlap removal clears existing PostScript glyph hints in the exported master. Enable automatic hinting to infer new hints; the source document is unchanged.".into(),
                });
            }
            // Compute losses from the source before preprocessing clears TT programs.
            warnings.extend(
                fc_formats::type1::loss_warnings(&font, &master)
                    .into_iter()
                    .map(|warning| ExportWarning {
                        code: warning.code.into(),
                        message: warning.message,
                    }),
            );
            if params.options.remove_overlap || params.options.autohint {
                font = fc_compile::prepare_postscript_export(
                    &font,
                    &master,
                    params.options.remove_overlap,
                    params.options.autohint,
                )?;
            }
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

/// Whether an output format stores the editable MATH table.
fn carries_math(format: ExportFormat) -> bool {
    matches!(
        format,
        ExportFormat::Ttf
            | ExportFormat::Otf
            | ExportFormat::Woff
            | ExportFormat::Woff2
            | ExportFormat::Cbdt
            | ExportFormat::Sbix
            | ExportFormat::Ufo
            | ExportFormat::Designspace
            | ExportFormat::Fcf
    )
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
        instance: options.instance,
        variable: options.variable,
        master_subset: options.master_subset.clone(),
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
