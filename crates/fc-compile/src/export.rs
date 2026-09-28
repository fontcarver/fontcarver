//! Public export orchestration and report types.

use std::{collections::HashSet, time::Instant};

use fc_model::{Font, GlyphName, MasterId, Tag};
use read_fonts::FontRef;
use write_fonts::FontBuilder;

use crate::{
    compile_static_with_metadata, preserve_merge, wrap_woff1, wrap_woff2, CompileOptions, Error,
};

/// A container format supported by the P1 exporter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExportFormat {
    /// A TrueType-flavored OpenType sfnt.
    #[default]
    Ttf,
    /// OpenType sfnt containing static, name-keyed CFF1 outlines.
    Otf,
    /// WOFF 1 containing a TrueType-flavored sfnt.
    Woff,
    /// WOFF 2 containing a TrueType-flavored sfnt.
    Woff2,
    /// FontCarver's native document format; serialized by `fc-doc`, not this crate.
    Fcf,
}

/// Share of an export that is complete when preservation starts.
const PRESERVE_PERCENT: u8 = 60;

/// Share of an export that is complete when the container is encoded.
const WRITE_PERCENT: u8 = 90;

/// A stage of the export pipeline, reported through the progress callback.
///
/// Stages are emitted immediately before the work they name starts, so a
/// consumer that never sees a later stage knows where the export stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExportStage {
    /// Compiling the model into an sfnt.
    Compile,
    /// Merging preserved tables from the imported font.
    Preserve,
    /// Subsetting and encoding the output container.
    Write,
}

impl ExportStage {
    /// Returns the stable identifier used on the API event stream.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Compile => "compile",
            Self::Preserve => "preserve",
            Self::Write => "write",
        }
    }
}

impl std::fmt::Display for ExportStage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Options controlling static export and preservation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportOptions {
    /// Index of a named static instance to interpolate instead of selecting a master.
    pub instance: Option<usize>,
    /// Compile all masters as a variable font (gvar for TTF, CFF2 for OTF).
    pub variable: bool,
    /// Optional source-master subset for variable compilation; includes the default.
    pub master_subset: Option<Vec<MasterId>>,
    /// Output container format.
    pub format: ExportFormat,
    /// Master to compile; the first master is used when absent.
    pub master: Option<MasterId>,
    /// Remove overlaps in a static export copy before compilation and hinting.
    pub remove_overlap: bool,
    /// Preserve TrueType programs when no glyph has been touched.
    pub keep_tt_instructions: bool,
    /// Copy eligible imported raw tables into the generated font.
    pub preserve_untouched_tables: bool,
    /// Use production glyph names where the model supplies them.
    pub production_names: bool,
    /// Infer CFF hints or run the optional external TrueType hinter.
    pub autohint: bool,
    /// Optional allowlist for non-core tables in the output.
    pub table_subset: Option<Vec<Tag>>,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            instance: None,
            variable: false,
            master_subset: None,
            format: ExportFormat::Ttf,
            master: None,
            remove_overlap: false,
            keep_tt_instructions: true,
            preserve_untouched_tables: true,
            production_names: false,
            autohint: false,
            table_subset: None,
        }
    }
}

/// A non-fatal issue encountered while exporting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportWarning {
    /// Stable machine-readable warning identifier.
    pub code: &'static str,
    /// Human-readable warning detail.
    pub message: String,
}

/// Exported bytes together with diagnostics and timing data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportReport {
    /// Encoded font bytes in the requested format.
    pub bytes: Vec<u8>,
    /// Non-fatal export diagnostics.
    pub warnings: Vec<ExportWarning>,
    /// Logical sfnt table tags and uncompressed byte lengths.
    pub tables: Vec<(Tag, u32)>,
    /// Total wall-clock time spent exporting.
    pub timing_ms: u64,
}

