//! Conversion between horizontal AFM metrics and editable font documents.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{Read, Write},
    path::Path,
};

use fc_model::{FeaturesSource, Font, Glyph, GlyphName, KernSide, Layer, MasterId};
use read_fonts::{ps::charmap::Charmap, types::GlyphId};

use super::{err, read_afm, validate, write_afm, GlyphMetrics, KerningPair, MetricsFont};
use crate::Error;

const SOURCE_KEY: &str = "org.fontcarver.afm";
const MAX_FILE_BYTES: usize = 32 * 1024 * 1024;

/// Reads AFM into an outline-free font with 1000 units per em.
///
/// AFM encoding codes are retained as source data, never interpreted as Unicode.
/// Unicode assignments are inferred from Adobe glyph names; alternate names do
/// not receive fallback Unicode assignments.
pub fn read_afm_path(path: &Path) -> Result<Font, Error> {
    let mut text = String::new();
    File::open(path)
        .map_err(|error| Error::io(path, error))?
        .take((MAX_FILE_BYTES + 1) as u64)
        .read_to_string(&mut text)
        .map_err(|error| Error::io(path, error))?;
    if text.len() > MAX_FILE_BYTES {
        return Err(err("input exceeds the 32 MiB limit"));
    }
    into_font(read_afm(&text)?)
}

/// Converts AFM metrics to an outline-free font and retains the complete source.
///
/// Character bounds and ligature declarations stay in `org.fontcarver.afm` user
/// data. They do not create outlines or active OpenType features.
pub fn into_font(metrics: MetricsFont) -> Result<Font, Error> {
    validate(&metrics)?;
    let mut font = Font::new_empty(
        metrics
            .properties
            .get("FamilyName")
            .unwrap_or(&metrics.font_name),
        1000,
    );
    font.info.style_name = metrics
        .properties
        .get("Weight")
        .cloned()
        .unwrap_or_else(|| "Regular".into());
    font.info.postscript_font_name = Some(metrics.font_name.clone());
    font.info.postscript_full_name = metrics.properties.get("FullName").cloned();
    font.info.copyright = metrics.properties.get("Notice").cloned();
    font.info.open_type_name_version = metrics.properties.get("Version").cloned();
    font.info.postscript_is_fixed_pitch = metrics
        .properties
        .get("IsFixedPitch")
        .map(|value| value == "true");
    font.info.postscript_underline_position = property_number(&metrics, "UnderlinePosition")?;
    font.info.postscript_underline_thickness = property_number(&metrics, "UnderlineThickness")?;
    let master = &mut font.masters[0];
    master.name = font.info.style_name.clone();
    master.metrics.ascender = property_number(&metrics, "Ascender")?.unwrap_or_default();
    master.metrics.descender = property_number(&metrics, "Descender")?.unwrap_or_default();
    master.metrics.x_height = property_number(&metrics, "XHeight")?.unwrap_or_default();
    master.metrics.cap_height = property_number(&metrics, "CapHeight")?.unwrap_or_default();
    master.metrics.italic_angle = property_number(&metrics, "ItalicAngle")?.unwrap_or_default();
    let master_id = master.id.clone();
    let charmap = Charmap::from_glyph_names(
        metrics
            .glyphs
            .iter()
            .enumerate()
            .map(|(index, glyph)| (GlyphId::new(index as u32), glyph.name.as_ref())),
    );
    let mut unicodes = BTreeMap::<u32, Vec<u32>>::new();
    let mut assigned = BTreeSet::new();
    for (code, glyph) in charmap.iter() {
        // Fontations tags name variants with bit 31. These are fallback choices,
        // not independent Unicode mappings for glyphs such as A.alt.
        if char::from_u32(code).is_some() && assigned.insert(code) {
            unicodes.entry(glyph.to_u32()).or_default().push(code);
        }
    }
    for (index, metric) in metrics.glyphs.iter().enumerate() {
        let mut glyph = Glyph::new(metric.name.clone());
        glyph.unicodes = unicodes.remove(&(index as u32)).unwrap_or_default();
        let mut layer = Layer::new_master(master_id.clone());
        layer.width = metric.width;
        glyph.layers.push(layer);
        font.insert_glyph(glyph)
            .map_err(|error| err(error.to_string()))?;
    }
    validate_references(&metrics, &font)?;
    apply_to_font(&metrics, &mut font, &master_id)?;
    font.user_data = serde_json::json!({(SOURCE_KEY): metrics});
    Ok(font)
}

