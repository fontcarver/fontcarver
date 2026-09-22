//! Preservation of FontCarver data that UFO cannot represent.

use std::{collections::HashSet, path::Path, str::FromStr};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use fc_model::{FeaturesSource, Font, Master, Tag, TtIndex};
use plist::Value;

use super::value::json_to_plist;
use crate::Error;

#[path = "feature_metadata.rs"]
pub(crate) mod feature_metadata;

pub(crate) const EXTRA_KEY: &str = "com.fontcarver.extra";
// Read data exported before the product rename without losing preserved tables.
const LEGACY_EXTRA_KEY: &str = "com.fontcutter.extra";
const ORIGINAL_LAYOUT_TABLES: [Tag; 9] = [
    Tag(*b"GSUB"),
    Tag(*b"GPOS"),
    Tag(*b"GDEF"),
    Tag(*b"BASE"),
    Tag(*b"kern"),
    Tag(*b"kerx"),
    Tag(*b"morx"),
    Tag(*b"mort"),
    Tag(*b"JSTF"),
];

pub(crate) fn copy_data_and_images(
    font: &Font,
    master: &Master,
    target: &mut norad::Font,
) -> Result<(), Error> {
    let source_root = master
        .user_data
        .get("ufo.source_path")
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            font.extra
                .source
                .as_ref()
                .and_then(|source| source.path.as_deref())
        })
        .map(Path::new);
    if let Some(source_root) = source_root.filter(|path| path.is_dir()) {
        let source = norad::Font::load_requested_data(source_root, norad::DataRequest::all())?;
        for (path, data) in source.data.iter() {
            let data = data.map_err(|error| Error::UnsupportedPlist(error.to_string()))?;
            target
                .data
                .insert(path.clone(), data.to_vec())
                .map_err(|error| Error::UnsupportedPlist(error.to_string()))?;
        }
        for (path, data) in source.images.iter() {
            let data = data.map_err(|error| Error::UnsupportedPlist(error.to_string()))?;
            target
                .images
                .insert(path.clone(), data.to_vec())
                .map_err(|error| Error::UnsupportedPlist(error.to_string()))?;
        }
    }
    let mut copied = HashSet::new();
    for image in font
        .glyphs
        .values()
        .flat_map(|glyph| &glyph.layers)
        .filter(|layer| super::convert::layer_belongs(font, layer, &master.id))
        .filter_map(|layer| layer.background_image.as_ref())
    {
        if !copied.insert(image.file_name.clone()) {
            continue;
        }
        let data = image.data.clone().or_else(|| {
            source_root
                .and_then(|root| std::fs::read(root.join("images").join(&image.file_name)).ok())
        });
        if let Some(data) = data {
            target
                .images
                .insert(image.file_name.clone().into(), data)
                .map_err(|error| Error::UnsupportedPlist(error.to_string()))?;
        }
    }
    Ok(())
}

pub(crate) fn import(font: &mut Font) -> Result<(), Error> {
    let Some(extra) = font
        .extra
        .lib
        .get("ufo.lib")
        .and_then(serde_json::Value::as_object)
        .and_then(|lib| lib.get(EXTRA_KEY).or_else(|| lib.get(LEGACY_EXTRA_KEY)))
        .cloned()
    else {
        return Ok(());
    };
    let Some(extra) = extra.as_object() else {
        return Ok(());
    };
    if let Some(raw_tables) = extra
        .get("raw_tables")
        .and_then(serde_json::Value::as_object)
    {
        for (tag, value) in raw_tables {
            let Some(encoded) = value.as_str() else {
                continue;
            };
            let Ok(tag) = Tag::from_str(tag) else {
                continue;
            };
            if let Ok(bytes) = STANDARD.decode(encoded) {
                font.extra.raw_tables.insert(tag, bytes);
            }
        }
    }
    if let Some(strikes) = extra.get("bitmap_strikes") {
        font.bitmap_strikes = serde_json::from_value(strikes.clone())?;
        font.bitmap_tables_owned = extra
            .get("bitmap_tables_owned")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(!font.bitmap_strikes.is_empty());
    }
    let master_id = font.default_master().id.clone();
    if let Some(instructions) = extra
        .get("tt_instructions")
        .and_then(serde_json::Value::as_object)
    {
        for (name, encoded) in instructions {
            let Some(encoded) = encoded.as_str() else {
                continue;
            };
            let Some(layer) = font
                .glyphs
                .get_mut(name.as_str())
                .and_then(|glyph| glyph.master_layer_mut(&master_id))
            else {
                continue;
            };
            if let Ok(bytes) = STANDARD.decode(encoded) {
                layer.tt_instructions = Some(bytes);
            }
        }
    }
    if let Some(hints) = extra.get("hints").and_then(serde_json::Value::as_object) {
        for (name, value) in hints {
            let Some(layer) = font
                .glyphs
                .get_mut(name.as_str())
                .and_then(|glyph| glyph.master_layer_mut(&master_id))
            else {
                continue;
            };
            layer.hints = serde_json::from_value(value.clone())?;
        }
    }
    if let Some(encoded) = extra.get("tt_indices").and_then(serde_json::Value::as_str) {
        let indices = decode_packed_indices(encoded)?;
        for (point, index) in font
            .glyphs
            .values_mut()
            .filter_map(|glyph| glyph.master_layer_mut(&master_id))
            .flat_map(|layer| layer.contours.iter_mut())
            .flat_map(|contour| contour.points.iter_mut())
            .zip(indices)
        {
            point.tt_index = index;
        }
    } else if let Some(indices) = extra
        .get("tt_indices")
        .and_then(serde_json::Value::as_object)
    {
        for (name, value) in indices {
            let Some(layer) = font
                .glyphs
                .get_mut(name.as_str())
                .and_then(|glyph| glyph.master_layer_mut(&master_id))
            else {
                continue;
            };
            let indices = decode_indices(value)?;
            for (point, index) in layer
                .contours
                .iter_mut()
                .flat_map(|contour| contour.points.iter_mut())
                .zip(indices)
            {
                point.tt_index = index;
            }
        }
    }
    if let Some(components) = extra
        .get("component_data")
        .and_then(serde_json::Value::as_object)
    {
        for (name, value) in components {
            let Some(layer) = font
                .glyphs
                .get_mut(name.as_str())
                .and_then(|glyph| glyph.master_layer_mut(&master_id))
            else {
                continue;
            };
            import_component_data(&mut layer.components, value)?;
        }
    }
    if let Some(lib) = font
        .extra
        .lib
        .get_mut("ufo.lib")
        .and_then(serde_json::Value::as_object_mut)
    {
        lib.remove(EXTRA_KEY);
        lib.remove(LEGACY_EXTRA_KEY);
    }
    Ok(())
}

