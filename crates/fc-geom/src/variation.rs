//! Multi-axis interpolation shared by previews, smart components and export.
use std::collections::{HashMap, HashSet};

use fc_model::{Axis, DesignLocation, Font, Glyph, Layer, LayerKind};
use fontdrasil::{
    coords::{CoordConverter, DesignCoord, NormalizedCoord, NormalizedLocation, UserCoord},
    types::{Axis as IrAxis, Tag},
    variations::{RoundingBehaviour, VariationModel},
};
use kurbo::{Affine, BezPath};

use crate::{compatible, Error};

fn invalid(error: impl std::fmt::Display) -> Error {
    Error::DesignSpace(error.to_string())
}

/// Build compiler axes, including explicit mapped minimum, default and maximum.
pub fn compiler_axes(axes: &[Axis]) -> Result<Vec<IrAxis>, Error> {
    axes.iter()
        .map(|axis| {
            axis.validate().map_err(invalid)?;
            let mut points = axis.map.clone();
            for user in [axis.min, axis.default, axis.max] {
                if !points.iter().any(|(u, _)| *u == user) {
                    points.push((user, axis.user_to_design(user)));
                }
            }
            points.sort_by(|a, b| a.0.total_cmp(&b.0));
            let default = points
                .iter()
                .position(|(u, _)| *u == axis.default)
                .ok_or_else(|| invalid("axis default is missing"))?;
            let converter = CoordConverter::new(
                points
                    .into_iter()
                    .map(|(u, d)| (UserCoord::new(u), DesignCoord::new(d)))
                    .collect(),
                default,
            )
            .map_err(invalid)?;
            Ok(IrAxis {
                name: axis.name.clone(),
                tag: Tag::new(&axis.tag.0),
                min: UserCoord::new(axis.min),
                default: UserCoord::new(axis.default),
                max: UserCoord::new(axis.max),
                hidden: axis.hidden,
                converter,
                localized_names: HashMap::new(),
            })
        })
        .collect()
}

/// Normalize a validated full design location, preserving the font's axis order.
pub fn normalized_location(axes: &[Axis], location: &DesignLocation) -> NormalizedLocation {
    axes.iter()
        .map(|axis| {
            (
                Tag::new(&axis.tag.0),
                NormalizedCoord::new(
                    axis.normalize_design(
                        location
                            .get(&axis.tag)
                            .copied()
                            .unwrap_or_else(|| axis.user_to_design(axis.default)),
                    ),
                ),
            )
        })
        .collect()
}

/// Master and intermediate sources for one glyph. Sparse non-default masters are allowed.
pub fn glyph_sources<'a>(
    font: &Font,
    glyph: &'a Glyph,
) -> Result<Vec<(DesignLocation, &'a Layer)>, Error> {
    let mut result = Vec::new();
    for layer in &glyph.layers {
        let location = match &layer.kind {
            LayerKind::Master(master) => font.complete_location(
                &font
                    .master(master)
                    .ok_or_else(|| invalid("source master missing"))?
                    .location,
            ),
            LayerKind::Brace { master, location } => font.intermediate_location(master, location),
            _ => continue,
        }
        .map_err(invalid)?;
        result.push((location, layer));
    }
    Ok(result)
}

/// Interpolate compatible sources without rounding preview coordinates.
pub fn interpolate_sources(
    axes: &[Axis],
    sources: &[(DesignLocation, &Layer)],
    at: &DesignLocation,
) -> Result<Layer, Error> {
    if axes.len() > 16 {
        return Err(invalid("interpolation exceeds 16 axes"));
    }
    let mut tags = HashSet::new();
    for axis in axes {
        axis.validate().map_err(invalid)?;
        if !tags.insert(axis.tag) {
            return Err(invalid("duplicate interpolation axis"));
        }
    }
    validate_location(axes, at)?;
    if sources.is_empty() || sources.len() > 1024 {
        return Err(invalid("interpolation requires 1..=1024 sources"));
    }
    let default = sources
        .iter()
        .find(|(loc, _)| {
            axes.iter().all(|a| {
                loc.get(&a.tag)
                    .copied()
                    .unwrap_or_else(|| a.user_to_design(a.default))
                    == a.user_to_design(a.default)
            })
        })
        .map(|(_, layer)| *layer)
        .ok_or_else(|| invalid("glyph has no default source"))?;
    let mut values = HashMap::new();
    let mut budget = 8_000_000usize;
    for (location, layer) in sources {
        validate_location(axes, location)?;
        let size = 3usize
            .saturating_add(
                layer
                    .contours
                    .iter()
                    .map(|c| c.points.len().saturating_mul(2))
                    .sum::<usize>(),
            )
            .saturating_add(
                layer
                    .components
                    .iter()
                    .map(|c| 6 + c.smart.as_ref().map_or(0, |s| s.len()))
                    .sum::<usize>(),
            )
            .saturating_add(layer.anchors.len().saturating_mul(2));
        budget = budget
            .checked_sub(size)
            .ok_or_else(|| invalid("interpolation exceeds coordinate work limit"))?;
        compatible(default, layer).map_err(Error::Incompatible)?;
        if layer.vertical_width.is_some() != default.vertical_width.is_some()
            || layer.vertical_origin.is_some() != default.vertical_origin.is_some()
        {
            return Err(invalid(
                "vertical metrics must be present consistently across sources",
            ));
        }
        let packed = pack_layer(layer, default)?;
        if packed.iter().any(|v| !v.is_finite()) {
            return Err(invalid("source contains non-finite geometry"));
        }
        if values
            .insert(normalized_location(axes, location), packed)
            .is_some()
        {
            return Err(invalid("glyph has duplicate source locations"));
        }
    }
    let model = VariationModel::new(
        values.keys().cloned().collect::<HashSet<_>>(),
        axes.iter().map(|a| Tag::new(&a.tag.0)).collect(),
    );
    let deltas = model
        .deltas_with_rounding::<f64, f64>(&values, RoundingBehaviour::None)
        .map_err(invalid)?;
    let result = model.interpolate_from_deltas(&normalized_location(axes, at), &deltas);
    if result.iter().any(|v| !v.is_finite()) {
        return Err(invalid("interpolation produced non-finite geometry"));
    }
    let mut layer = default.clone();
    unpack_layer(&mut layer, &result)?;
    Ok(layer)
}

