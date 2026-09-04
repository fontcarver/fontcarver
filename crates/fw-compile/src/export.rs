//! Public export orchestration and report types.

use std::{collections::HashSet, time::Instant};

use fw_model::{Font, GlyphName, MasterId, Tag};
use read_fonts::FontRef;
use write_fonts::FontBuilder;

use crate::{compile_static, preserve_merge, wrap_woff1, wrap_woff2, CompileOptions, Error};

/// A container format supported by the P1 exporter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExportFormat {
    /// A TrueType-flavored OpenType sfnt.
    #[default]
    Ttf,
    /// WOFF 1 containing a TrueType-flavored sfnt.
    Woff,
    /// WOFF 2 containing a TrueType-flavored sfnt.
    Woff2,
    /// FontWorkshop's native document format; serialized by `fw-doc`, not this crate.
    Fwf,
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
    /// Output container format.
    pub format: ExportFormat,
    /// Master to compile; the first master is used when absent.
    pub master: Option<MasterId>,
    /// Request overlap removal. P1 reports that geometry support is unavailable.
    pub remove_overlap: bool,
    /// Preserve TrueType programs when no glyph has been touched.
    pub keep_tt_instructions: bool,
    /// Copy eligible imported raw tables into the generated font.
    pub preserve_untouched_tables: bool,
    /// Use production glyph names where the model supplies them.
    pub production_names: bool,
    /// Request automatic hinting. P1 reports that hinting support is unavailable.
    pub autohint: bool,
    /// Optional allowlist for non-core tables in the output.
    pub table_subset: Option<Vec<Tag>>,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
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
    let started = Instant::now();
    if options.format == ExportFormat::Fwf {
        return Err(Error::UnsupportedFormat("FWF"));
    }
    let master = options
        .master
        .as_ref()
        .unwrap_or_else(|| &font.default_master().id);
    if font.master(master).is_none() {
        return Err(Error::MissingMaster(master.clone()));
    }

    let mut warnings = Vec::new();
    if options.remove_overlap {
        warnings.push(ExportWarning {
            code: "remove-overlap-unavailable",
            message: "Overlap removal is unavailable until fw-geom is connected in Task 5"
                .to_owned(),
        });
    }
    if options.autohint {
        warnings.push(ExportWarning {
            code: "autohint-unavailable",
            message: "Automatic hinting is deferred to P5 and was not applied".to_owned(),
        });
    }
    warnings.extend(crate::metadata::name_warnings(&font.info));
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
    let compiled = compile_static(
        font,
        master,
        &CompileOptions {
            production_names: options.production_names,
        },
    )?;
    progress(ExportStage::Preserve, PRESERVE_PERCENT);
    let touched = touched_since_import(font, glyph_generations);
    let (merged, preservation_warnings) = preserve_merge(&compiled, font, &touched, options)?;
    warnings.extend(preservation_warnings);
    progress(ExportStage::Write, WRITE_PERCENT);
    let sfnt = if let Some(subset) = &options.table_subset {
        subset_tables(&merged, subset)?
    } else {
        merged
    };
    let tables = table_list(&sfnt)?;
    let bytes = match options.format {
        ExportFormat::Ttf => sfnt,
        ExportFormat::Woff => wrap_woff1(&sfnt)?,
        ExportFormat::Woff2 => wrap_woff2(&sfnt)?,
        ExportFormat::Fwf => return Err(Error::UnsupportedFormat("FWF")),
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
    const CORE: [[u8; 4]; 13] = [
        *b"head", *b"hhea", *b"maxp", *b"OS/2", *b"hmtx", *b"cmap", *b"loca", *b"glyf", *b"name",
        *b"post", *b"fpgm", *b"prep", *b"cvt ",
    ];
    let requested: HashSet<_> = requested.iter().copied().collect();
    let face = FontRef::new(sfnt).map_err(|error| Error::parse("sfnt subset", error))?;
    let mut builder = FontBuilder::new();
    for record in face.table_directory().table_records() {
        let tag = Tag(record.tag().to_be_bytes());
        if CORE.contains(&tag.0) || requested.contains(&tag) {
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
