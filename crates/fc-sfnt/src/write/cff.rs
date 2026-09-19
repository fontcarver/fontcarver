//! Name-keyed CFF1 writer. See Adobe Technical Note 5176.

use std::collections::{HashMap, HashSet};

use fc_model::{Font, GlyphName, MasterId};
use kurbo::{BezPath, Rect};
use read_fonts::ps::string::STANDARD_STRINGS;

use super::{
    charstring,
    encoding::{index, integer, offset, real},
    WriteError,
};

/// CFF bytes and geometry needed by the enclosing OpenType tables.
pub struct CffOutput {
    /// A complete name-keyed CFF1 table.
    pub bytes: Vec<u8>,
    /// Per-glyph curve bounds in the supplied glyph-ID order; empty outlines are `None`.
    pub bounds: Vec<Option<Rect>>,
    /// Rounded horizontal advances in the supplied glyph-ID order.
    pub advances: Vec<u16>,
}

/// Writes one static master as CFF1, retaining the caller's exact glyph-ID order.
///
/// The order must begin with `.notdef`. A missing `.notdef` gets an empty outline.
/// Returns an error for invalid geometry, names, widths or format limits.
pub fn write_cff(
    font: &Font,
    master: &MasterId,
    order: &[GlyphName],
    production_names: bool,
) -> Result<CffOutput, WriteError> {
    if font.master(master).is_none() {
        return Err(WriteError(format!("master {master} does not exist")));
    }
    if order.first().map(|name| name.as_ref()) != Some(".notdef") || order.len() > 65535 {
        return Err(WriteError(
            "glyph order must start with .notdef and contain at most 65,535 glyphs".into(),
        ));
    }
    if !(16..=16384).contains(&font.info.units_per_em) {
        return Err(WriteError("units per em outside 16..=16384".into()));
    }
    let mut strings = Strings::new();
    let mut charset = vec![0];
    let mut charstrings = Vec::with_capacity(order.len());
    let mut bounds = Vec::with_capacity(order.len());
    let mut advances = Vec::with_capacity(order.len());
    let mut seen = HashSet::new();
    let mut bbox: Option<Rect> = None;
    for (gid, name) in order.iter().enumerate() {
        let glyph = font.glyph(name);
        let output_name = if gid == 0 {
            ".notdef"
        } else {
            glyph
                .and_then(|glyph| {
                    production_names
                        .then_some(glyph.production_name.as_deref())
                        .flatten()
                })
                .unwrap_or(name.as_ref())
        };
        check_name(output_name, 127)?;
        if !seen.insert(output_name.to_owned()) {
            return Err(WriteError(format!(
                "duplicate output glyph name {output_name}"
            )));
        }
        if gid > 0 {
            charset.extend_from_slice(&strings.sid(output_name)?.to_be_bytes());
        }
        let (path, width) = if let Some(glyph) = glyph {
            let layer = glyph
                .master_layer(master)
                .ok_or_else(|| WriteError(format!("{name} lacks master {master}")))?;
            let mut visited = HashSet::new();
            let mut budget = 1_000_000usize;
            (
                checked_path(font, name, master, &mut visited, &mut budget)?,
                layer.width,
            )
        } else if gid == 0 {
            (BezPath::new(), 0.0)
        } else {
            return Err(WriteError(format!("missing glyph {name}")));
        };
        if !width.is_finite() || !(0.0..=32767.0).contains(&width.round()) {
            return Err(WriteError(format!(
                "advance {width} for {name} exceeds the supported CFF 0..=32767 width range"
            )));
        }
        let (encoded, bound) = charstring::encode(&path, width.round())
            .map_err(|error| WriteError(format!("{name}: {}", error.0)))?;
        if let Some(bound) = bound {
            if [
                bound.x0.floor(),
                bound.y0.floor(),
                bound.x1.ceil(),
                bound.y1.ceil(),
            ]
            .iter()
            .any(|v| !v.is_finite() || !(-32768.0..=32767.0).contains(v))
            {
                return Err(WriteError(format!(
                    "outline bounds for {name} exceed OpenType signed 16-bit limits"
                )));
            }
            bbox = Some(bbox.map_or(bound, |old| old.union(bound)));
        }
        charstrings.push(encoded);
        bounds.push(bound);
        advances.push(width.round() as u16);
    }
    let fallback: String = format!("{}-{}", font.info.family_name, font.info.style_name)
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-')
        .collect();
    let font_name = font
        .info
        .postscript_font_name
        .as_deref()
        .unwrap_or(if fallback.is_empty() {
            "FontCarver-Regular"
        } else {
            &fallback
        });
    check_name(font_name, 63)?;
    let name_index = index(&[font_name.as_bytes().to_vec()])?;
    let string_index = index(&strings.custom)?;
    let charstring_index = index(&charstrings)?;
    let private = vec![139, 20, 139, 21]; // defaultWidthX = nominalWidthX = 0
    let bbox = bbox.unwrap_or(Rect::ZERO);
    let top = top_dict(font.info.units_per_em, bbox, 0, 0, 0, private.len())?;
    let charset_offset = 4 + name_index.len() + index(&[top])?.len() + string_index.len() + 2;
    let charstrings_offset = charset_offset + charset.len();
    let private_offset = charstrings_offset + charstring_index.len();
    let top = index(&[top_dict(
        font.info.units_per_em,
        bbox,
        charset_offset,
        charstrings_offset,
        private_offset,
        private.len(),
    )?])?;
    let mut bytes = vec![1, 0, 4, 4];
    bytes.extend(name_index);
    bytes.extend(top);
    bytes.extend(string_index);
    bytes.extend([0, 0]); // Global Subrs INDEX
    bytes.extend(charset);
    bytes.extend(charstring_index);
    bytes.extend(private);
    Ok(CffOutput {
        bytes,
        bounds,
        advances,
    })
}

