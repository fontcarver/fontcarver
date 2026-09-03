//! Preservation of FontWorkshop data that UFO cannot represent.

use std::{collections::HashSet, path::Path, str::FromStr};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use fw_model::{Font, Master, Tag};
use plist::Value;

use super::value::json_to_plist;
use crate::Error;

pub(crate) const EXTRA_KEY: &str = "dev.fontworkshop.extra";

pub(crate) fn copy_images(
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
    let mut copied = HashSet::new();
    for image in font
        .glyphs
        .values()
        .flat_map(|glyph| &glyph.layers)
        .filter(|layer| super::convert::layer_belongs(layer, &master.id))
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
        .and_then(|lib| lib.get(EXTRA_KEY))
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
    if let Some(indices) = extra
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
            let indices: Vec<Vec<fw_model::TtIndex>> = serde_json::from_value(value.clone())?;
            for (contour, contour_indices) in layer.contours.iter_mut().zip(indices) {
                for (point, index) in contour.points.iter_mut().zip(contour_indices) {
                    point.tt_index = index;
                }
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
            layer.components = serde_json::from_value(value.clone())?;
        }
    }
    Ok(())
}

pub(crate) fn build(font: &Font, master: &Master) -> Result<Value, Error> {
    let raw_tables = font
        .extra
        .raw_tables
        .iter()
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
            (layer.hints != fw_model::PsHints::default())
                .then(|| serde_json::to_value(&layer.hints).map(|value| (name.to_string(), value)))
        })
        .collect::<Result<serde_json::Map<_, _>, _>>()?;
    let tt_indices = font
        .glyphs
        .iter()
        .filter_map(|(name, glyph)| {
            let layer = glyph.master_layer(&master.id)?;
            let indices = layer
                .contours
                .iter()
                .map(|contour| {
                    contour
                        .points
                        .iter()
                        .map(|point| point.tt_index)
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            indices
                .iter()
                .flatten()
                .any(|index| *index != fw_model::TtIndex::Unnumbered)
                .then(|| serde_json::to_value(indices).map(|value| (name.to_string(), value)))
        })
        .collect::<Result<serde_json::Map<_, _>, _>>()?;
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
                .then(|| {
                    serde_json::to_value(&layer.components).map(|value| (name.to_string(), value))
                })
        })
        .collect::<Result<serde_json::Map<_, _>, _>>()?;
    let json = serde_json::json!({
        "version": 1,
        "raw_tables": raw_tables,
        "bitmap_strikes": font.bitmap_strikes,
        "tt_instructions": tt_instructions,
        "hints": hints,
        "tt_indices": tt_indices,
        "component_data": component_data,
    });
    json_to_plist(&json)
}
