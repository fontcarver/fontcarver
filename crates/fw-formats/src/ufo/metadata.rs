//! Preservation of UFO object metadata absent from the core model.

use plist::Dictionary;
use serde::{Deserialize, Serialize};

use super::value::{dict_to_json, json_to_dict};
use crate::Error;

pub(crate) const IDENTIFIERS_KEY: &str = "dev.fontworkshop.ufo.identifiers";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct ObjectMetadata {
    pub(crate) glyph: Option<String>,
    pub(crate) layer: Option<String>,
    pub(crate) kind: String,
    pub(crate) index: usize,
    pub(crate) point: Option<usize>,
    pub(crate) identifier: Option<String>,
    pub(crate) color: Option<String>,
    pub(crate) lib: Option<serde_json::Value>,
}

impl ObjectMetadata {
    pub(crate) fn new(
        glyph: Option<&str>,
        layer: Option<&str>,
        kind: &str,
        index: usize,
        point: Option<usize>,
    ) -> Self {
        Self {
            glyph: glyph.map(ToOwned::to_owned),
            layer: layer.map(ToOwned::to_owned),
            kind: kind.to_owned(),
            index,
            point,
            ..Self::default()
        }
    }

    pub(crate) fn with_object(
        mut self,
        identifier: Option<&norad::Identifier>,
        color: Option<norad::Color>,
        lib: Option<&Dictionary>,
    ) -> Result<Self, Error> {
        self.identifier = identifier.map(ToString::to_string);
        self.color = color.map(|color| color.to_rgba_string());
        self.lib = lib.map(dict_to_json).transpose()?;
        Ok(self)
    }

    pub(crate) fn decoded_lib(&self) -> Result<Option<Dictionary>, Error> {
        self.lib.as_ref().map(json_to_dict).transpose()
    }

    pub(crate) fn has_data(&self) -> bool {
        self.identifier.is_some() || self.color.is_some() || self.lib.is_some()
    }
}

pub(crate) fn lookup<'a>(
    metadata: &'a [ObjectMetadata],
    glyph: Option<&str>,
    layer: Option<&str>,
    kind: &str,
    index: usize,
    point: Option<usize>,
) -> Option<&'a ObjectMetadata> {
    metadata.iter().find(|entry| {
        entry.glyph.as_deref() == glyph
            && entry.layer.as_deref() == layer
            && entry.kind == kind
            && entry.index == index
            && entry.point == point
    })
}

pub(crate) fn decode(value: Option<&serde_json::Value>) -> Result<Vec<ObjectMetadata>, Error> {
    value
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map(Option::unwrap_or_default)
        .map_err(Error::from)
}

pub(crate) fn record_global_guides(
    info: &norad::FontInfo,
    entries: &mut Vec<ObjectMetadata>,
) -> Result<(), Error> {
    for (index, guide) in info
        .guidelines
        .as_deref()
        .unwrap_or_default()
        .iter()
        .enumerate()
    {
        let entry = ObjectMetadata::new(None, None, "guideline", index, None).with_object(
            guide.identifier(),
            guide.color,
            guide.lib(),
        )?;
        if entry.has_data() {
            entries.push(entry);
        }
    }
    Ok(())
}

pub(crate) fn record_glyph_objects(
    glyph: &norad::Glyph,
    glyph_name: &str,
    layer_name: &str,
    entries: &mut Vec<ObjectMetadata>,
) -> Result<(), Error> {
    for (index, contour) in glyph.contours.iter().enumerate() {
        push_metadata(
            entries,
            ObjectMetadata::new(Some(glyph_name), Some(layer_name), "contour", index, None)
                .with_object(contour.identifier(), None, contour.lib())?,
        );
        for (point_index, point) in contour.points.iter().enumerate() {
            push_metadata(
                entries,
                ObjectMetadata::new(
                    Some(glyph_name),
                    Some(layer_name),
                    "point",
                    index,
                    Some(point_index),
                )
                .with_object(point.identifier(), None, point.lib())?,
            );
        }
    }
    for (index, component) in glyph.components.iter().enumerate() {
        push_metadata(
            entries,
            ObjectMetadata::new(Some(glyph_name), Some(layer_name), "component", index, None)
                .with_object(component.identifier(), None, component.lib())?,
        );
    }
    for (index, anchor) in glyph.anchors.iter().enumerate() {
        push_metadata(
            entries,
            ObjectMetadata::new(Some(glyph_name), Some(layer_name), "anchor", index, None)
                .with_object(anchor.identifier(), anchor.color, anchor.lib())?,
        );
    }
    for (index, guideline) in glyph.guidelines.iter().enumerate() {
        push_metadata(
            entries,
            ObjectMetadata::new(Some(glyph_name), Some(layer_name), "guideline", index, None)
                .with_object(guideline.identifier(), guideline.color, guideline.lib())?,
        );
    }
    Ok(())
}

fn push_metadata(entries: &mut Vec<ObjectMetadata>, entry: ObjectMetadata) {
    if entry.has_data() {
        entries.push(entry);
    }
}
