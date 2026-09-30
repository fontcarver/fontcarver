//! Metrics-only model adaptation with preserved PFM byte encodings.

use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::Path,
};

use fc_model::{FeaturesSource, Font, Glyph, GlyphName, KernSide, Master, MasterId};

use super::{read_pfm, write_pfm, PfmKernPair, PfmMetrics};
use crate::Error;

const SOURCE_KEY: &str = "fontcarver.pfm";
const CODE_KEY: &str = "fontcarver.pfm.code";
const CHARSET_KEY: &str = "fontcarver.pfm.charset";

fn error(reason: impl Into<String>) -> Error {
    Error::Source {
        format: "PFM",
        reason: reason.into(),
    }
}

/// Read a metrics-only PFM into byte-addressed, empty glyph layers.
///
/// Glyph names are `pfmXX` and glyph metadata retains the byte code; no Unicode
/// encoding is inferred. Byte strings are displayed with a Latin-1 byte mapping
/// (a fallback, not an encoding claim). Full original bytes, including the fixed
/// copyright field, remain in `font.user_data["fontcarver.pfm"]` for lossless
/// semantic round trips. New or edited names must be ASCII.
pub fn read_pfm_path(path: &Path) -> Result<Font, Error> {
    let file = std::fs::File::open(path).map_err(|e| Error::io(path, e))?;
    let mut bytes = Vec::new();
    file.take(super::MAX_FILE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| Error::io(path, e))?;
    let metrics = read_pfm(&bytes)?;
    let units = u16::try_from(metrics.extended.master_units)
        .ok()
        .filter(|units| *units != 0)
        .unwrap_or(1000);
    let mut font = Font::new_empty(&display_bytes(&metrics.face_name), units);
    font.info.postscript_font_name = Some(display_bytes(&metrics.postscript_name));
    font.info.copyright = Some(display_copyright(&metrics.copyright));
    font.info.open_type_os2_weight_class = Some(metrics.header.weight);
    font.info.postscript_is_fixed_pitch = Some(metrics.header.pitch_and_family & 1 == 0);
    font.info.postscript_underline_position = Some(-f64::from(metrics.extended.underline_offset));
    font.info.postscript_underline_thickness = Some(f64::from(metrics.extended.underline_width));
    font.info.open_type_os2_strikeout_position = Some(f64::from(metrics.extended.strikeout_offset));
    font.info.open_type_os2_strikeout_size = Some(f64::from(metrics.extended.strikeout_width));
    let master = &mut font.masters[0];
    master.metrics.ascender = f64::from(metrics.header.ascent);
    master.metrics.descender = -f64::from(metrics.extended.lowercase_descent);
    master.metrics.cap_height = f64::from(metrics.extended.cap_height);
    master.metrics.x_height = f64::from(metrics.extended.x_height);
    master.metrics.italic_angle = -f64::from(metrics.extended.slant) / 10.0;
    for pair in &metrics.kerning {
        master.kerning_h.0.insert(
            (
                KernSide::Glyph(glyph_name(pair.left)),
                KernSide::Glyph(glyph_name(pair.right)),
            ),
            f64::from(pair.value),
        );
    }
    for (index, width) in metrics.widths.iter().enumerate() {
        let code = usize::from(metrics.first_char) + index;
        let code = u8::try_from(code).map_err(|_| error("character range exceeds one byte"))?;
        let mut glyph = Glyph::new(glyph_name(code));
        glyph.user_data = serde_json::json!({ CODE_KEY: code });
        font.insert_glyph(glyph).map_err(|e| error(e.to_string()))?;
        // insert_glyph guarantees a layer for the default master.
        let layer = font
            .glyphs
            .get_mut(&glyph_name(code))
            .and_then(|glyph| glyph.layers.first_mut())
            .ok_or_else(|| error("missing imported glyph layer"))?;
        layer.width = f64::from(*width);
    }
    font.user_data = serde_json::json!({ SOURCE_KEY: metrics });
    Ok(font)
}

