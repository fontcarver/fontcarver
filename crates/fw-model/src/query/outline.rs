//! Recursive outline resolution and bounds.

use std::collections::HashSet;

use kurbo::{BezPath, Rect, Shape};

use crate::{Error, Font, GlyphName, MasterId};

const MAX_COMPONENT_DEPTH: usize = 64;

/// Resolves a master layer's contours and transformed components into one path.
pub fn flatten_layer(font: &Font, glyph: &str, master: &MasterId) -> Result<BezPath, Error> {
    let mut visited = HashSet::new();
    flatten_inner(font, &GlyphName::from(glyph), master, 0, &mut visited)
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
