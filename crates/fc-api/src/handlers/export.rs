//! Export handlers shared by the export dialog and the `fontcarver` command line.

use std::{
    collections::HashMap,
    fs,
    io::{Read, Write},
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
    validate_options(&params.options)?;
    if params.collection_docs.len() > 255 {
        return Err(ApiError::invalid_params(
            "collection_docs exceeds the 256-face limit",
        ));
    }
    if !params.collection_docs.is_empty() && params.options.format != ExportFormat::Ttc {
        return Err(ApiError::invalid_params(
            "collection_docs requires Ttc output",
        ));
    }
    let path = PathBuf::from(&params.path);
    let (mut font, generations) = snapshot(
        api,
        params.doc,
        matches!(
            params.options.format,
            ExportFormat::Ufo | ExportFormat::Designspace
        ),
    )?;
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
    let svg_authored = font.svg.is_some();
    if params.options.format == ExportFormat::Otb {
        if font.bitmap_strikes.is_empty() {
            return Err(ApiError::invalid_params(
                "OTB requires at least one bitmap strike",
            ));
        }
        // OTB is bitmap-only; source outlines and layout remain in the native document.
        font.extra = Default::default();
        font.features = Default::default();
        font.svg = None;
        font.svg_table_owned = true;
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
    if matches!(
        params.options.format,
        ExportFormat::Svg | ExportFormat::Eps | ExportFormat::Afm | ExportFormat::Pfm
    ) {
        warnings.push(ExportWarning {
            code: "interchange-source-loss".into(),
            message: match params.options.format {
                ExportFormat::Afm | ExportFormat::Pfm => "Metrics output stores supported widths and kerning without glyph outlines, hints or OpenType layout; retain the native document.",
                ExportFormat::Eps => "EPS output stores one glyph's outlines without the remaining font, hints or layout; retain the native document.",
                _ => "SVG font output stores supported outlines and metrics without OpenType layout, hints or the complete source model; retain the native document.",
            }.into(),
        });
    }
    if svg_authored && !carries_math(params.options.format) {
        warnings.push(ExportWarning { code: "svg-not-exported".into(), message: "This format does not store editable SVG documents; keep the native document or export an OpenType font to retain them.".into() });
    }
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
        | ExportFormat::Sbix
        | ExportFormat::Ttx
        | ExportFormat::Ttc
        | ExportFormat::Cff
        | ExportFormat::Type42 => {
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
            let bytes = match params.options.format {
                ExportFormat::Ttx => fc_sfnt::ttx::encode(&report.bytes, 0)?.into_bytes(),
                ExportFormat::Cff => {
                    let face = read_fonts::FontRef::new(&report.bytes)
                        .map_err(|error| ApiError::parse(error.to_string()))?;
                    let cff = face
                        .table_data(read_fonts::types::Tag::new(b"CFF "))
                        .ok_or_else(|| ApiError::internal("compiled font has no CFF table"))?;
                    warnings.push(ExportWarning { code: "cff-container-loss".into(), message: "Bare CFF stores outlines and PostScript data without OpenType layout, Unicode mapping or other sfnt tables; retain the native document or OTF export for these tables.".into() });
                    tables.retain(|(tag, _)| tag == "CFF ");
                    cff.as_bytes().to_vec()
                }
                ExportFormat::Type42 => fc_formats::type42::write_bytes(&report.bytes)?,
                ExportFormat::Ttc => {
                    let mut faces = vec![report.bytes];
                    for doc in &params.collection_docs {
                        let (font, generations) = snapshot(api, *doc, false)?;
                        let selected_master = params
                            .options
                            .master
                            .as_ref()
                            .unwrap_or(&font.default_master().id);
                        let face_options = compile_options(&params.options, selected_master)?;
                        let face = fc_compile::export(
                            &font,
                            &generation_lookup(&generations),
                            &face_options,
                        )?;
                        warnings.extend(face.warnings.into_iter().map(|warning| ExportWarning {
                            code: warning.code.into(),
                            message: format!("{}: {}", doc.0, warning.message),
                        }));
                        faces.push(face.bytes);
                    }
                    fc_sfnt::collection::encode(
                        &faces.iter().map(Vec::as_slice).collect::<Vec<_>>(),
                    )?
                }
                _ => report.bytes,
            };
            write_file(&path, &bytes)?;
        }
        ExportFormat::Svg => {
            api.emit_progress(params.doc, WRITE_STAGE, 0);
            fc_formats::vector::write_svg(&font, &master, &path)?;
        }
        ExportFormat::Eps => {
            api.emit_progress(params.doc, WRITE_STAGE, 0);
            let glyph = params
                .options
                .glyph
                .as_ref()
                .ok_or_else(|| ApiError::invalid_params("EPS export requires glyph"))?;
            fc_formats::vector::write_eps(&font, &master, glyph, &path)?;
        }
        ExportFormat::Afm => {
            api.emit_progress(params.doc, WRITE_STAGE, 0);
            fc_formats::metrics::write_afm_font(&font, &master, &path)?;
        }
        ExportFormat::Pfm => {
            api.emit_progress(params.doc, WRITE_STAGE, 0);
            fc_formats::metrics::pfm::write_pfm_font(&font, &master, &path)?;
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
            if params.options.add_extrema
                || params.options.round_coordinates
                || params.options.decompose_components
            {
                let geometry_options = compile_options(&params.options, &master)?;
                font = fc_compile::prepare_geometry_export(&font, &master, &geometry_options)?;
                warnings.push(ExportWarning { code: "geometry-preprocessed".into(), message: "Geometry preprocessing was applied to the export copy; source contours and components remain editable. Existing point-bound hints were cleared on modified layers.".into() });
            }
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

fn validate_options(options: &ExportOptions) -> Result<(), ApiError> {
    let sfnt = matches!(
        options.format,
        ExportFormat::Ttf
            | ExportFormat::Otf
            | ExportFormat::Woff
            | ExportFormat::Woff2
            | ExportFormat::Ttx
            | ExportFormat::Ttc
            | ExportFormat::Cff
            | ExportFormat::Type42
            | ExportFormat::Otb
            | ExportFormat::Cbdt
            | ExportFormat::Sbix
    );
    let compiled = sfnt
        && !matches!(
            options.format,
            ExportFormat::Otb
                | ExportFormat::Cbdt
                | ExportFormat::Sbix
                | ExportFormat::Cff
                | ExportFormat::Type42
        );
    let postscript = matches!(options.format, ExportFormat::Pfa | ExportFormat::Pfb);
    if options.subroutinize
        && (!matches!(options.format, ExportFormat::Otf | ExportFormat::Cff) || options.variable)
    {
        return Err(ApiError::invalid_params(
            "subroutinize requires static OTF or bare CFF output",
        ));
    }
    for (name, requested) in [
        ("add_extrema", options.add_extrema),
        ("round_coordinates", options.round_coordinates),
        ("decompose_components", options.decompose_components),
        ("remove_overlap", options.remove_overlap),
        ("autohint", options.autohint),
        ("production_names", options.production_names),
    ] {
        if requested
            && ((!sfnt && !postscript)
                || (options.format == ExportFormat::Otb && name != "production_names"))
        {
            return Err(ApiError::invalid_params(format!(
                "{name} requires a compiled sfnt or Type 1 output"
            )));
        }
    }
    if options.variable && !compiled {
        return Err(ApiError::invalid_params(
            "variable compilation requires TTF, OTF, WOFF, WOFF2, TTX or TTC output",
        ));
    }
    if options.instance.is_some()
        && (!sfnt
            || matches!(
                options.format,
                ExportFormat::Otb | ExportFormat::Cbdt | ExportFormat::Sbix
            ))
    {
        return Err(ApiError::invalid_params(
            "instance compilation requires a compiled outline font output",
        ));
    }
    if options.master_subset.is_some() && (!compiled || !options.variable) {
        return Err(ApiError::invalid_params(
            "master_subset requires variable compiled output",
        ));
    }
    if options.table_subset.is_some() && (!sfnt || options.format == ExportFormat::Cff) {
        return Err(ApiError::invalid_params(
            "table_subset requires compiled sfnt output",
        ));
    }
    if options.bitmap_strike.is_some()
        && !matches!(
            options.format,
            ExportFormat::Bdf
                | ExportFormat::Pcf
                | ExportFormat::Psf
                | ExportFormat::Psf1
                | ExportFormat::Fnt
        )
    {
        return Err(ApiError::invalid_params(
            "bitmap_strike requires a single-strike bitmap output",
        ));
    }
    if options.glyph.is_some() && options.format != ExportFormat::Eps {
        return Err(ApiError::invalid_params(
            "glyph selection requires EPS output",
        ));
    }
    if options.format == ExportFormat::Eps && options.glyph.is_none() {
        return Err(ApiError::invalid_params("EPS export requires glyph"));
    }
    Ok(())
}

/// Whether an output format stores the editable MATH table.
fn carries_math(format: ExportFormat) -> bool {
    matches!(
        format,
        ExportFormat::Ttf
            | ExportFormat::Otf
            | ExportFormat::Woff
            | ExportFormat::Woff2
            | ExportFormat::Ttx
            | ExportFormat::Ttc
            | ExportFormat::Type42
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
fn snapshot(
    api: &Api,
    doc: fc_doc::DocId,
    embed_images: bool,
) -> Result<(Font, Generations), ApiError> {
    let (mut font, generations, document_path) = api
        .session
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
            (document.font.clone(), generations, document.path.clone())
        })
        .map_err(ApiError::from)?;
    if embed_images {
        let mut hydrated_bytes = 0usize;
        for image in font
            .glyphs
            .values_mut()
            .flat_map(|glyph| &mut glyph.layers)
            .filter_map(|layer| layer.background_image.as_mut())
            .filter(|image| image.data.is_none())
        {
            let path = fc_doc::image_paths::resolve_image_path(
                document_path.as_deref(),
                &image.file_name,
            )?;
            let mut bytes = Vec::new();
            fs::File::open(&path)
                .and_then(|file| {
                    file.take(fc_raster::source_image::MAX_IMAGE_BYTES as u64 + 1)
                        .read_to_end(&mut bytes)
                })
                .map_err(|error| {
                    ApiError::io(format!(
                        "failed to read linked source image `{}`: {error}",
                        path.display()
                    ))
                })?;
            fc_raster::source_image::image_dimensions(&bytes)
                .map_err(|error| ApiError::invalid_params(error.to_string()))?;
            hydrated_bytes = hydrated_bytes.saturating_add(bytes.len());
            if hydrated_bytes > 128 * 1024 * 1024 {
                return Err(ApiError::invalid_params(
                    "linked source images exceed the 128 MiB export budget",
                ));
            }
            image.data = Some(bytes);
        }
    }
    Ok((font, generations))
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
            ExportFormat::Otf | ExportFormat::Cff => CompileFormat::Otf,
            ExportFormat::Woff => CompileFormat::Woff,
            ExportFormat::Woff2 => CompileFormat::Woff2,
            _ => CompileFormat::Ttf,
        },
        master: Some(master.clone()),
        add_extrema: options.add_extrema,
        round_coordinates: options.round_coordinates,
        decompose_components: options.decompose_components,
        subroutinize: options.subroutinize,
        remove_overlap: options.remove_overlap,
        keep_tt_instructions: options.keep_tt_instructions,
        preserve_untouched_tables: options.preserve_untouched_tables
            && options.format != ExportFormat::Cff,
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