fn check_name(name: &str, limit: usize) -> Result<(), WriteError> {
    if name.is_empty()
        || name.len() > limit
        || name
            .bytes()
            .any(|byte| !(33..=126).contains(&byte) || b"[](){}<>/%".contains(&byte))
    {
        return Err(WriteError(format!(
            "{name:?} is not a valid PostScript name (maximum {limit} ASCII bytes)"
        )));
    }
    Ok(())
}

fn top_dict(
    upem: u16,
    bbox: Rect,
    charset: usize,
    charstrings: usize,
    private: usize,
    private_size: usize,
) -> Result<Vec<u8>, WriteError> {
    let mut data = Vec::new();
    for value in [
        bbox.x0.floor(),
        bbox.y0.floor(),
        bbox.x1.ceil(),
        bbox.y1.ceil(),
    ] {
        integer(&mut data, value as i32);
    }
    data.push(5);
    for value in [
        1.0 / f64::from(upem),
        0.0,
        0.0,
        1.0 / f64::from(upem),
        0.0,
        0.0,
    ] {
        real(&mut data, value)?;
    }
    data.extend([12, 7]);
    offset(&mut data, charset)?;
    data.push(15);
    offset(&mut data, charstrings)?;
    data.push(17);
    offset(&mut data, private_size)?;
    offset(&mut data, private)?;
    data.push(18);
    Ok(data)
}

fn checked_path(
    font: &Font,
    name: &GlyphName,
    master: &MasterId,
    visited: &mut HashSet<GlyphName>,
    budget: &mut usize,
) -> Result<BezPath, WriteError> {
    *budget = budget
        .checked_sub(1)
        .ok_or_else(|| WriteError(format!("{name} exceeds the component expansion limit")))?;
    if visited.len() >= 64 || !visited.insert(name.clone()) {
        return Err(WriteError(format!(
            "component cycle or depth limit at {name}"
        )));
    }
    let glyph = font
        .glyph(name)
        .ok_or_else(|| WriteError(format!("missing component {name}")))?;
    let layer = glyph
        .master_layer(master)
        .ok_or_else(|| WriteError(format!("{name} lacks master {master}")))?;
    let mut result = BezPath::new();
    for contour in &layer.contours {
        if !contour.closed
            || contour
                .points
                .iter()
                .any(|point| !point.x.is_finite() || !point.y.is_finite())
        {
            return Err(WriteError(format!(
                "{name} has an open or non-finite contour"
            )));
        }
        *budget = budget
            .checked_sub(contour.points.len())
            .ok_or_else(|| WriteError(format!("{name} exceeds the outline point limit")))?;
        result.extend(
            contour
                .try_to_bezpath()
                .map_err(|error| WriteError(error.to_string()))?
                .elements()
                .iter()
                .copied(),
        );
    }
    for component in &layer.components {
        if component.point_match.is_some()
            || component
                .smart
                .as_ref()
                .is_some_and(|values| !values.is_empty())
        {
            return Err(WriteError(format!("{name} has point-matched or smart components that must be resolved before CFF export")));
        }
        if component
            .transform
            .as_coeffs()
            .iter()
            .any(|value| !value.is_finite())
        {
            return Err(WriteError(format!(
                "{name} has a non-finite component transform"
            )));
        }
        let path = checked_path(font, &component.base, master, visited, budget)?;
        result.extend((component.transform * path).elements().iter().copied());
    }
    visited.remove(name);
    Ok(result)
}

struct Strings {
    ids: HashMap<String, u16>,
    custom: Vec<Vec<u8>>,
}
impl Strings {
    fn new() -> Self {
        Self {
            ids: STANDARD_STRINGS
                .iter()
                .enumerate()
                .map(|(id, name)| ((*name).into(), id as u16))
                .collect(),
            custom: Vec::new(),
        }
    }
    fn sid(&mut self, value: &str) -> Result<u16, WriteError> {
        if let Some(sid) = self.ids.get(value) {
            return Ok(*sid);
        }
        let sid = self.custom.len() + STANDARD_STRINGS.len();
        if sid > 64999 {
            return Err(WriteError(
                "name-keyed CFF exceeds its 65,000 string limit; CID export is required".into(),
            ));
        }
        self.custom.push(value.as_bytes().to_vec());
        self.ids.insert(value.into(), sid as u16);
        Ok(sid as u16)
    }
}