/// Export horizontal metrics and explicit glyph kerning for one selected master.
///
/// Imported PFM byte assignments and unrelated header fields are preserved.
/// New glyph assignments require one ASCII Unicode value in Windows ANSI
/// charset 0, at 1000 units per em. This writes no outlines. Class kerning and
/// layout are rejected. Clearing an imported optional underline/strikeout
/// metric writes zero to its required PFM field.
/// The destination is replaced atomically only after complete validation.
pub fn write_pfm_font(font: &Font, master_id: &MasterId, path: &Path) -> Result<(), Error> {
    let master = font
        .master(master_id)
        .ok_or_else(|| Error::MissingMaster(master_id.to_string()))?;
    validate_layout(font, master)?;
    let original = font
        .user_data
        .get(SOURCE_KEY)
        .map(|value| serde_json::from_value::<PfmMetrics>(value.clone()))
        .transpose()?;
    let mut metrics = original.clone().unwrap_or_default();
    if original.is_none() {
        let charset = font.user_data.get(CHARSET_KEY);
        if charset.is_some_and(|value| value.as_u64() != Some(0)) {
            return Err(error("new PFM fonts support only Windows ANSI charset 0"));
        }
        metrics.header.charset = 0;
    }
    let codes = collect_codes(font, master, &mut metrics, original.as_ref())?;
    update_names(font, &mut metrics, original.as_ref())?;
    update_metrics(font, master, &mut metrics, original.as_ref())?;
    let mut kerning = Vec::new();
    for ((left, right), value) in &master.kerning_h.0 {
        let (KernSide::Glyph(left), KernSide::Glyph(right)) = (left, right) else {
            return Err(error("class kerning is not supported in PFM"));
        };
        let left = codes
            .get(left)
            .ok_or_else(|| error("kerning references an unencoded glyph"))?;
        let right = codes
            .get(right)
            .ok_or_else(|| error("kerning references an unencoded glyph"))?;
        kerning.push(PfmKernPair {
            left: *left,
            right: *right,
            value: signed(*value, "kerning")?,
        });
    }
    kerning.sort_by_key(|pair| (pair.left, pair.right));
    // Preserve the original pair order when its complete meaning is unchanged.
    let unchanged = original.as_ref().is_some_and(|source| {
        let mut pairs = source.kerning.clone();
        pairs.sort_by_key(|pair| (pair.left, pair.right));
        pairs == kerning
    });
    if !unchanged {
        metrics.kerning = kerning;
    }
    let bytes = write_pfm(&metrics)?;
    read_pfm(&bytes)?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary =
        tempfile::NamedTempFile::new_in(parent).map_err(|e| Error::io(parent, e))?;
    temporary
        .write_all(&bytes)
        .map_err(|e| Error::io(path, e))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|e| Error::io(path, e))?;
    temporary
        .persist(path)
        .map_err(|e| Error::io(path, e.error))?;
    Ok(())
}

fn validate_layout(font: &Font, master: &Master) -> Result<(), Error> {
    if font.has_unapplied_feature_draft() {
        return Err(Error::UnappliedFeatureDraft);
    }
    if font.features.source_of_truth == FeaturesSource::ImportedTables {
        return Err(Error::ImportedLayoutRequiresConversion);
    }
    if !font.features.to_fea().trim().is_empty()
        || !master.kerning_v.0.is_empty()
        || master.metrics.default_vertical_advance.is_some()
        || master.metrics.default_vertical_origin.is_some()
    {
        return Err(error(
            "PFM cannot represent OpenType layout or vertical kerning",
        ));
    }
    if !font.kerning_groups.side1.is_empty()
        || !font.kerning_groups.side2.is_empty()
        || font
            .glyphs
            .values()
            .any(|glyph| glyph.kern_group_side1.is_some() || glyph.kern_group_side2.is_some())
    {
        return Err(error("class kerning is not supported in PFM"));
    }
    Ok(())
}

