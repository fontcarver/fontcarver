//! Local smart-component interpolation on a bounded rectangular source grid.
use crate::{Error, Glyph, Layer, LayerKind, MasterId, TtIndex};
use kurbo::Affine;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

/// One local smart-component axis, addressed by name at each component use.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SmartAxis {
    /// Persistent local axis name.
    pub name: String,
    /// Minimum local coordinate.
    pub min: f64,
    /// Coordinate of the ordinary master layer.
    pub default: f64,
    /// Maximum local coordinate.
    pub max: f64,
}
fn invalid(reason: impl Into<String>) -> Error {
    Error::InvalidDesignSpace(reason.into())
}
impl Glyph {
    /// Validate local axis definitions and all local source positions.
    pub fn validate_smart_axes(&self) -> Result<(), Error> {
        if self.smart_axes.len() > 8 {
            return Err(invalid("smart components support at most eight local axes"));
        }
        let mut names = HashSet::new();
        for axis in &self.smart_axes {
            if axis.name.trim().is_empty()
                || !names.insert(&axis.name)
                || [axis.min, axis.default, axis.max]
                    .iter()
                    .any(|n| !n.is_finite() || n.abs() > 1_000_000.)
                || axis.min > axis.default
                || axis.default > axis.max
                || axis.min >= axis.max
            {
                return Err(invalid(
                    "smart axes require unique names and min <= default <= max",
                ));
            }
        }
        let mut positions = HashSet::new();
        for layer in &self.layers {
            let (master, position) = match &layer.kind {
                LayerKind::Master(master) if !self.smart_axes.is_empty() => (
                    master,
                    self.smart_axes.iter().map(|axis| axis.default).collect(),
                ),
                LayerKind::Smart { master, location } => {
                    if self.smart_axes.is_empty() {
                        return Err(invalid("smart sources require local axes"));
                    }
                    (master, coordinates(self, location)?)
                }
                _ => continue,
            };
            let key = position
                .iter()
                .map(|value| if *value == 0. { 0 } else { value.to_bits() })
                .collect::<Vec<_>>();
            if !positions.insert((master, key)) {
                return Err(invalid("duplicate smart source position"));
            }
        }
        Ok(())
    }
}
fn coordinates(glyph: &Glyph, location: &BTreeMap<String, f64>) -> Result<Vec<f64>, Error> {
    if location
        .keys()
        .any(|name| !glyph.smart_axes.iter().any(|a| a.name == *name))
    {
        return Err(invalid("unknown smart-component axis"));
    }
    glyph
        .smart_axes
        .iter()
        .map(|axis| {
            let value = location.get(&axis.name).copied().unwrap_or(axis.default);
            if !value.is_finite() || value < axis.min || value > axis.max {
                return Err(invalid(format!(
                    "smart value for {} is outside its range",
                    axis.name
                )));
            }
            Ok(value)
        })
        .collect()
}