/// Returns the glyphs whose caller-provided generation is newer than import.
///
/// P1 defines import generation as zero. Task 5 supplies document-aware values.
pub fn touched_since_import(
    font: &Font,
    glyph_generations: &dyn Fn(&GlyphName) -> u64,
) -> HashSet<GlyphName> {
    font.glyph_order()
        .filter(|name| glyph_generations(name) > 0)
        .cloned()
        .collect()
}

/// Compiles, preserves, subsets, and wraps a static font.
///
/// # Errors
///
/// Returns an error when the master is missing, when compilation fails, or when
/// the output container cannot be encoded.
pub fn export(
    font: &Font,
    glyph_generations: &dyn Fn(&GlyphName) -> u64,
    options: &ExportOptions,
) -> Result<ExportReport, Error> {
    export_with_progress(font, glyph_generations, options, &mut |_, _| {})
}

/// Exports `font`, reporting each pipeline stage before it starts.
///
/// `progress` is called with the stage and the percentage of the whole export
/// that is complete when the stage begins. A failure stops the sequence at the
/// stage that failed; callers that drive a progress bar must emit their own
/// terminal event on the error path.
///
/// # Errors
///
/// Returns an error when the master is missing, when compilation fails, or when
/// the output container cannot be encoded.
pub fn export_with_progress(
    font: &Font,
    glyph_generations: &dyn Fn(&GlyphName) -> u64,
    options: &ExportOptions,
    progress: &mut dyn FnMut(ExportStage, u8),
) -> Result<ExportReport, Error> {
    let mut report = export_inner(font, glyph_generations, options, progress, false)?;
    // Inspect the original source before instance/smart/overlap preprocessing
    // can discard TT metadata from the temporary CFF compilation copy.
    if options.format == ExportFormat::Otf && crate::tt::selected_authored(font, options) {
        report.warnings.push(ExportWarning {
            code: "tt-authored-programs-omitted",
            message: "Authored TrueType programs remain in the source document but cannot be written to CFF outlines".into(),
        });
    }
    Ok(report)
}