/// Applies only explicitly supplied advances and direct horizontal kerning pairs.
///
/// Values are scaled from AFM's 1000 units per em. Existing unlisted pairs remain
/// intact. Every target and scaled value is validated before the first mutation.
/// Font metadata, bounds, outlines, ligatures and other masters remain unchanged.
pub fn apply_to_font(
    metrics: &MetricsFont,
    font: &mut Font,
    master: &MasterId,
) -> Result<(), Error> {
    validate(metrics)?;
    font.master(master)
        .ok_or_else(|| Error::MissingMaster(master.to_string()))?;
    let scale = scale(font)?;
    let mut widths = BTreeMap::new();
    for glyph in &metrics.glyphs {
        require_layer(font, &glyph.name, master)?;
        widths.insert(glyph.name.clone(), scaled(glyph.width, scale)?);
    }
    let mut pairs = Vec::with_capacity(metrics.kerning.len());
    for pair in &metrics.kerning {
        require_layer(font, &pair.left, master)?;
        require_layer(font, &pair.right, master)?;
        pairs.push((
            (
                KernSide::Glyph(pair.left.clone()),
                KernSide::Glyph(pair.right.clone()),
            ),
            scaled(pair.value, scale)?,
        ));
    }
    for glyph in font.glyphs.values_mut() {
        if let Some(width) = widths.get(&glyph.name) {
            if let Some(layer) = glyph.master_layer_mut(master) {
                layer.width = *width;
            }
        }
    }
    if let Some(master) = font.masters.iter_mut().find(|item| &item.id == master) {
        master.kerning_h.0.extend(pairs);
    }
    Ok(())
}