/// Resolve local parameters at one global master using multilinear interpolation.
/// Neighboring grid corners must exist and have matching outline topology.
pub fn smart_layer(
    glyph: &Glyph,
    master: &MasterId,
    location: &BTreeMap<String, f64>,
) -> Result<Layer, Error> {
    glyph.validate_smart_axes()?;
    let at = coordinates(glyph, location)?;
    let base = glyph
        .master_layer(master)
        .ok_or_else(|| Error::MissingMasterLayer {
            glyph: glyph.name.clone(),
            master: master.clone(),
        })?;
    if glyph.smart_axes.is_empty() {
        return Ok(base.clone());
    }
    let mut sources = vec![(
        glyph
            .smart_axes
            .iter()
            .map(|a| a.default)
            .collect::<Vec<_>>(),
        base,
    )];
    for layer in &glyph.layers {
        if let LayerKind::Smart {
            master: id,
            location,
        } = &layer.kind
        {
            if id == master {
                sources.push((coordinates(glyph, location)?, layer));
            }
        }
    }
    if sources.len() > 1024 {
        return Err(invalid("smart component exceeds 1024 sources"));
    }
    let key = |position: &[f64]| {
        position
            .iter()
            .map(|n| if *n == 0. { 0 } else { n.to_bits() })
            .collect::<Vec<_>>()
    };
    let mut positions = HashSet::new();
    for (position, _) in &sources {
        if !positions.insert(key(position)) {
            return Err(invalid("duplicate smart source position"));
        }
    }
    let mut corners = vec![(Vec::new(), 1.)];
    for (index, value) in at.iter().copied().enumerate() {
        let low = sources
            .iter()
            .map(|(p, _)| p[index])
            .filter(|n| *n <= value)
            .max_by(f64::total_cmp)
            .ok_or_else(|| invalid("smart source grid does not cover the requested value"))?;
        let high = sources
            .iter()
            .map(|(p, _)| p[index])
            .filter(|n| *n >= value)
            .min_by(f64::total_cmp)
            .ok_or_else(|| invalid("smart source grid does not cover the requested value"))?;
        let mut next = Vec::new();
        for (position, weight) in corners {
            let mut lower = position.clone();
            lower.push(low);
            next.push((
                lower,
                weight
                    * if low == high {
                        1.
                    } else {
                        (high - value) / (high - low)
                    },
            ));
            if low != high {
                let mut upper = position;
                upper.push(high);
                next.push((upper, weight * (value - low) / (high - low)));
            }
        }
        corners = next;
    }
    let layers = corners
        .iter()
        .map(|(position, weight)| {
            let source = sources
                .iter()
                .find(|(p, _)| p == position)
                .ok_or_else(|| invalid("smart component is missing a required grid corner"))?;
            Ok((source.1, *weight))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    blend(base, &layers)
}

fn blend(template: &Layer, layers: &[(&Layer, f64)]) -> Result<Layer, Error> {
    let points: usize = template.contours.iter().map(|c| c.points.len()).sum();
    if points
        .saturating_add(template.components.len())
        .saturating_mul(layers.len())
        > 1_000_000
    {
        return Err(invalid("smart interpolation exceeds geometry work limit"));
    }
    for (layer, _) in layers {
        if layer.contours.len() != template.contours.len()
            || layer.components.len() != template.components.len()
            || layer.anchors.len() != template.anchors.len()
            || layer.vertical_width.is_some() != template.vertical_width.is_some()
            || layer.vertical_origin.is_some() != template.vertical_origin.is_some()
        {
            return Err(invalid("incompatible smart source topology or metrics"));
        }
        for (a, b) in template.contours.iter().zip(&layer.contours) {
            if a.closed != b.closed
                || a.points.len() != b.points.len()
                || a.points
                    .iter()
                    .zip(&b.points)
                    .any(|(a, b)| a.kind != b.kind)
            {
                return Err(invalid("incompatible smart source points"));
            }
        }
        for (a, b) in template.components.iter().zip(&layer.components) {
            if a.base != b.base
                || a.smart.as_ref().map(|m| m.keys().collect::<Vec<_>>())
                    != b.smart.as_ref().map(|m| m.keys().collect::<Vec<_>>())
            {
                return Err(invalid("incompatible nested smart components"));
            }
        }
        if template
            .anchors
            .iter()
            .any(|a| layer.anchors.iter().filter(|b| b.name == a.name).count() != 1)
        {
            return Err(invalid("incompatible smart source anchors"));
        }
    }
    let value = |get: &dyn Fn(&Layer) -> f64| -> Result<f64, Error> {
        let mut result = 0.;
        for (layer, weight) in layers {
            let v = get(layer);
            if !v.is_finite() {
                return Err(invalid("nonfinite smart geometry"));
            }
            result += weight * v;
        }
        if !result.is_finite() {
            return Err(invalid("nonfinite smart interpolation"));
        }
        Ok(result)
    };
    let mut result = template.clone();
    result.width = value(&|l| l.width)?;
    if result.vertical_width.is_some() {
        result.vertical_width = Some(value(&|l| l.vertical_width.unwrap_or_default())?);
    }
    if result.vertical_origin.is_some() {
        result.vertical_origin = Some(value(&|l| l.vertical_origin.unwrap_or_default())?);
    }
    for (ci, c) in result.contours.iter_mut().enumerate() {
        for (pi, p) in c.points.iter_mut().enumerate() {
            p.x = value(&|l| l.contours[ci].points[pi].x)?;
            p.y = value(&|l| l.contours[ci].points[pi].y)?;
            p.tt_index = TtIndex::Unnumbered;
        }
    }
    for (i, c) in result.components.iter_mut().enumerate() {
        let mut transform = [0.; 6];
        for (j, v) in transform.iter_mut().enumerate() {
            *v = value(&|l| l.components[i].transform.as_coeffs()[j])?;
        }
        c.transform = Affine::new(transform);
        if let Some(parameters) = &mut c.smart {
            for (name, v) in parameters {
                *v = value(&|l| {
                    l.components[i]
                        .smart
                        .as_ref()
                        .and_then(|m| m.get(name))
                        .copied()
                        .unwrap_or_default()
                })?;
            }
        }
    }
    for anchor in &mut result.anchors {
        anchor.x = value(&|l| {
            l.anchors
                .iter()
                .find(|a| a.name == anchor.name)
                .map_or(0., |a| a.x)
        })?;
        anchor.y = value(&|l| {
            l.anchors
                .iter()
                .find(|a| a.name == anchor.name)
                .map_or(0., |a| a.y)
        })?;
    }
    result.tt_instructions = None;
    result.tt_binding = None;
    result.hints = Default::default();
    Ok(result)
}