fn collect_codes(
    font: &Font,
    master: &Master,
    metrics: &mut PfmMetrics,
    original: Option<&PfmMetrics>,
) -> Result<BTreeMap<GlyphName, u8>, Error> {
    let mut codes = BTreeMap::new();
    let mut widths = BTreeMap::new();
    for glyph in font.glyphs.values().filter(|glyph| glyph.export) {
        let code = if let Some(value) = glyph.user_data.get(CODE_KEY) {
            if original.is_none() {
                return Err(error(
                    "raw PFM byte assignments require imported charset metadata",
                ));
            }
            value.as_u64().and_then(|code| u8::try_from(code).ok())
        } else if metrics.header.charset == 0 {
            match glyph.unicodes.as_slice() {
                [code] if *code <= 0x7f => Some(*code as u8),
                _ => None,
            }
        } else {
            None
        }
        .ok_or_else(|| {
            error(format!(
                "glyph {} requires a supported, unique byte assignment",
                glyph.name
            ))
        })?;
        let layer = glyph
            .master_layer(&master.id)
            .ok_or_else(|| error(format!("glyph {} has no selected-master layer", glyph.name)))?;
        if layer.vertical_width.is_some() || layer.vertical_origin.is_some() {
            return Err(error("PFM cannot represent vertical glyph metrics"));
        }
        if widths
            .insert(code, unsigned(layer.width, "advance width")?)
            .is_some()
        {
            return Err(error("duplicate PFM byte assignment"));
        }
        codes.insert(glyph.name.clone(), code);
    }
    let first = *widths
        .first_key_value()
        .ok_or_else(|| error("PFM needs at least one encoded glyph"))?
        .0;
    let last = *widths
        .last_key_value()
        .ok_or_else(|| error("PFM needs at least one encoded glyph"))?
        .0;
    // Retain imported range boundaries so the default/break offsets keep their
    // meanings, even when a glyph is removed or marked non-exporting.
    let first = original.map_or(first, |source| first.min(source.first_char));
    let last = original.map_or(usize::from(last), |source| {
        usize::from(last)
            .max(usize::from(source.first_char) + source.widths.len().saturating_sub(1))
    });
    if last > 255 {
        return Err(error("character range exceeds one byte"));
    }
    let output: Vec<u16> = (usize::from(first)..=last)
        .map(|code| widths.get(&(code as u8)).copied().unwrap_or(0))
        .collect();
    if let Some(source) = original {
        let shift = source.first_char - first;
        metrics.header.default_char = source
            .header
            .default_char
            .checked_add(shift)
            .ok_or_else(|| error("default character exceeds byte range"))?;
        metrics.header.break_char = source
            .header
            .break_char
            .checked_add(shift)
            .ok_or_else(|| error("break character exceeds byte range"))?;
    } else {
        metrics.header.default_char = 0;
        metrics.header.break_char = if first <= 32 && last >= 32 {
            32 - first
        } else {
            0
        };
    }
    let widths_changed =
        original.is_none_or(|source| source.widths != output || source.first_char != first);
    let fixed = font.info.postscript_is_fixed_pitch.unwrap_or(false);
    if widths_changed && fixed {
        let reference_width = widths.first_key_value().map(|(_, width)| width);
        if widths.values().any(|width| Some(width) != reference_width) {
            return Err(error("fixed pitch PFM requires equal advances"));
        }
    }
    if widths_changed {
        metrics.header.maximum_width = output.iter().copied().max().unwrap_or(0);
        metrics.header.average_width = (widths.values().map(|width| u64::from(*width)).sum::<u64>()
            / widths.len() as u64) as u16;
        if fixed && metrics.header.pixel_width != 0 {
            metrics.header.pixel_width = metrics.header.maximum_width;
        }
    }
    metrics.first_char = first;
    metrics.widths = output;
    Ok(codes)
}

fn update_names(
    font: &Font,
    metrics: &mut PfmMetrics,
    original: Option<&PfmMetrics>,
) -> Result<(), Error> {
    if original.is_none_or(|source| display_bytes(&source.face_name) != font.info.family_name) {
        metrics.face_name = ascii_bytes(&font.info.family_name, "family name")?;
    }
    let name = font
        .info
        .postscript_font_name
        .as_deref()
        .unwrap_or(&font.info.family_name);
    if original.is_none_or(|source| display_bytes(&source.postscript_name) != name) {
        metrics.postscript_name = ascii_bytes(name, "PostScript name")?;
    }
    let copyright = font.info.copyright.as_deref().unwrap_or("");
    if original.is_none_or(|source| display_copyright(&source.copyright) != copyright) {
        let mut bytes = ascii_bytes(copyright, "copyright")?;
        if bytes.len() > 60 {
            return Err(error("copyright exceeds the 60-byte PFM field"));
        }
        bytes.resize(60, 0);
        metrics.copyright = bytes;
    }
    Ok(())
}