/// Selects horizontal metrics from one master, expressing them in 1000-em units.
///
/// Outlines are used solely to compute bounds; empty imported glyphs retain their
/// source bounds, codes and ligatures. This is an explicit metrics-only export.
/// Class/vertical kerning, active feature source, imported layout, vertical
/// advances and live effects are rejected because they cannot be represented.
pub fn from_font(font: &Font, master_id: &MasterId) -> Result<MetricsFont, Error> {
    let master = font
        .master(master_id)
        .ok_or_else(|| Error::MissingMaster(master_id.to_string()))?;
    let scale = 1.0 / scale(font)?;
    crate::check_layer_effects(font)?;
    if font.has_unapplied_feature_draft() {
        return Err(Error::UnappliedFeatureDraft);
    }
    if font.features.source_of_truth == FeaturesSource::ImportedTables {
        return Err(Error::ImportedLayoutRequiresConversion);
    }
    if !font.features.to_fea().trim().is_empty() {
        return Err(err(
            "active OpenType feature source cannot be represented as AFM metrics",
        ));
    }
    if !master.kerning_v.0.is_empty()
        || master.metrics.default_vertical_advance.is_some()
        || master.metrics.default_vertical_origin.is_some()
    {
        return Err(err("vertical metrics and kerning are not supported"));
    }
    let source = font
        .user_data
        .get(SOURCE_KEY)
        .map(|value| serde_json::from_value::<MetricsFont>(value.clone()))
        .transpose()?;
    if let Some(source) = &source {
        validate(source)?;
    }
    let mut result = source.clone().unwrap_or_default();
    result.font_name = font
        .info
        .postscript_font_name
        .clone()
        .or_else(|| source.as_ref().map(|source| source.font_name.clone()))
        .unwrap_or_else(|| {
            format!("{}-{}", font.info.family_name, font.info.style_name)
                .chars()
                .filter(|ch| !ch.is_whitespace())
                .collect()
        });
    update_properties(&mut result, font, master_id, source.as_ref(), scale)?;
    let original: BTreeMap<_, _> = source
        .iter()
        .flat_map(|source| &source.glyphs)
        .map(|glyph| (&glyph.name, glyph))
        .collect();
    result.glyphs.clear();
    result.kerning.clear();
    let mut actual_outlines = false;
    let mut bounds = None::<kurbo::Rect>;
    for glyph in font.glyphs.values().filter(|glyph| glyph.export) {
        let layer = require_layer(font, &glyph.name, master_id)?;
        if layer.vertical_width.is_some() || layer.vertical_origin.is_some() {
            return Err(err(format!(
                "glyph {}: vertical metrics are not supported",
                glyph.name
            )));
        }
        let mut metric = original.get(&glyph.name).map_or_else(
            || GlyphMetrics {
                name: glyph.name.clone(),
                ..Default::default()
            },
            |metric| (*metric).clone(),
        );
        metric.width = scaled(layer.width, scale)?;
        let bbox = fc_model::query::layer_bbox(font, &glyph.name, master_id)
            .map_err(|error| err(error.to_string()))?;
        if let Some(bbox) = bbox {
            actual_outlines = true;
            metric.bbox = Some([
                scaled(bbox.x0, scale)?,
                scaled(bbox.y0, scale)?,
                scaled(bbox.x1, scale)?,
                scaled(bbox.y1, scale)?,
            ]);
        } else if !layer.contours.is_empty() || !layer.components.is_empty() {
            // An explicitly drawn empty component no longer represents the
            // imported glyph's original AFM bounds.
            metric.bbox = None;
            actual_outlines = true;
        }
        if let Some([x0, y0, x1, y1]) = metric.bbox {
            let next = kurbo::Rect::new(x0, y0, x1, y1);
            bounds = Some(bounds.map_or(next, |bounds| bounds.union(next)));
        }
        result.glyphs.push(metric);
    }
    for ((left, right), value) in &master.kerning_h.0 {
        let (KernSide::Glyph(left), KernSide::Glyph(right)) = (left, right) else {
            return Err(err("class kerning must be expanded before AFM export"));
        };
        result.kerning.push(KerningPair {
            left: left.clone(),
            right: right.clone(),
            value: scaled(*value, scale)?,
        });
    }
    let glyph_set_changed = result.glyphs.len() != original.len()
        || result
            .glyphs
            .iter()
            .any(|glyph| !original.contains_key(&glyph.name));
    if actual_outlines || source.is_none() || glyph_set_changed {
        let bbox = bounds.unwrap_or_default();
        result.properties.insert(
            "FontBBox".into(),
            format!("{} {} {} {}", bbox.x0, bbox.y0, bbox.x1, bbox.y1),
        );
    }
    if result.properties.contains_key("Characters") {
        result
            .properties
            .insert("Characters".into(), result.glyphs.len().to_string());
    }
    // Global CharWidth is a default advance; all exported glyphs have explicit
    // widths, so retain it only when still truthful after model edits.
    if result.properties.contains_key("CharWidth") {
        let first = result.glyphs.first().map(|glyph| glyph.width);
        if let Some(width) =
            first.filter(|width| result.glyphs.iter().all(|glyph| glyph.width == *width))
        {
            result
                .properties
                .insert("CharWidth".into(), format!("{width} 0"));
        } else {
            result.properties.remove("CharWidth");
        }
    }
    validate(&result)?;
    validate_export_references(&result)?;
    Ok(result)
}

/// Writes selected horizontal metrics, atomically replacing the destination.
///
/// Complete validation and serialization occur before creating the temporary
/// file. Outline geometry is deliberately omitted by this metrics-only format.
pub fn write_afm_font(font: &Font, master: &MasterId, path: &Path) -> Result<(), Error> {
    let text = write_afm(&from_font(font, master)?)?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary =
        tempfile::NamedTempFile::new_in(parent).map_err(|error| Error::io(parent, error))?;
    temporary
        .write_all(text.as_bytes())
        .map_err(|error| Error::io(path, error))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|error| Error::io(path, error))?;
    temporary
        .persist(path)
        .map_err(|error| Error::io(path, error.error))?;
    Ok(())
}

fn scale(font: &Font) -> Result<f64, Error> {
    if font.info.units_per_em == 0 {
        return Err(err("units per em must be nonzero"));
    }
    Ok(f64::from(font.info.units_per_em) / 1000.0)
}