fn export_inner(
    font: &Font,
    glyph_generations: &dyn Fn(&GlyphName) -> u64,
    options: &ExportOptions,
    progress: &mut dyn FnMut(ExportStage, u8),
    outline_changed: bool,
) -> Result<ExportReport, Error> {
    let started = Instant::now();
    if options.format == ExportFormat::Fcf {
        return Err(Error::UnsupportedFormat("FCF"));
    }
    if font.has_unapplied_feature_draft() {
        return Err(Error::InvalidSource(
            "apply or discard the feature draft before exporting a font".to_owned(),
        ));
    }
    crate::tt::preflight(font, options)?;
    if font
        .glyphs
        .values()
        .flat_map(|g| &g.layers)
        .any(|l| !l.effects.is_empty() || l.has_smart_corners())
    {
        let (baked, approximated) =
            fc_geom::bake_effects_font(font).map_err(|e| Error::InvalidSource(e.to_string()))?;
        if !font
            .glyphs
            .values()
            .flat_map(|g| &g.layers)
            .any(fc_model::Layer::has_live_geometry)
        {
            return export_inner(
                &baked,
                glyph_generations,
                options,
                progress,
                outline_changed,
            );
        }
        let mut options = options.clone();
        options.keep_tt_instructions = false;
        let mut report = export_inner(&baked, &|_| 1, &options, progress, true)?;
        report.warnings.push(ExportWarning { code: "layer-effects-baked", message: "Live corners and layer effects were evaluated on the export copy; source nodes and stacks remain editable".into() });
        if !approximated.is_empty() {
            report.warnings.push(ExportWarning {
                code: "layer-effects-refitted",
                message: format!(
                    "{} effect layers required polygon curve refitting",
                    approximated.len()
                ),
            });
        }
        return Ok(report);
    }
    if font.glyphs.values().any(|glyph| {
        !glyph.smart_axes.is_empty()
            || glyph
                .layers
                .iter()
                .flat_map(|l| &l.components)
                .any(|c| c.smart.as_ref().is_some_and(|v| !v.is_empty()))
    }) {
        let resolved =
            fc_geom::resolve_smart_font(font).map_err(|e| Error::InvalidSource(e.to_string()))?;
        let mut options = options.clone();
        options.keep_tt_instructions = false;
        return export_inner(&resolved, &|_| 1, &options, progress, outline_changed);
    }
    if let Some(index) = options.instance {
        if options.variable || options.master_subset.is_some() {
            return Err(Error::InvalidSource(
                "choose either a static instance or variable export".into(),
            ));
        }
        let instance = font
            .instances
            .get(index)
            .ok_or_else(|| Error::InvalidSource("export instance does not exist".into()))?;
        let resolved = crate::instantiate(font, &instance.location, Some(instance))?;
        let mut options = options.clone();
        options.instance = None;
        options.master = Some(resolved.default_master().id.clone());
        options.keep_tt_instructions = false;
        return export_inner(&resolved, &|_| 1, &options, progress, outline_changed);
    }
    if !options.variable && options.master_subset.is_some() {
        return Err(Error::InvalidSource(
            "source-master subsets require variable export".into(),
        ));
    }
    if options.variable {
        if let Some(subset) = &options.master_subset {
            let selected = crate::variable::master_subset(font, subset)?;
            let mut options = options.clone();
            options.master_subset = None;
            return export_inner(
                &selected,
                glyph_generations,
                &options,
                progress,
                outline_changed,
            );
        }
        crate::variable::validate_export(font)?;
    }
    let master = if options.variable {
        &font
            .design_default_master()
            .map_err(|e| Error::InvalidSource(e.to_string()))?
            .id
    } else {
        options
            .master
            .as_ref()
            .unwrap_or_else(|| &font.default_master().id)
    };
    if font.master(master).is_none() {
        return Err(Error::MissingMaster(master.clone()));
    }
    if !options.variable
        && font.glyphs.values().any(|glyph| {
            glyph
                .layers
                .iter()
                .any(|layer| matches!(layer.kind, fc_model::LayerKind::Bracket { .. }))
        })
    {
        let source = font
            .master(master)
            .ok_or_else(|| Error::MissingMaster(master.clone()))?;
        let resolved = crate::instantiate(font, &source.location, None)?;
        let mut options = options.clone();
        options.master = Some(resolved.default_master().id.clone());
        options.keep_tt_instructions = false;
        return export_inner(&resolved, &|_| 1, &options, progress, outline_changed);
    }

    if options.remove_overlap {
        if options.variable {
            return Err(Error::InvalidSource("remove overlaps in compatible source masters before variable export, or export a static instance".into()));
        }
        let prepared = crate::preprocess::overlap(font, master)?;
        let mut options = options.clone();
        options.remove_overlap = false;
        options.keep_tt_instructions = false;
        return export_inner(&prepared, &|_| 1, &options, progress, true);
    }
    if options.autohint && options.variable {
        return Err(Error::InvalidSource(
            "automatic hinting currently requires a static master or instance".into(),
        ));
    }
    if options.autohint && options.format == ExportFormat::Otf {
        let prepared = crate::preprocess::ps_autohint(font, master)?;
        let mut options = options.clone();
        options.autohint = false;
        return export_inner(
            &prepared,
            glyph_generations,
            &options,
            progress,
            outline_changed,
        );
    }
    let mut warnings = Vec::new();
    let authored_tt = crate::tt::authored(font, master);
    warnings.extend(crate::metadata::name_warnings(&font.info));
    warnings.extend(crate::horizontal_kerning_warnings(font, master));
    if font
        .glyphs
        .values()
        .any(|glyph| !glyph.variation_selectors.is_empty())
    {
        warnings.push(ExportWarning {
            code: "variation-sequences-dropped",
            message: "Unicode variation sequences are not emitted by the static P1 exporter"
                .to_owned(),
        });
    }
    warnings.extend(value_clamp_warnings(font, master));
    if crate::large_font::requires_workaround(font, options.production_names) {
        warnings.push(ExportWarning {
            code: "post-names-omitted-capacity",
            message: "The font exceeds post format 2 name capacity; a name-less post format 3 table was emitted".to_owned(),
        });
    }

    progress(ExportStage::Compile, 0);
    let prepared_cff = (options.format == ExportFormat::Otf && !options.variable)
        .then(|| crate::otf::prepare(font, master, options.production_names))
        .transpose()?;
    let compile_options = CompileOptions {
        production_names: options.production_names,
    };
    let (compiled, variable_sources) = if options.variable {
        let (compiled, prepared) =
            crate::source::compile_variable_with_sources(font, &compile_options)?;
        (compiled, Some(prepared))
    } else {
        (
            compile_static_with_metadata(font, master, &compile_options)?,
            None,
        )
    };
    let font = variable_sources.as_ref().unwrap_or(font);
    progress(ExportStage::Preserve, PRESERVE_PERCENT);
    let touched = touched_since_import(font, glyph_generations);
    let mut preservation_options = options.clone();
    preservation_options.master = Some(master.clone());
    if options.format == ExportFormat::Otf || options.variable || authored_tt {
        preservation_options.keep_tt_instructions = false;
    }
    let (merged, preservation_warnings) =
        preserve_merge(&compiled.bytes, font, &touched, &preservation_options)?;
    warnings.extend(preservation_warnings);
    let merged = if authored_tt && options.format != ExportFormat::Otf {
        let (bytes, diagnostics) = crate::tt::apply(
            &merged,
            font,
            master,
            &compiled.glyph_order,
            options.keep_tt_instructions,
        )?;
        warnings.extend(diagnostics);
        bytes
    } else {
        merged
    };
    if outline_changed {
        crate::otf_preservation::check_geometry_independent_layout(&merged)
            .and_then(|()| crate::otf_preservation_aat::check(&merged))
            .map_err(|error| {
                Error::InvalidSource(format!(
                    "outline preprocessing cannot preserve this layout: {error}"
                ))
            })?;
    }
    let merged = if options.format == ExportFormat::Otf && options.variable {
        crate::variable_cff::convert(&merged, font, &compiled.glyph_order)?
    } else if let Some(prepared) = prepared_cff {
        crate::otf::convert(
            &merged,
            font,
            master,
            &compiled.glyph_order,
            options.production_names,
            prepared,
        )?
    } else {
        merged
    };
    let merged = if options.autohint {
        match fc_hint::external::autohint(&merged) {
            Ok(bytes) => bytes,
            Err(fc_hint::Error::Unavailable) => {
                warnings.push(ExportWarning {code:"autohint-unavailable",message:"ttfautohint is not installed on PATH; the exported TrueType font was not automatically hinted".into()});
                merged
            }
            Err(error) => return Err(Error::InvalidSource(error.to_string())),
        }
    } else {
        merged
    };
    let merged = fc_sfnt::bitmap::replace_tables(&merged, font, &compiled.glyph_order)
        .map_err(|e| Error::write("embedded bitmaps", e))?;
    let merged = crate::math::apply(&merged, font, &compiled.glyph_order, &mut warnings)?;
    let merged = crate::color::apply(&merged, font, &compiled.glyph_order, &mut warnings)?;
    let merged = crate::svg::apply(&merged, font, &compiled.glyph_order, &mut warnings)?;
    progress(ExportStage::Write, WRITE_PERCENT);
    let sfnt = if let Some(subset) = &options.table_subset {
        subset_tables(&merged, subset)?
    } else {
        merged
    };
    let tables = table_list(&sfnt)?;
    let bytes = match options.format {
        ExportFormat::Ttf | ExportFormat::Otf => sfnt,
        ExportFormat::Woff => wrap_woff1(&sfnt)?,
        ExportFormat::Woff2 => wrap_woff2(&sfnt)?,
        ExportFormat::Fcf => return Err(Error::UnsupportedFormat("FCF")),
    };
    Ok(ExportReport {
        bytes,
        warnings,
        tables,
        timing_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    })
}