fn update_metrics(
    font: &Font,
    master: &Master,
    metrics: &mut PfmMetrics,
    original: Option<&PfmMetrics>,
) -> Result<(), Error> {
    let model = &master.metrics;
    if original.is_none_or(|source| {
        u16::try_from(source.extended.master_units).unwrap_or(1000) != font.info.units_per_em
    }) {
        metrics.extended.master_units = i16::try_from(font.info.units_per_em)
            .map_err(|_| error("PFM units per em exceed signed 16-bit range"))?;
        if metrics.extended.master_units == 0 {
            return Err(error("PFM units per em must be positive"));
        }
    }
    metrics.header.ascent = unsigned(model.ascender, "ascender")?;
    metrics.extended.lowercase_descent = signed(-model.descender, "descender")?;
    metrics.extended.cap_height = signed(model.cap_height, "cap height")?;
    metrics.extended.x_height = signed(model.x_height, "x-height")?;
    if original.is_none_or(|source| -f64::from(source.extended.slant) / 10.0 != model.italic_angle)
    {
        metrics.extended.slant = signed(-model.italic_angle * 10.0, "italic angle")?;
        metrics.header.italic = model.italic_angle != 0.0;
    }
    metrics.header.weight = font.info.open_type_os2_weight_class.unwrap_or(400);
    if let Some(value) = font
        .info
        .postscript_underline_position
        .or_else(|| original.map(|_| 0.0))
    {
        metrics.extended.underline_offset = signed(-value, "underline position")?;
    }
    if let Some(value) = font
        .info
        .postscript_underline_thickness
        .or_else(|| original.map(|_| 0.0))
    {
        metrics.extended.underline_width = signed(value, "underline thickness")?;
    }
    if let Some(value) = font
        .info
        .open_type_os2_strikeout_position
        .or_else(|| original.map(|_| 0.0))
    {
        metrics.extended.strikeout_offset = signed(value, "strikeout position")?;
    }
    if let Some(value) = font
        .info
        .open_type_os2_strikeout_size
        .or_else(|| original.map(|_| 0.0))
    {
        metrics.extended.strikeout_width = signed(value, "strikeout thickness")?;
    }
    let fixed = font.info.postscript_is_fixed_pitch.unwrap_or(false);
    if original.is_none_or(|source| (source.header.pitch_and_family & 1 == 0) != fixed) {
        metrics.header.pitch_and_family = (metrics.header.pitch_and_family & !1) | u8::from(!fixed);
        if fixed {
            let width = metrics.widths.first().copied().unwrap_or(0);
            if metrics
                .widths
                .iter()
                .any(|other| *other != 0 && *other != width)
            {
                return Err(error("fixed pitch PFM requires equal advances"));
            }
            metrics.header.pixel_width = width;
        } else {
            metrics.header.pixel_width = 0;
        }
    }
    Ok(())
}

fn glyph_name(code: u8) -> GlyphName {
    format!("pfm{code:02X}").into()
}

fn display_bytes(bytes: &[u8]) -> String {
    bytes.iter().copied().map(char::from).collect()
}

fn display_copyright(bytes: &[u8]) -> String {
    display_bytes(bytes.split(|byte| *byte == 0).next().unwrap_or_default())
}

fn ascii_bytes(text: &str, field: &str) -> Result<Vec<u8>, Error> {
    if !text.is_ascii() || text.as_bytes().contains(&0) {
        return Err(error(format!(
            "new or edited {field} must be ASCII without NUL"
        )));
    }
    Ok(text.as_bytes().to_vec())
}

fn unsigned(value: f64, field: &str) -> Result<u16, Error> {
    if !value.is_finite() || !(0.0..=f64::from(u16::MAX)).contains(&value.round()) {
        return Err(error(format!("{field} exceeds unsigned 16-bit range")));
    }
    Ok(value.round() as u16)
}

fn signed(value: f64, field: &str) -> Result<i16, Error> {
    if !value.is_finite() || !(f64::from(i16::MIN)..=f64::from(i16::MAX)).contains(&value.round()) {
        return Err(error(format!("{field} exceeds signed 16-bit range")));
    }
    Ok(value.round() as i16)
}