fn validate_location(axes: &[Axis], location: &DesignLocation) -> Result<(), Error> {
    for (tag, value) in location {
        let axis = axes
            .iter()
            .find(|a| a.tag == *tag)
            .ok_or_else(|| invalid("unknown interpolation axis"))?;
        if !value.is_finite()
            || *value < axis.user_to_design(axis.min)
            || *value > axis.user_to_design(axis.max)
        {
            return Err(invalid(
                "interpolation coordinate is outside its mapped range",
            ));
        }
    }
    Ok(())
}

/// Interpolate a glyph at a checked design location, including intermediate layers.
pub fn interpolate_glyph(font: &Font, glyph: &Glyph, at: &DesignLocation) -> Result<Layer, Error> {
    if font.extra.lib.contains_key("ufo.designspace.rules") {
        return Err(invalid(
            "preserved native designspace rules require conversion before interpolation",
        ));
    }
    let at = font.complete_location(at).map_err(invalid)?;
    let vertical = glyph
        .layers
        .iter()
        .any(|layer| layer.vertical_width.is_some() || layer.vertical_origin.is_some())
        || font.masters.iter().any(|master| {
            master.metrics.default_vertical_advance.is_some()
                || master.metrics.default_vertical_origin.is_some()
        });
    let resolve = |layer: &Layer| -> Result<Layer, Error> {
        let mut resolved = if crate::effects::has_effects(font, layer) {
            let evaluated = crate::evaluate_layer(font, &glyph.name, &layer.id).map_err(invalid)?;
            let mut baked = layer.clone();
            baked.contours = fc_model::Contour::from_bezpath(&evaluated.path);
            baked.components.clear();
            baked.effects.clear();
            baked.hints = Default::default();
            baked.tt_instructions = None;
            baked.tt_binding = None;
            baked
        } else {
            crate::resolve_smart_layer(font, layer)?
        };
        if vertical {
            let id = match &layer.kind {
                LayerKind::Master(id)
                | LayerKind::Brace { master: id, .. }
                | LayerKind::Bracket { master: id, .. }
                | LayerKind::Smart { master: id, .. } => Some(id),
                _ => None,
            };
            if let Some(master) = id.and_then(|id| font.master(id)) {
                resolved.vertical_width.get_or_insert(
                    master
                        .metrics
                        .default_vertical_advance
                        .unwrap_or(f64::from(font.info.units_per_em)),
                );
                resolved.vertical_origin.get_or_insert(
                    master
                        .metrics
                        .default_vertical_origin
                        .unwrap_or(master.metrics.ascender),
                );
            }
        }
        Ok(resolved)
    };
    if font.axes.is_empty() {
        return glyph
            .master_layer(&font.default_master().id)
            .ok_or_else(|| invalid("glyph has no default source"))
            .and_then(resolve);
    }
    let mut condition = None;
    for layer in &glyph.layers {
        if let LayerKind::Bracket { axis, min, max, .. } = &layer.kind {
            let value = at
                .get(axis)
                .copied()
                .ok_or_else(|| invalid("alternate axis is missing"))?;
            if min.is_none_or(|n| value >= n) && max.is_none_or(|n| value <= n) {
                let key = (*axis, *min, *max);
                if condition.is_some_and(|existing| existing != key) {
                    return Err(invalid("overlapping alternate layer conditions"));
                }
                condition = Some(key);
            }
        }
    }
    let sources = if let Some(condition) = condition {
        glyph
            .layers
            .iter()
            .filter_map(|layer| {
                if let LayerKind::Bracket {
                    master,
                    axis,
                    min,
                    max,
                } = &layer.kind
                {
                    ((*axis, *min, *max) == condition).then(|| {
                        font.master(master)
                            .ok_or_else(|| invalid("alternate master is missing"))
                            .and_then(|m| font.complete_location(&m.location).map_err(invalid))
                            .map(|loc| (loc, layer))
                    })
                } else {
                    None
                }
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        glyph_sources(font, glyph)?
    };
    let resolved = sources
        .into_iter()
        .map(|(location, layer)| resolve(layer).map(|layer| (location, layer)))
        .collect::<Result<Vec<_>, _>>()?;
    let sources = resolved
        .iter()
        .map(|(location, layer)| (location.clone(), layer))
        .collect::<Vec<_>>();
    interpolate_sources(&font.axes, &sources, &at)
}

/// Resolve an interpolated glyph and its components with bounded recursion/work.
pub fn interpolated_path(font: &Font, name: &str, at: &DesignLocation) -> Result<BezPath, Error> {
    if font.extra.lib.contains_key("ufo.designspace.rules") {
        return Err(invalid(
            "preserved native designspace rules require conversion before interpolation",
        ));
    }
    let at = font.complete_location(at).map_err(invalid)?;
    let mut visited = HashSet::new();
    let mut budget = 1_000_000;
    resolve_path(font, name, &at, &mut visited, &mut budget)
}

fn resolve_path(
    font: &Font,
    name: &str,
    at: &DesignLocation,
    visited: &mut HashSet<String>,
    budget: &mut usize,
) -> Result<BezPath, Error> {
    if visited.len() >= 64 || !visited.insert(name.to_owned()) {
        return Err(invalid("component cycle or depth limit"));
    }
    let glyph = font
        .glyph(name)
        .ok_or_else(|| invalid(format!("missing component {name}")))?;
    let layer = interpolate_glyph(font, glyph, at)?;
    let mut path = BezPath::new();
    for contour in &layer.contours {
        let curve = contour.try_to_bezpath().map_err(invalid)?;
        *budget = budget
            .checked_sub(curve.elements().len())
            .ok_or_else(|| invalid("component work limit"))?;
        path.extend(curve);
    }
    for component in &layer.components {
        let base = resolve_path(font, component.base.as_ref(), at, visited, budget)?;
        path.extend(component.transform * base);
    }
    visited.remove(name);
    Ok(path)
}

fn pack_layer(layer: &Layer, template: &Layer) -> Result<Vec<f64>, Error> {
    let mut values = vec![layer.width];
    values.extend(layer.vertical_width);
    values.extend(layer.vertical_origin);
    for contour in &layer.contours {
        for point in &contour.points {
            values.extend([point.x, point.y, point.corner_radius]);
        }
    }
    for (component, reference) in layer.components.iter().zip(&template.components) {
        values.extend(component.transform.as_coeffs());
        let keys: Vec<_> = reference.smart.iter().flat_map(|m| m.keys()).collect();
        if component.smart.as_ref().map_or(0, |m| m.len()) != keys.len() {
            return Err(invalid("smart component parameters differ between sources"));
        }
        for key in keys {
            values.push(
                *component
                    .smart
                    .as_ref()
                    .and_then(|m| m.get(key))
                    .ok_or_else(|| invalid("smart component parameter missing"))?,
            );
        }
    }
    let mut used = HashSet::new();
    for anchor in &template.anchors {
        let (index, source) = layer
            .anchors
            .iter()
            .enumerate()
            .find(|(i, a)| !used.contains(i) && a.name == anchor.name)
            .ok_or_else(|| invalid("anchor is missing"))?;
        used.insert(index);
        values.extend([source.x, source.y]);
    }
    Ok(values)
}

fn unpack_layer(layer: &mut Layer, values: &[f64]) -> Result<(), Error> {
    if values.iter().any(|v| !v.is_finite()) {
        return Err(invalid("interpolation produced non-finite geometry"));
    }
    let mut values = values.iter().copied();
    let mut next = || {
        values
            .next()
            .ok_or_else(|| invalid("interpolation produced incomplete geometry"))
    };
    layer.width = next()?;
    if let Some(value) = layer.vertical_width.as_mut() {
        *value = next()?;
    }
    if let Some(value) = layer.vertical_origin.as_mut() {
        *value = next()?;
    }
    for contour in &mut layer.contours {
        for point in &mut contour.points {
            point.x = next()?;
            point.y = next()?;
            point.corner_radius = next()?;
        }
    }
    for component in &mut layer.components {
        component.transform = Affine::new([next()?, next()?, next()?, next()?, next()?, next()?]);
        if let Some(smart) = component.smart.as_mut() {
            for value in smart.values_mut() {
                *value = next()?;
            }
        }
    }
    for anchor in &mut layer.anchors {
        anchor.x = next()?;
        anchor.y = next()?;
    }
    Ok(())
}