fn scaled(value: f64, scale: f64) -> Result<f64, Error> {
    let value = value * scale;
    if !value.is_finite() {
        return Err(err("nonfinite or overflowing metric value"));
    }
    Ok(value)
}

fn require_layer<'a>(
    font: &'a Font,
    name: &GlyphName,
    master: &MasterId,
) -> Result<&'a Layer, Error> {
    font.glyph(name)
        .ok_or_else(|| err(format!("missing glyph {name}")))?
        .master_layer(master)
        .ok_or_else(|| err(format!("glyph {name}: missing layer for master {master}")))
}

fn property_number(metrics: &MetricsFont, key: &str) -> Result<Option<f64>, Error> {
    metrics
        .properties
        .get(key)
        .map(|value| {
            value
                .parse::<f64>()
                .map_err(|_| err(format!("invalid {key}")))
        })
        .transpose()
}

fn validate_references(metrics: &MetricsFont, font: &Font) -> Result<(), Error> {
    for glyph in &metrics.glyphs {
        for ligature in &glyph.ligatures {
            for name in [&ligature.successor, &ligature.replacement] {
                if font.glyph(name).is_none() {
                    return Err(err(format!("ligature references missing glyph {name}")));
                }
            }
        }
    }
    Ok(())
}

fn validate_export_references(metrics: &MetricsFont) -> Result<(), Error> {
    let names: BTreeSet<_> = metrics.glyphs.iter().map(|glyph| &glyph.name).collect();
    for name in metrics
        .kerning
        .iter()
        .flat_map(|pair| [&pair.left, &pair.right])
        .chain(
            metrics
                .glyphs
                .iter()
                .flat_map(|glyph| &glyph.ligatures)
                .flat_map(|ligature| [&ligature.successor, &ligature.replacement]),
        )
    {
        if !names.contains(name) {
            return Err(err(format!(
                "metric references missing or non-exported glyph {name}"
            )));
        }
    }
    Ok(())
}

fn update_properties(
    result: &mut MetricsFont,
    font: &Font,
    master: &MasterId,
    source: Option<&MetricsFont>,
    scale: f64,
) -> Result<(), Error> {
    let master = font
        .master(master)
        .ok_or_else(|| Error::MissingMaster(master.to_string()))?;
    for (key, value, fallback) in [
        (
            "FamilyName",
            Some(font.info.family_name.clone()),
            source.map(|source| source.font_name.as_str()).unwrap_or(""),
        ),
        ("Weight", Some(font.info.style_name.clone()), "Regular"),
        ("FullName", font.info.postscript_full_name.clone(), ""),
        ("Notice", font.info.copyright.clone(), ""),
        ("Version", font.info.open_type_name_version.clone(), ""),
    ] {
        match value {
            Some(value) if value.is_empty() => {
                result.properties.remove(key);
            }
            Some(value)
                if result.properties.contains_key(key)
                    || (source.is_none() && !value.is_empty())
                    || value != fallback =>
            {
                result.properties.insert(key.into(), value);
            }
            None => {
                result.properties.remove(key);
            }
            _ => {}
        }
    }
    for (key, value, factor) in [
        ("Ascender", Some(master.metrics.ascender), scale),
        ("Descender", Some(master.metrics.descender), scale),
        ("XHeight", Some(master.metrics.x_height), scale),
        ("CapHeight", Some(master.metrics.cap_height), scale),
        ("ItalicAngle", Some(master.metrics.italic_angle), 1.0),
        (
            "UnderlinePosition",
            font.info.postscript_underline_position,
            scale,
        ),
        (
            "UnderlineThickness",
            font.info.postscript_underline_thickness,
            scale,
        ),
    ] {
        if let Some(value) = value {
            let value = scaled(value, factor)?;
            if result.properties.contains_key(key) || source.is_none() || value != 0.0 {
                // Preserve equivalent textual values to keep source semantics.
                if property_number(result, key)? != Some(value) {
                    result.properties.insert(key.into(), value.to_string());
                }
            }
        } else {
            result.properties.remove(key);
        }
    }
    if let Some(value) = font.info.postscript_is_fixed_pitch {
        result
            .properties
            .insert("IsFixedPitch".into(), value.to_string());
    } else {
        result.properties.remove("IsFixedPitch");
    }
    Ok(())
}
