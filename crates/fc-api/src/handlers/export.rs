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
    Api, ApiError, DocParams, ExportFormat, ExportOptions, ExportParams, ExportProfile,
    ExportProfilesResult, ExportReport, ExportWarning, ResolveExportProfileParams,
    ResolvedExportProfile, SetExportProfilesParams, VariableExportConfig, WRITE_STAGE,
};

/// Namespaced collection holding the project's export profiles.
const PROFILE_COLLECTION: &str = "export_profiles";

/// Namespaced collection holding the project's variable-font configurations.
const VARIABLE_COLLECTION: &str = "export_variable_configs";

/// Upper bound shared by the profile picker and the instance/profile matrix.
const MAX_PROFILES: usize = 24;

/// Upper bound on stored variable-font configurations.
const MAX_VARIABLE_CONFIGS: usize = 24;

/// Longest accepted profile name, configuration name, subfolder or file name.
const MAX_NAME_CHARS: usize = 80;

/// Longest accepted destination component derived from a stored configuration.
const MAX_PATH_CHARS: usize = 200;

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
    for (name, present) in [
        ("master_subset", options.master_subset.is_some()),
        ("axis_subset", options.axis_subset.is_some()),
        ("instance_subset", options.instance_subset.is_some()),
    ] {
        if present && (!compiled || !options.variable) {
            return Err(ApiError::invalid_params(format!(
                "{name} requires variable compiled output"
            )));
        }
    }
    if let Some(axes) = &options.axis_subset {
        if axes.is_empty() {
            return Err(ApiError::invalid_params(
                "axis_subset must name at least one axis",
            ));
        }
        for axis in axes {
            axis.parse::<Tag>()
                .map_err(|error| ApiError::invalid_params(error.to_string()))?;
        }
    }
    if options
        .instance_subset
        .as_ref()
        .is_some_and(|names| names.iter().any(|name| name.trim().is_empty()))
    {
        return Err(ApiError::invalid_params(
            "instance_subset cannot contain an empty instance name",
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
        axis_subset: match &options.axis_subset {
            Some(axes) => Some(
                axes.iter()
                    .map(|axis| {
                        axis.parse::<Tag>()
                            .map_err(|error| ApiError::invalid_params(error.to_string()))
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            None => None,
        },
        instance_subset: options.instance_subset.clone(),
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
        tt_autohint: options.tt_autohint.resolve()?,
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

/// Reads the export configurations stored in one document.
pub(crate) fn profiles_get(api: &Api, params: DocParams) -> Result<ExportProfilesResult, ApiError> {
    api.session
        .with_doc(params.doc, |document| {
            read_configurations(&document.font.info.user_data)
        })
        .map_err(ApiError::from)?
}

/// Replaces the export configurations of one document as a single undo entry.
pub(crate) fn profiles_set(
    api: &Api,
    params: SetExportProfilesParams,
) -> Result<ExportProfilesResult, ApiError> {
    let result = validate_configurations(ExportProfilesResult {
        profiles: params.profiles,
        variable_configs: params.variable_configs,
    })?;
    let profiles = serde_json::to_value(&result.profiles)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let configs = serde_json::to_value(&result.variable_configs)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    api.session
        .with_doc_mut(params.doc, |document| {
            // An unreadable stored collection must never be replaced silently.
            read_configurations(&document.font.info.user_data)?;
            super::inventory::write_collection(
                document,
                PROFILE_COLLECTION,
                profiles,
                "Edit export profiles",
            )?;
            super::inventory::write_collection(
                document,
                VARIABLE_COLLECTION,
                configs,
                "Edit export profiles",
            )?;
            Ok(result)
        })
        .map_err(ApiError::from)?
}

/// Resolves one stored configuration into concrete export options and a name.
pub(crate) fn resolve_profile(
    api: &Api,
    params: ResolveExportProfileParams,
) -> Result<ResolvedExportProfile, ApiError> {
    api.session
        .with_doc(params.doc, |document| {
            let stored = read_configurations(&document.font.info.user_data)?;
            resolve(&stored, &document.font, &params)
        })
        .map_err(ApiError::from)?
}

fn resolve(
    stored: &ExportProfilesResult,
    font: &Font,
    params: &ResolveExportProfileParams,
) -> Result<ResolvedExportProfile, ApiError> {
    let profile = stored
        .profiles
        .iter()
        .find(|profile| same_name(&profile.name, &params.profile))
        .ok_or_else(|| {
            ApiError::not_found(format!(
                "no export profile named `{}` is stored in this document",
                params.profile
            ))
        })?;
    let mut options = profile.options.clone();
    let mut base = profile.file_name.clone();
    // A profile stores one file name, but a matrix runs the profile once per
    // row. Without the row's own name every row of the run would resolve to the
    // same path and the last one would win.
    let stored_name = std::mem::take(&mut base);
    /// Combines the profile's stored file name with the row's own name.
    ///
    /// Empty means "no stored name": the caller then falls back to the
    /// family-derived stem, which already carries the row name.
    fn row_stem(stored: &str, row: &str) -> String {
        if stored.is_empty() {
            return String::new();
        }
        let row = sanitize(row);
        if row.is_empty() {
            stored.to_owned()
        } else {
            format!("{stored}-{row}")
        }
    }
    if params.variable_config.is_some() && params.instance.is_some() {
        return Err(ApiError::invalid_params(
            "choose either a variable-font configuration or a named instance",
        ));
    }
    if let Some(name) = &params.variable_config {
        let config = stored
            .variable_configs
            .iter()
            .find(|config| same_name(&config.name, name))
            .ok_or_else(|| {
                ApiError::not_found(format!(
                    "no variable-font configuration named `{name}` is stored in this document"
                ))
            })?;
        for master in &config.masters {
            if font.master(master).is_none() {
                return Err(ApiError::not_found(format!(
                    "variable-font configuration `{name}` names master `{master}`, which no longer exists"
                )));
            }
        }
        for axis in &config.axes {
            if !font.axes.iter().any(|value| value.tag.to_string() == *axis) {
                return Err(ApiError::not_found(format!(
                    "variable-font configuration `{name}` names axis `{axis}`, which no longer exists"
                )));
            }
        }
        for instance in &config.instances {
            if !font.instances.iter().any(|value| value.name == *instance) {
                return Err(ApiError::not_found(format!(
                    "variable-font configuration `{name}` names instance `{instance}`, which no longer exists"
                )));
            }
        }
        options.variable = true;
        options.instance = None;
        options.master_subset = (!config.masters.is_empty()).then(|| config.masters.clone());
        options.axis_subset = (!config.axes.is_empty()).then(|| config.axes.clone());
        options.instance_subset = (!config.instances.is_empty()).then(|| config.instances.clone());
        // A configuration's own file name is already per-row, so it wins.
        base = if !config.file_name.is_empty() {
            config.file_name.clone()
        } else {
            row_stem(&stored_name, &config.name)
        };
        if base.is_empty() {
            base = format!("{}-{}", family_stem(font), sanitize(&config.name));
        }
    } else if let Some(name) = &params.instance {
        let index = font
            .instances
            .iter()
            .position(|instance| instance.name == *name)
            .ok_or_else(|| {
                ApiError::not_found(format!(
                    "no named instance `{name}` exists in this document"
                ))
            })?;
        options.variable = false;
        options.master_subset = None;
        options.axis_subset = None;
        options.instance_subset = None;
        options.instance = Some(index);
        base = row_stem(&stored_name, name);
        if base.is_empty() {
            base = format!("{}-{}", family_stem(font), sanitize(name));
        }
    } else {
        base = stored_name;
        if base.is_empty() {
            base = format!("{}-{}", family_stem(font), sanitize(&profile.name));
        }
    }
    validate_options(&options)?;
    let relative_path = if profile.subfolder.is_empty() {
        format!("{base}{}", options.format.extension())
    } else {
        format!(
            "{}/{base}{}",
            profile.subfolder.trim_end_matches('/'),
            options.format.extension()
        )
    };
    Ok(ResolvedExportProfile {
        options,
        relative_path,
    })
}

/// A family-derived destination stem, always non-empty.
fn family_stem(font: &Font) -> String {
    let stem = sanitize(&font.info.family_name);
    if stem.is_empty() {
        "Untitled".to_owned()
    } else {
        stem
    }
}

/// Keeps letters, digits, dot, underscore and hyphen; collapses everything else.
fn sanitize(name: &str) -> String {
    let mut result = String::new();
    for character in name.chars() {
        if character.is_alphanumeric() || matches!(character, '.' | '_' | '-') {
            result.push(character);
        } else if !result.ends_with('-') {
            result.push('-');
        }
    }
    result
        .trim_matches(|c| matches!(c, '.' | '_' | '-'))
        .to_owned()
}

/// Compares stored names the way the pickers do: NFC, case-insensitive, trimmed.
fn same_name(left: &str, right: &str) -> bool {
    normalized(left) == normalized(right)
}

fn normalized(name: &str) -> String {
    name.trim().to_lowercase()
}

fn read_configurations(user_data: &serde_json::Value) -> Result<ExportProfilesResult, ApiError> {
    let profiles = match super::inventory::read_collection(user_data, PROFILE_COLLECTION)? {
        Some(value) => {
            <Vec<ExportProfile> as serde::Deserialize>::deserialize(value).map_err(|error| {
                ApiError::invalid_params(format!("invalid stored export profiles: {error}"))
            })?
        }
        None => Vec::new(),
    };
    let variable_configs = match super::inventory::read_collection(user_data, VARIABLE_COLLECTION)?
    {
        Some(value) => <Vec<VariableExportConfig> as serde::Deserialize>::deserialize(value)
            .map_err(|error| {
                ApiError::invalid_params(format!(
                    "invalid stored variable-font configurations: {error}"
                ))
            })?,
        None => Vec::new(),
    };
    validate_configurations(ExportProfilesResult {
        profiles,
        variable_configs,
    })
}

fn validate_configurations(
    mut stored: ExportProfilesResult,
) -> Result<ExportProfilesResult, ApiError> {
    if stored.profiles.len() > MAX_PROFILES {
        return Err(ApiError::invalid_params(format!(
            "at most {MAX_PROFILES} export profiles may be stored"
        )));
    }
    if stored.variable_configs.len() > MAX_VARIABLE_CONFIGS {
        return Err(ApiError::invalid_params(format!(
            "at most {MAX_VARIABLE_CONFIGS} variable-font configurations may be stored"
        )));
    }
    let mut names = std::collections::HashSet::new();
    for profile in &mut stored.profiles {
        profile.name = profile.name.trim().to_owned();
        profile.subfolder = profile
            .subfolder
            .trim()
            .replace('\\', "/")
            .trim_end_matches('/')
            .to_owned();
        profile.file_name = profile.file_name.trim().to_owned();
        check_name(&profile.name, "export profile")?;
        if !names.insert(normalized(&profile.name)) {
            return Err(ApiError::invalid_params(format!(
                "two export profiles are named `{}`",
                profile.name
            )));
        }
        check_subfolder(&profile.subfolder)?;
        check_file_name(&profile.file_name)?;
        validate_options(&profile.options)?;
    }
    let mut config_names = std::collections::HashSet::new();
    for config in &mut stored.variable_configs {
        config.name = config.name.trim().to_owned();
        config.file_name = config.file_name.trim().to_owned();
        check_name(&config.name, "variable-font configuration")?;
        if !config_names.insert(normalized(&config.name)) {
            return Err(ApiError::invalid_params(format!(
                "two variable-font configurations are named `{}`",
                config.name
            )));
        }
        check_file_name(&config.file_name)?;
        if config.masters.len() > 1024 || config.axes.len() > 16 || config.instances.len() > 4096 {
            return Err(ApiError::invalid_params(
                "a variable-font configuration exceeds the design-space limits",
            ));
        }
        check_unique(config.masters.iter().map(MasterId::to_string), "master")?;
        check_unique(config.axes.iter().cloned(), "axis")?;
        check_unique(config.instances.iter().cloned(), "instance")?;
        for axis in &config.axes {
            axis.parse::<Tag>()
                .map_err(|error| ApiError::invalid_params(error.to_string()))?;
        }
        for instance in &config.instances {
            if instance.trim().is_empty() || instance.chars().count() > MAX_PATH_CHARS {
                return Err(ApiError::invalid_params(
                    "a variable-font configuration names an empty or overlong instance",
                ));
            }
        }
    }
    Ok(stored)
}

fn check_unique(values: impl Iterator<Item = String>, what: &str) -> Result<(), ApiError> {
    let mut seen = std::collections::HashSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(ApiError::invalid_params(format!(
                "a variable-font configuration repeats a {what}"
            )));
        }
    }
    Ok(())
}

fn check_name(name: &str, what: &str) -> Result<(), ApiError> {
    if name.is_empty() {
        return Err(ApiError::invalid_params(format!("an {what} needs a name")));
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(ApiError::invalid_params(format!(
            "an {what} name is limited to {MAX_NAME_CHARS} characters"
        )));
    }
    if name.chars().any(char::is_control) {
        return Err(ApiError::invalid_params(format!(
            "an {what} name cannot contain control characters"
        )));
    }
    Ok(())
}

fn check_subfolder(subfolder: &str) -> Result<(), ApiError> {
    if subfolder.is_empty() {
        return Ok(());
    }
    if subfolder.chars().count() > MAX_PATH_CHARS
        || subfolder.starts_with('/')
        || subfolder.contains(':')
        || subfolder.chars().any(char::is_control)
        || subfolder
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(ApiError::invalid_params(
            "an export profile subfolder must be a relative path without `.` or `..` components",
        ));
    }
    Ok(())
}

fn check_file_name(file_name: &str) -> Result<(), ApiError> {
    if file_name.is_empty() {
        return Ok(());
    }
    if file_name.chars().count() > MAX_PATH_CHARS
        || file_name.contains('/')
        || file_name.contains('\\')
        || file_name.contains(':')
        || file_name.chars().any(char::is_control)
        || file_name.starts_with('.')
    {
        return Err(ApiError::invalid_params(
            "an export file name must be a plain base name without a path or leading dot",
        ));
    }
    Ok(())
}
