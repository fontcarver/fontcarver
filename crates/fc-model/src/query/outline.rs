//! Recursive outline resolution and bounds, including local smart components.
use crate::{Error, Font, GlyphName, Layer, LayerId, LayerKind, MasterId};
use kurbo::{BezPath, Rect, Shape};
use std::{
    borrow::Cow,
    collections::{BTreeMap, HashSet},
};

const MAX_COMPONENT_DEPTH: usize = 64;
const MAX_PATH_ELEMENTS: usize = 1_000_000;

/// Optional geometry-stage filter, called after a layer's components resolve.
/// The callback must not mutate the font and returns a bounded finite path.
pub type LayerPathFilter<'a> = dyn Fn(&Layer, BezPath) -> Result<BezPath, Error> + 'a;

/// Resolve raw source geometry, including ordinary and smart components.
/// Persistent layer effects are evaluated only by geometry consumers via the
/// explicit filter variants; this model-only query leaves them unapplied.
pub fn flatten_layer(font: &Font, glyph: &str, master: &MasterId) -> Result<BezPath, Error> {
    let mut budget = MAX_PATH_ELEMENTS;
    flatten_inner(
        font,
        &glyph.into(),
        master,
        None,
        &mut HashSet::new(),
        &mut budget,
        None,
    )
}

/// Resolve sources with a layer filter, including every nested component source.
/// Child filters run before component transforms; the parent filter runs last.
pub fn flatten_layer_with(
    font: &Font,
    glyph: &str,
    master: &MasterId,
    filter: &LayerPathFilter<'_>,
) -> Result<BezPath, Error> {
    let mut budget = MAX_PATH_ELEMENTS;
    flatten_inner(
        font,
        &glyph.into(),
        master,
        None,
        &mut HashSet::new(),
        &mut budget,
        Some(filter),
    )
}

/// Resolve one component's local parameters in a global master, with cycle/work limits.
pub fn flatten_smart_component(
    font: &Font,
    glyph: &str,
    master: &MasterId,
    values: &BTreeMap<String, f64>,
) -> Result<BezPath, Error> {
    let mut budget = MAX_PATH_ELEMENTS;
    flatten_inner(
        font,
        &glyph.into(),
        master,
        Some(values),
        &mut HashSet::new(),
        &mut budget,
        None,
    )
}

/// Resolve a local smart instance, then filter its geometry and nested sources.
pub fn flatten_smart_component_with(
    font: &Font,
    glyph: &str,
    master: &MasterId,
    values: &BTreeMap<String, f64>,
    filter: &LayerPathFilter<'_>,
) -> Result<BezPath, Error> {
    let mut budget = MAX_PATH_ELEMENTS;
    flatten_inner(
        font,
        &glyph.into(),
        master,
        Some(values),
        &mut HashSet::new(),
        &mut budget,
        Some(filter),
    )
}

/// Resolve a specific layer; its components use the associated master.
pub fn flatten_layer_by_id(font: &Font, glyph: &str, layer: &LayerId) -> Result<BezPath, Error> {
    flatten_selected_layer(font, glyph, layer, None)
}

/// Resolve a specific layer and filter it after its component sources resolve.
pub fn flatten_layer_by_id_with(
    font: &Font,
    glyph: &str,
    layer: &LayerId,
    filter: &LayerPathFilter<'_>,
) -> Result<BezPath, Error> {
    flatten_selected_layer(font, glyph, layer, Some(filter))
}

fn flatten_selected_layer(
    font: &Font,
    glyph: &str,
    layer: &LayerId,
    filter: Option<&LayerPathFilter<'_>>,
) -> Result<BezPath, Error> {
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
        | LayerKind::Bracket { master, .. }
        | LayerKind::Smart { master, .. } => master,
        LayerKind::Backup { of, .. } => of,
        LayerKind::Custom(_) => &font.default_master().id,
    };
    let mut budget = MAX_PATH_ELEMENTS;
    flatten_contents(
        font,
        layer_data,
        master,
        &mut HashSet::from([name]),
        &mut budget,
        filter,
    )
}

/// Return exact bounds of raw source geometry, without layer effects.
pub fn layer_bbox(font: &Font, glyph: &str, master: &MasterId) -> Result<Option<Rect>, Error> {
    let path = flatten_layer(font, glyph, master)?;
    Ok((!path.elements().is_empty()).then(|| path.bounding_box()))
}

fn flatten_inner(
    font: &Font,
    name: &GlyphName,
    master: &MasterId,
    smart: Option<&BTreeMap<String, f64>>,
    visited: &mut HashSet<GlyphName>,
    budget: &mut usize,
    filter: Option<&LayerPathFilter<'_>>,
) -> Result<BezPath, Error> {
    if visited.len() >= MAX_COMPONENT_DEPTH {
        return Err(Error::ComponentDepthLimit {
            glyph: name.clone(),
        });
    }
    if !visited.insert(name.clone()) {
        return Err(Error::ComponentCycle {
            glyph: name.clone(),
        });
    }
    let glyph = font.glyph(name).ok_or_else(|| Error::MissingGlyph {
        glyph: name.clone(),
    })?;
    let layer = if !glyph.smart_axes.is_empty() || smart.is_some_and(|v| !v.is_empty()) {
        Cow::Owned(crate::smart_layer(
            glyph,
            master,
            smart.unwrap_or(&BTreeMap::new()),
        )?)
    } else {
        Cow::Borrowed(
            glyph
                .master_layer(master)
                .ok_or_else(|| Error::MissingMasterLayer {
                    glyph: name.clone(),
                    master: master.clone(),
                })?,
        )
    };
    let result = flatten_contents(font, &layer, master, visited, budget, filter);
    visited.remove(name);
    result
}

fn flatten_contents(
    font: &Font,
    layer: &Layer,
    master: &MasterId,
    visited: &mut HashSet<GlyphName>,
    budget: &mut usize,
    filter: Option<&LayerPathFilter<'_>>,
) -> Result<BezPath, Error> {
    let mut path = BezPath::new();
    for contour in &layer.contours {
        let curve = contour.try_to_bezpath()?;
        *budget = budget
            .checked_sub(curve.elements().len())
            .ok_or_else(|| Error::InvalidDesignSpace("component outline work limit".into()))?;
        path.extend(curve);
    }
    for component in &layer.components {
        if component
            .transform
            .as_coeffs()
            .iter()
            .any(|v| !v.is_finite())
        {
            return Err(Error::InvalidDesignSpace(
                "nonfinite component transform".into(),
            ));
        }
        let base = flatten_inner(
            font,
            &component.base,
            master,
            component.smart.as_ref(),
            visited,
            budget,
            filter,
        )?;
        let transformed = component.transform * base;
        if !transformed.is_finite() {
            return Err(Error::InvalidDesignSpace(
                "component transform overflows outline coordinates".into(),
            ));
        }
        path.extend(transformed);
    }
    if let Some(filter) = filter {
        path = filter(layer, path)?;
        if !path.is_finite() {
            return Err(Error::InvalidDesignSpace(
                "layer outline filter returned nonfinite coordinates".into(),
            ));
        }
        *budget = budget
            .checked_sub(path.elements().len())
            .ok_or_else(|| Error::InvalidDesignSpace("filtered outline work limit".into()))?;
    }
    Ok(path)
}