pub(crate) fn build(font: &Font, master: &Master) -> Result<Option<Value>, Error> {
    let raw_tables = font
        .extra
        .raw_tables
        .iter()
        // Original layout restore belongs to FCF. A source UFO starts a new
        // editable baseline and must not carry competing original layout.
        .filter(|(tag, _)| {
            font.features.source_of_truth != FeaturesSource::Fea
                || !ORIGINAL_LAYOUT_TABLES.contains(tag)
        })
        .map(|(tag, bytes)| {
            (
                tag.to_string(),
                serde_json::Value::String(STANDARD.encode(bytes)),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    let tt_instructions = font
        .glyphs
        .iter()
        .filter_map(|(name, glyph)| {
            glyph
                .master_layer(&master.id)
                .and_then(|layer| layer.tt_instructions.as_ref())
                .map(|bytes| {
                    (
                        name.to_string(),
                        serde_json::Value::String(STANDARD.encode(bytes)),
                    )
                })
        })
        .collect::<serde_json::Map<_, _>>();
    let hints = font
        .glyphs
        .iter()
        .filter_map(|(name, glyph)| {
            let layer = glyph.master_layer(&master.id)?;
            (layer.hints != fc_model::PsHints::default())
                .then(|| serde_json::to_value(&layer.hints).map(|value| (name.to_string(), value)))
        })
        .collect::<Result<serde_json::Map<_, _>, _>>()?;
    let has_tt_indices = font
        .glyphs
        .values()
        .filter_map(|glyph| glyph.master_layer(&master.id))
        .flat_map(|layer| &layer.contours)
        .flat_map(|contour| &contour.points)
        .any(|point| point.tt_index != TtIndex::Unnumbered);
    let tt_indices = has_tt_indices.then(|| {
        pack_indices(
            font.glyphs
                .values()
                .filter_map(|glyph| glyph.master_layer(&master.id))
                .flat_map(|layer| &layer.contours)
                .flat_map(|contour| &contour.points)
                .map(|point| point.tt_index),
        )
    });
    let component_data = font
        .glyphs
        .iter()
        .filter_map(|(name, glyph)| {
            let layer = glyph.master_layer(&master.id)?;
            layer
                .components
                .iter()
                .any(|component| {
                    component.anchor.is_some()
                        || component.auto_align
                        || component.use_my_metrics
                        || component.round_to_grid
                        || component.smart.is_some()
                        || component.point_match.is_some()
                })
                .then(|| (name.to_string(), build_component_data(&layer.components)))
        })
        .collect::<serde_json::Map<_, _>>();
    let has_data = !raw_tables.is_empty()
        || font.bitmap_tables_owned
        || !font.bitmap_strikes.is_empty()
        || !tt_instructions.is_empty()
        || !hints.is_empty()
        || tt_indices.is_some()
        || !component_data.is_empty();
    if !has_data {
        return Ok(None);
    }
    let json = serde_json::json!({
        "version": 1,
        "raw_tables": raw_tables,
        "bitmap_strikes": font.bitmap_strikes,
        "bitmap_tables_owned": font.bitmap_tables_owned,
        "tt_instructions": tt_instructions,
        "hints": hints,
        "tt_indices": tt_indices,
        "component_data": component_data,
    });
    json_to_plist(&json).map(Some)
}

fn pack_indices(indices: impl Iterator<Item = TtIndex>) -> String {
    let mut bytes = Vec::new();
    for index in indices {
        match index {
            TtIndex::Index(value) if value != u16::MAX => {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            TtIndex::Unnumbered => {
                bytes.extend_from_slice(&u16::MAX.to_le_bytes());
                bytes.extend_from_slice(&0_u16.to_le_bytes());
            }
            TtIndex::Implied => {
                bytes.extend_from_slice(&u16::MAX.to_le_bytes());
                bytes.extend_from_slice(&1_u16.to_le_bytes());
            }
            TtIndex::Index(_) => {
                bytes.extend_from_slice(&u16::MAX.to_le_bytes());
                bytes.extend_from_slice(&2_u16.to_le_bytes());
            }
        }
    }
    STANDARD.encode(bytes)
}

fn decode_indices(value: &serde_json::Value) -> Result<Vec<TtIndex>, Error> {
    if let Some(encoded) = value.as_str() {
        return decode_packed_indices(encoded);
    }
    let Some(contours) = value.as_array() else {
        let legacy: Vec<Vec<TtIndex>> = serde_json::from_value(value.clone())?;
        return Ok(legacy.into_iter().flatten().collect());
    };
    if !contours.iter().all(|contour| contour.is_string()) {
        let legacy: Vec<Vec<TtIndex>> = serde_json::from_value(value.clone())?;
        return Ok(legacy.into_iter().flatten().collect());
    }
    let mut result = Vec::new();
    for contour in contours {
        result.extend(decode_packed_indices(contour.as_str().unwrap_or_default())?);
    }
    Ok(result)
}

fn decode_packed_indices(encoded: &str) -> Result<Vec<TtIndex>, Error> {
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|error| Error::UnsupportedPlist(error.to_string()))?;
    if bytes.len() % 2 != 0 {
        return Err(Error::UnsupportedPlist(
            "packed TrueType indices have an odd byte length".to_owned(),
        ));
    }
    let words = bytes
        .chunks_exact(2)
        .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
        .collect::<Vec<_>>();
    let mut result = Vec::new();
    let mut index = 0;
    while index < words.len() {
        if words[index] != u16::MAX {
            result.push(TtIndex::Index(words[index]));
            index += 1;
            continue;
        }
        let tag = words.get(index + 1).ok_or_else(|| {
            Error::UnsupportedPlist("truncated packed TrueType index escape".to_owned())
        })?;
        result.push(match tag {
            0 => TtIndex::Unnumbered,
            1 => TtIndex::Implied,
            2 => TtIndex::Index(u16::MAX),
            _ => {
                return Err(Error::UnsupportedPlist(format!(
                    "unknown packed TrueType index escape {tag}"
                )));
            }
        });
        index += 2;
    }
    Ok(result)
}

fn build_component_data(components: &[fc_model::Component]) -> serde_json::Value {
    let flags = components
        .iter()
        .map(|component| {
            u8::from(component.auto_align)
                | (u8::from(component.use_my_metrics) << 1)
                | (u8::from(component.round_to_grid) << 2)
        })
        .collect::<Vec<_>>();
    let details = components
        .iter()
        .enumerate()
        .filter(|(_, component)| {
            component.anchor.is_some()
                || component.smart.is_some()
                || component.point_match.is_some()
        })
        .map(|(index, component)| {
            (
                index.to_string(),
                serde_json::json!([component.anchor, component.smart, component.point_match]),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    if details.is_empty() {
        serde_json::Value::String(STANDARD.encode(flags))
    } else {
        serde_json::json!({ "flags": STANDARD.encode(flags), "details": details })
    }
}

fn import_component_data(
    components: &mut Vec<fc_model::Component>,
    value: &serde_json::Value,
) -> Result<(), Error> {
    if value.is_array() {
        *components = serde_json::from_value(value.clone())?;
        return Ok(());
    }
    let encoded = value
        .as_str()
        .or_else(|| value.get("flags").and_then(serde_json::Value::as_str))
        .ok_or_else(|| Error::UnsupportedPlist("invalid component metadata".to_owned()))?;
    let flags = STANDARD
        .decode(encoded)
        .map_err(|error| Error::UnsupportedPlist(error.to_string()))?;
    for (component, flags) in components.iter_mut().zip(flags) {
        component.auto_align = flags & 1 != 0;
        component.use_my_metrics = flags & 2 != 0;
        component.round_to_grid = flags & 4 != 0;
    }
    if let Some(details) = value.get("details").and_then(serde_json::Value::as_object) {
        for (index, details) in details {
            let Some(component) = index
                .parse::<usize>()
                .ok()
                .and_then(|index| components.get_mut(index))
            else {
                continue;
            };
            let Some(details) = details.as_array() else {
                continue;
            };
            component.anchor = details
                .first()
                .cloned()
                .map(serde_json::from_value)
                .transpose()?
                .flatten();
            component.smart = details
                .get(1)
                .cloned()
                .map(serde_json::from_value)
                .transpose()?
                .flatten();
            component.point_match = details
                .get(2)
                .cloned()
                .map(serde_json::from_value)
                .transpose()?
                .flatten();
        }
    }
    Ok(())
}
