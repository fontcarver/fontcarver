//! Recursive outline resolution and bounds.

use std::collections::HashSet;

use kurbo::{BezPath, Rect, Shape};

use crate::{Error, Font, GlyphName, LayerId, LayerKind, MasterId};

const MAX_COMPONENT_DEPTH: usize = 64;

/// Resolves a master layer's contours and transformed components into one path.
pub fn flatten_layer(font: &Font, glyph: &str, master: &MasterId) -> Result<BezPath, Error> {
    let mut visited = HashSet::new();
    flatten_inner(font, &GlyphName::from(glyph), master, 0, &mut visited)
}

/// Resolves one specific layer's contours and transformed components into one path.
///
/// Components are resolved in the master the layer belongs to, so a background,
/// brace, bracket, or backup layer flattens against its own master's outlines.
/// A custom layer resolves components in the default master.
pub fn flatten_layer_by_id(font: &Font, glyph: &str, layer: &LayerId) -> Result<BezPath, Error> {
    let name = GlyphName::from(glyph);
    let glyph_data = font.glyph(&name).ok_or_else(|| Error::MissingGlyph {
        glyph: name.clone(),
    })?;
    let layer_data = glyph_data.layer(layer).ok_or_else(|| Error::MissingLayer {
        glyph: name.clone(),
        layer: layer.clone(),
    })?;
    let master = match &layer_data.kind {
        LayerKind::Master(master)
        | LayerKind::Background(master)
        | LayerKind::Brace { master, .. }
        | LayerKind::Bracket { master, .. } => master.clone(),
        LayerKind::Backup { of, .. } => of.clone(),
        LayerKind::Custom(_) => font.default_master().id.clone(),
    };
    let mut path = BezPath::new();
    for contour in &layer_data.contours {
        path.extend(contour.to_bezpath().elements().iter().copied());
    }
    let mut visited = HashSet::from([name]);
    for component in &layer_data.components {
        let base = flatten_inner(font, &component.base, &master, 1, &mut visited)?;
        path.extend((component.transform * base).elements().iter().copied());
    }
    Ok(path)
}

/// Returns the exact curve bounds of a flattened master layer.
pub fn layer_bbox(font: &Font, glyph: &str, master: &MasterId) -> Result<Option<Rect>, Error> {
    let path = flatten_layer(font, glyph, master)?;
    if path.elements().is_empty() {
        Ok(None)
    } else {
        Ok(Some(path.bounding_box()))
    }
}

fn flatten_inner(
    font: &Font,
    glyph_name: &GlyphName,
    master: &MasterId,
    depth: usize,
    visited: &mut HashSet<GlyphName>,
) -> Result<BezPath, Error> {
    if depth > MAX_COMPONENT_DEPTH {
        return Err(Error::ComponentDepthLimit {
            glyph: glyph_name.clone(),
        });
    }
    if !visited.insert(glyph_name.clone()) {
        return Err(Error::ComponentCycle {
            glyph: glyph_name.clone(),
        });
    }
    let result = flatten_contents(font, glyph_name, master, depth, visited);
    visited.remove(glyph_name);
    result
}

fn flatten_contents(
    font: &Font,
    glyph_name: &GlyphName,
    master: &MasterId,
    depth: usize,
    visited: &mut HashSet<GlyphName>,
) -> Result<BezPath, Error> {
    let glyph = font.glyph(glyph_name).ok_or_else(|| Error::MissingGlyph {
        glyph: glyph_name.clone(),
    })?;
    let layer = glyph
        .master_layer(master)
        .ok_or_else(|| Error::MissingMasterLayer {
            glyph: glyph_name.clone(),
            master: master.clone(),
        })?;

    let mut path = BezPath::new();
    for contour in &layer.contours {
        path.extend(contour.to_bezpath().elements().iter().copied());
    }
    for component in &layer.components {
        let base = flatten_inner(font, &component.base, master, depth + 1, visited)?;
        path.extend((component.transform * base).elements().iter().copied());
    }
    Ok(path)
}