fn table_list(sfnt: &[u8]) -> Result<Vec<(Tag, u32)>, Error> {
    let face = FontRef::new(sfnt).map_err(|error| Error::parse("exported sfnt", error))?;
    face.table_directory()
        .table_records()
        .iter()
        .map(|record| {
            let data = face
                .table_data(record.tag())
                .ok_or_else(|| Error::parse("table directory", record.tag()))?;
            Ok((
                Tag(record.tag().to_be_bytes()),
                u32::try_from(data.len()).map_err(|_| Error::Limit("table length"))?,
            ))
        })
        .collect()
}

fn subset_tables(sfnt: &[u8], requested: &[Tag]) -> Result<Vec<u8>, Error> {
    const CORE: [[u8; 4]; 14] = [
        *b"head", *b"hhea", *b"maxp", *b"OS/2", *b"hmtx", *b"cmap", *b"loca", *b"glyf", *b"name",
        *b"post", *b"fpgm", *b"prep", *b"cvt ", *b"CFF ",
    ];
    let mut requested: HashSet<_> = requested.iter().copied().collect();
    if requested.contains(&Tag(*b"COLR")) {
        requested.insert(Tag(*b"CPAL"));
    }
    let face = FontRef::new(sfnt).map_err(|error| Error::parse("sfnt subset", error))?;
    let mut builder = FontBuilder::new();
    for record in face.table_directory().table_records() {
        let tag = Tag(record.tag().to_be_bytes());
        let variable_core = face
            .table_data(read_fonts::types::Tag::new(b"fvar"))
            .is_some()
            && [
                *b"CFF2", *b"fvar", *b"avar", *b"gvar", *b"HVAR", *b"VVAR", *b"MVAR", *b"STAT",
                *b"vhea", *b"vmtx", *b"VORG", *b"GDEF", *b"GPOS", *b"GSUB",
            ]
            .contains(&tag.0);
        if CORE.contains(&tag.0) || variable_core || requested.contains(&tag) {
            let data = face
                .table_data(record.tag())
                .ok_or_else(|| Error::parse("sfnt subset table", record.tag()))?;
            builder.add_raw(record.tag(), data.as_bytes());
        }
    }
    Ok(builder.build())
}

