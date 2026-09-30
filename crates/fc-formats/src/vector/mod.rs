//! SVG 1.1 font sources and bounded single-glyph EPS outline interchange.
//!
//! These are static outline formats, distinct from the sfnt SVG color table.
//! Export omits layout, hinting, anchors, source metadata and other masters.
//! Components, live effects, color outlines and vertical advances must be
//! resolved or removed first. Reads never execute PostScript or resolve XML
//! entities. See each entry point for the supported native subset.

mod eps;
mod svg;
mod svg_path;

use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

use fc_model::{Font, GlyphName, MasterId};
use kurbo::{BezPath, PathEl};

use crate::Error;

const MAX_FILE_BYTES: usize = 32 * 1024 * 1024;
const MAX_ELEMENTS: usize = 1_000_000;

/// Reads one horizontal SVG 1.1 `<font>` containing scalar Unicode mappings.
///
/// Supports `d` outlines using M/L/H/V/Q/C/Z commands, relative variants and
/// repeated coordinates. Rejects kerning, ligatures, contextual selection,
/// vertical metrics, transforms, child graphics and external resources.
pub fn read_svg(path: &Path) -> Result<Font, Error> {
    svg::read(path)
}

/// Writes selected-master closed outlines, family, em size and basic metrics.
///
/// Multiple Unicode assignments become equal named glyph definitions. Layout,
/// hinting, arbitrary source metadata, anchors and other masters are omitted.
/// Complete validation precedes atomic destination replacement.
pub fn write_svg(font: &Font, master: &MasterId, path: &Path) -> Result<(), Error> {
    svg::write(font, master, path)
}

/// Reads bounded declarative EPS paths into one editable glyph named `glyph`.
///
/// FontCarver DSC comments restore glyph name, width and em size when present.
/// Only literal path construction and nonzero fill are supported; PostScript
/// code, transforms, stroke and external resources are rejected, never executed.
pub fn read_eps(path: &Path) -> Result<Font, Error> {
    eps::read(path)
}

/// Writes one glyph as a native EPS filled outline with optional DSC metadata.
///
/// Coordinates remain in y-up font units. This is one glyph, not a font file.
/// Validation and complete serialization precede atomic destination replacement.
pub fn write_eps(
    font: &Font,
    master: &MasterId,
    glyph: &GlyphName,
    path: &Path,
) -> Result<(), Error> {
    eps::write(font, master, glyph, path)
}

fn source_error(format: &'static str, reason: impl Into<String>) -> Error {
    Error::Source {
        format,
        reason: reason.into(),
    }
}

fn read_text(path: &Path, format: &'static str) -> Result<String, Error> {
    let file = File::open(path).map_err(|e| Error::io(path, e))?;
    let mut bytes = Vec::new();
    file.take((MAX_FILE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| Error::io(path, e))?;
    if bytes.len() > MAX_FILE_BYTES {
        return Err(source_error(format, "input exceeds 32 MiB limit"));
    }
    String::from_utf8(bytes).map_err(|_| source_error(format, "input must be UTF-8 text"))
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    if bytes.len() > MAX_FILE_BYTES {
        return Err(source_error("vector", "output exceeds 32 MiB limit"));
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary =
        tempfile::NamedTempFile::new_in(parent).map_err(|e| Error::io(parent, e))?;
    temporary.write_all(bytes).map_err(|e| Error::io(path, e))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|e| Error::io(path, e))?;
    temporary
        .persist(path)
        .map_err(|e| Error::io(path, e.error))?;
    Ok(())
}

fn glyph_path(
    font: &Font,
    master: &MasterId,
    glyph: &GlyphName,
    format: &'static str,
) -> Result<BezPath, Error> {
    if font.has_unapplied_feature_draft() {
        return Err(Error::UnappliedFeatureDraft);
    }
    if font.master(master).is_none() {
        return Err(Error::MissingMaster(master.to_string()));
    }
    let glyph = font
        .glyph(glyph)
        .ok_or_else(|| source_error(format, "glyph does not exist"))?;
    let layer = glyph
        .master_layer(master)
        .ok_or_else(|| source_error(format, "glyph has no selected master layer"))?;
    if !layer.components.is_empty()
        || !layer.effects.is_empty()
        || layer.has_smart_corners()
        || layer
            .color_layers
            .as_ref()
            .is_some_and(|layers| !layers.is_empty())
        || layer.vertical_width.is_some()
        || layer.vertical_origin.is_some()
    {
        return Err(source_error(
            format,
            "components, live effects, color or vertical metrics are unsupported",
        ));
    }
    if !valid_number(layer.width) || layer.width < 0.0 {
        return Err(source_error(format, "invalid advance width"));
    }
    let mut result = BezPath::new();
    let mut points = 0usize;
    for contour in &layer.contours {
        points = points.saturating_add(contour.points.len());
        if points > MAX_ELEMENTS || !contour.closed {
            return Err(source_error(
                format,
                "outlines must be closed and within point limit",
            ));
        }
        let path = contour
            .try_to_bezpath()
            .map_err(|e| source_error(format, e.to_string()))?;
        validate_path(&path, format)?;
        result.extend(path);
    }
    validate_path(&result, format)?;
    Ok(result)
}

fn valid_number(value: f64) -> bool {
    value.is_finite() && value.abs() <= 1_000_000_000.0
}

fn validate_path(path: &BezPath, format: &'static str) -> Result<(), Error> {
    if path.elements().len() > MAX_ELEMENTS
        || path.elements().iter().any(|element| {
            let valid = |p: kurbo::Point| valid_number(p.x) && valid_number(p.y);
            match *element {
                PathEl::MoveTo(p) | PathEl::LineTo(p) => !valid(p),
                PathEl::QuadTo(a, b) => !valid(a) || !valid(b),
                PathEl::CurveTo(a, b, c) => !valid(a) || !valid(b) || !valid(c),
                PathEl::ClosePath => false,
            }
        })
    {
        return Err(source_error(
            format,
            "nonfinite, out-of-range or excessive path coordinates",
        ));
    }
    Ok(())
}
