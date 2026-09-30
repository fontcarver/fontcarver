//! Supplemental construction and node constraint data for GLIF source outlines.

use fc_model::{GlyphConstruction, Layer, NodeLink, NodeLinkedGuide};
use serde::{Deserialize, Serialize};

use crate::Error;

const KEY: &str = "com.fontcarver.constructionData";
const MAX_BYTES: usize = 1024 * 1024;

#[derive(Default, Serialize, Deserialize)]
struct Sources {
    #[serde(default)]
    constructions: Vec<GlyphConstruction>,
    #[serde(default)]
    node_links: Vec<NodeLink>,
    #[serde(default)]
    linked_guides: Vec<NodeLinkedGuide>,
    #[serde(default)]
    super_smooth: Vec<(usize, usize)>,
}

pub(super) fn validate(layer: &Layer) -> Result<(), Error> {
    layer.validate_node_links().map_err(source_error)?;
    layer.validate_constructions().map_err(source_error)
}

pub(super) fn write(layer: &Layer, glyph: &mut norad::Glyph) -> Result<(), Error> {
    validate(layer)?;
    glyph.lib.remove(KEY);
    let sources = Sources {
        constructions: layer.constructions.clone(),
        node_links: layer.node_links.clone(),
        linked_guides: layer.linked_guides.clone(),
        super_smooth: layer
            .contours
            .iter()
            .enumerate()
            .flat_map(|(ci, contour)| {
                contour
                    .points
                    .iter()
                    .enumerate()
                    .filter(|(_, point)| point.super_smooth)
                    .map(move |(pi, _)| (ci, pi))
            })
            .collect(),
    };
    if sources.constructions.is_empty()
        && sources.node_links.is_empty()
        && sources.linked_guides.is_empty()
        && sources.super_smooth.is_empty()
    {
        return Ok(());
    }
    let text = serde_json::to_string(&sources)?;
    if text.len() > MAX_BYTES {
        return Err(source_error("construction metadata exceeds 1 MiB"));
    }
    glyph.lib.insert(KEY.into(), plist::Value::String(text));
    Ok(())
}

pub(super) fn read(glyph: &norad::Glyph, layer: &mut Layer) -> Result<(), Error> {
    let Some(value) = glyph.lib.get(KEY) else {
        return Ok(());
    };
    let text = value
        .as_string()
        .ok_or_else(|| source_error("construction metadata must be JSON text"))?;
    if text.len() > MAX_BYTES {
        return Err(source_error("construction metadata exceeds 1 MiB"));
    }
    let sources: Sources = serde_json::from_str(text)?;
    for (ci, pi) in sources.super_smooth {
        let point = layer
            .contours
            .get_mut(ci)
            .and_then(|contour| contour.points.get_mut(pi))
            .ok_or_else(|| source_error("super smooth metadata refers to a missing node"))?;
        point.super_smooth = true;
    }
    layer.constructions = sources.constructions;
    layer.node_links = sources.node_links;
    layer.linked_guides = sources.linked_guides;
    validate(layer)
}

fn source_error(error: impl std::fmt::Display) -> Error {
    Error::Source {
        format: "UFO construction data",
        reason: error.to_string(),
    }
}