fn value_clamp_warnings(font: &Font, master: &MasterId) -> Vec<ExportWarning> {
    let mut coordinate_count = 0usize;
    let mut horizontal_advance_count = 0usize;
    let mut vertical_advance_count = 0usize;
    for glyph in font.glyphs.values().filter(|glyph| glyph.export) {
        let Some(layer) = glyph.master_layer(master) else {
            continue;
        };
        coordinate_count += layer
            .contours
            .iter()
            .flat_map(|contour| &contour.points)
            .flat_map(|point| [point.x, point.y])
            .filter(|value| outside_i16(*value))
            .count();
        coordinate_count += layer
            .components
            .iter()
            .flat_map(|component| {
                let coefficients = component.transform.as_coeffs();
                [coefficients[4], coefficients[5]]
            })
            .filter(|value| outside_i16(*value))
            .count();
        horizontal_advance_count += usize::from(outside_u16(layer.width));
        vertical_advance_count += usize::from(layer.vertical_width.is_some_and(outside_u16));
    }
    let mut warnings = Vec::new();
    for (table, count, description) in [
        ("glyf", coordinate_count, "outline coordinate values"),
        (
            "hmtx",
            horizontal_advance_count,
            "horizontal advance values",
        ),
        ("vmtx", vertical_advance_count, "vertical advance values"),
    ] {
        if count != 0 {
            warnings.push(ExportWarning {
                code: "value-clamped",
                message: format!("{table}: clamped {count} {description} to the OpenType range"),
            });
        }
    }
    warnings
}

fn outside_i16(value: f64) -> bool {
    !value.is_finite() || value.round() < f64::from(i16::MIN) || value.round() > f64::from(i16::MAX)
}

fn outside_u16(value: f64) -> bool {
    !value.is_finite() || value.round() < 0.0 || value.round() > f64::from(u16::MAX)
}
