//! Resolution of glyph and layer-local horizontal metrics formulas.

use std::collections::HashSet;

use fc_model::{Font, GlyphName, Layer, LayerId, MasterId, MetricsKeys};
use kurbo::{Line, ParamCurve, Shape};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::Error;

/// Whether resolved metrics came exclusively from stored values or used a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum MetricsSource {
    /// No horizontal metrics formula was present.
    Explicit,
    /// At least one horizontal metrics formula was resolved.
    Key,
}

/// Fully resolved horizontal metrics for one glyph and master.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct ResolvedMetrics {
    /// Left sidebearing in font units.
    pub lsb: f64,
    /// Right sidebearing in font units.
    pub rsb: f64,
    /// Horizontal advance width in font units.
    pub width: f64,
    /// Whether formulas contributed to the values.
    pub source: MetricsSource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Metric {
    Left,
    Right,
    Width,
}

type Visits = HashSet<(GlyphName, Option<LayerId>, Metric)>;

/// Resolves formulas, using local overrides from the glyph's selected master layer.
pub fn resolve(
    font: &Font,
    glyph: &GlyphName,
    master: &MasterId,
) -> Result<ResolvedMetrics, Error> {
    resolve_inner(font, glyph, master, None)
}

/// Resolves formulas for a specific layer, including its local overrides.
pub fn resolve_layer(
    font: &Font,
    glyph: &GlyphName,
    master: &MasterId,
    layer: &LayerId,
) -> Result<ResolvedMetrics, Error> {
    resolve_inner(font, glyph, master, Some(layer))
}

fn resolve_inner(
    font: &Font,
    glyph: &GlyphName,
    master: &MasterId,
    layer: Option<&LayerId>,
) -> Result<ResolvedMetrics, Error> {
    let keys = effective_keys(font, glyph, master, layer)?;
    let source = if keys.left.is_some() || keys.right.is_some() || keys.width.is_some() {
        MetricsSource::Key
    } else {
        MetricsSource::Explicit
    };
    let mut visited = HashSet::new();
    Ok(ResolvedMetrics {
        lsb: resolve_metric(font, glyph, master, layer, Metric::Left, &mut visited)?,
        rsb: resolve_metric(font, glyph, master, layer, Metric::Right, &mut visited)?,
        width: resolve_metric(font, glyph, master, layer, Metric::Width, &mut visited)?,
        source,
    })
}

/// Returns glyph formulas overlaid with the selected layer's nonempty fields.
pub fn effective_keys(
    font: &Font,
    glyph: &GlyphName,
    master: &MasterId,
    layer: Option<&LayerId>,
) -> Result<MetricsKeys, Error> {
    let data = font
        .glyph(glyph)
        .ok_or_else(|| Error::MissingGlyph(glyph.clone()))?;
    let selected = match layer {
        Some(id) => Some(
            data.layer(id)
                .ok_or_else(|| Error::InvalidCommand(format!("missing layer `{id}`")))?,
        ),
        None => data.master_layer(master),
    };
    let mut keys = data.metrics_keys.clone();
    if let Some(local) = selected.and_then(|layer| layer.metrics_keys.as_ref()) {
        for (global, local) in [
            (&mut keys.left, &local.left),
            (&mut keys.right, &local.right),
            (&mut keys.width, &local.width),
            (&mut keys.vertical_origin, &local.vertical_origin),
            (&mut keys.vertical_width, &local.vertical_width),
        ] {
            if local.is_some() {
                *global = local.clone();
            }
        }
    }
    Ok(keys)
}

fn selected_layer<'a>(
    font: &'a Font,
    glyph: &GlyphName,
    master: &MasterId,
    layer: Option<&LayerId>,
) -> Result<&'a Layer, Error> {
    let data = font
        .glyph(glyph)
        .ok_or_else(|| Error::MissingGlyph(glyph.clone()))?;
    layer
        .map_or_else(|| data.master_layer(master), |id| data.layer(id))
        .ok_or_else(|| {
            Error::InvalidCommand(format!(
                "glyph `{glyph}` has no selected layer for master `{master}`"
            ))
        })
}

fn resolve_metric(
    font: &Font,
    glyph: &GlyphName,
    master: &MasterId,
    layer: Option<&LayerId>,
    metric: Metric,
    visited: &mut Visits,
) -> Result<f64, Error> {
    let identity = (glyph.clone(), layer.cloned(), metric);
    // A finite chain limit also protects against maliciously deep acyclic keys.
    if visited.len() >= 128 || !visited.insert(identity.clone()) {
        return Err(Error::MetricsKeyCycle);
    }
    let keys = effective_keys(font, glyph, master, layer)?;
    let key = match metric {
        Metric::Left => &keys.left,
        Metric::Right => &keys.right,
        Metric::Width => &keys.width,
    };
    let result = if let Some(key) = key {
        match parse_key(key)? {
            Key::Literal(value) => value,
            Key::Automatic(offset) => {
                automatic_metric(font, glyph, master, layer, metric, visited)? + offset
            }
            Key::Reference {
                glyph: reference,
                mirrored,
                operation,
                height,
            } => {
                let reference = reference.unwrap_or_else(|| glyph.clone());
                let reference_layer = (&reference == glyph).then_some(layer).flatten();
                let referenced_metric = if mirrored {
                    match metric {
                        Metric::Left => Metric::Right,
                        Metric::Right => Metric::Left,
                        Metric::Width => Metric::Width,
                    }
                } else {
                    metric
                };
                let value = if let Some(height) = height {
                    explicit_metric(
                        font,
                        &reference,
                        master,
                        reference_layer,
                        referenced_metric,
                        Some(height),
                    )?
                } else {
                    resolve_metric(
                        font,
                        &reference,
                        master,
                        reference_layer,
                        referenced_metric,
                        visited,
                    )?
                };
                operation.apply(value)
            }
        }
    } else {
        // Keep unkeyed bearings stable. A width key instead fixes the advance,
        // leaving the unkeyed opposite bearing to absorb a keyed bearing change.
        match metric {
            Metric::Width if keys.left.is_some() || keys.right.is_some() => {
                ink_width(font, glyph, master, layer)?
                    + resolve_metric(font, glyph, master, layer, Metric::Left, visited)?
                    + resolve_metric(font, glyph, master, layer, Metric::Right, visited)?
            }
            Metric::Left if keys.width.is_some() && keys.right.is_some() => {
                resolve_metric(font, glyph, master, layer, Metric::Width, visited)?
                    - ink_width(font, glyph, master, layer)?
                    - resolve_metric(font, glyph, master, layer, Metric::Right, visited)?
            }
            Metric::Right if keys.width.is_some() => {
                resolve_metric(font, glyph, master, layer, Metric::Width, visited)?
                    - ink_width(font, glyph, master, layer)?
                    - resolve_metric(font, glyph, master, layer, Metric::Left, visited)?
            }
            _ => explicit_metric(font, glyph, master, layer, metric, None)?,
        }
    };
    visited.remove(&identity);
    if !result.is_finite() {
        return Err(Error::InvalidCommand(
            "metrics expression produced a non-finite value".into(),
        ));
    }
    Ok(result)
}

fn ink_width(
    font: &Font,
    glyph: &GlyphName,
    master: &MasterId,
    layer: Option<&LayerId>,
) -> Result<f64, Error> {
    let selected = selected_layer(font, glyph, master, layer)?;
    let path = fc_geom::flatten_layer_by_id(font, glyph, &selected.id)?;
    Ok(if path.is_empty() {
        0.0
    } else {
        path.bounding_box().width()
    })
}

fn explicit_metric(
    font: &Font,
    glyph: &GlyphName,
    master: &MasterId,
    layer: Option<&LayerId>,
    metric: Metric,
    height: Option<f64>,
) -> Result<f64, Error> {
    let selected = selected_layer(font, glyph, master, layer)?;
    if metric == Metric::Width {
        if height.is_some() {
            return Err(Error::InvalidCommand(
                "height-qualified metrics keys require a sidebearing".into(),
            ));
        }
        return Ok(selected.width);
    }
    let path = fc_geom::flatten_layer_by_id(font, glyph, &selected.id)?;
    let mut bounds = (!path.is_empty()).then(|| path.bounding_box());
    if let Some(height) = height {
        let Some(bbox) = bounds else {
            return Err(Error::InvalidCommand(
                "metrics measurement height does not intersect the outline".into(),
            ));
        };
        let line = Line::new((bbox.x0 - 1.0, height), (bbox.x1 + 1.0, height));
        let mut xs = Vec::new();
        for segment in path.segments() {
            // Including endpoints handles horizontal segments and extrema exactly.
            for point in [segment.start(), segment.end()] {
                if (point.y - height).abs() < 1e-8 {
                    xs.push(point.x);
                }
            }
            for hit in segment.intersect_line(line) {
                xs.push(segment.eval(hit.segment_t).x);
            }
        }
        let min = xs.iter().copied().reduce(f64::min);
        let max = xs.iter().copied().reduce(f64::max);
        bounds = match (min, max) {
            (Some(min), Some(max)) => Some(kurbo::Rect::new(min, height, max, height)),
            _ => {
                return Err(Error::InvalidCommand(
                    "metrics measurement height does not intersect the outline".into(),
                ))
            }
        };
    }
    Ok(match metric {
        Metric::Left => bounds.map_or(0.0, |bounds| bounds.x0),
        Metric::Right => bounds.map_or(selected.width, |bounds| selected.width - bounds.x1),
        Metric::Width => selected.width,
    })
}

fn automatic_metric(
    font: &Font,
    glyph: &GlyphName,
    master: &MasterId,
    layer: Option<&LayerId>,
    metric: Metric,
    visited: &mut Visits,
) -> Result<f64, Error> {
    let selected = selected_layer(font, glyph, master, layer)?;
    let base = selected
        .components
        .iter()
        .find(|component| component.auto_align && component.use_my_metrics)
        .or_else(|| {
            selected
                .components
                .iter()
                .find(|component| component.auto_align)
        })
        .filter(|_| selected.contours.is_empty())
        .ok_or_else(|| {
            Error::InvalidCommand(
                "relative metrics keys require an automatically aligned component glyph".into(),
            )
        })?;
    let [xx, xy, yx, _, _, _] = base.transform.as_coeffs();
    if xx <= 0.0 || xy.abs() > 1e-8 || yx.abs() > 1e-8 {
        return Err(Error::InvalidCommand(
            "automatic metrics require an upright, positively scaled base component".into(),
        ));
    }
    Ok(resolve_metric(font, &base.base, master, None, metric, visited)? * xx)
}

#[derive(Clone, Copy)]
enum Operation {
    None,
    Add(f64),
    Multiply(f64),
    Divide(f64),
}
impl Operation {
    fn apply(self, value: f64) -> f64 {
        match self {
            Self::None => value,
            Self::Add(number) => value + number,
            Self::Multiply(number) => value * number,
            Self::Divide(number) => value / number,
        }
    }
}

enum Key {
    Literal(f64),
    Automatic(f64),
    Reference {
        glyph: Option<GlyphName>,
        mirrored: bool,
        operation: Operation,
        height: Option<f64>,
    },
}

fn invalid_key(key: &str) -> Error {
    Error::InvalidCommand(format!("invalid metrics key `{key}`"))
}

fn parse_key(key: &str) -> Result<Key, Error> {
    let body = key
        .trim()
        .strip_prefix('=')
        .ok_or_else(|| invalid_key(key))?;
    let body = body.strip_prefix('=').unwrap_or(body).trim();
    if body.starts_with(['+', '-']) {
        let value = body.parse::<f64>().map_err(|_| invalid_key(key))?;
        if !value.is_finite() {
            return Err(invalid_key(key));
        }
        return Ok(Key::Automatic(value));
    }
    if let Ok(value) = body.parse::<f64>() {
        return if value.is_finite() {
            Ok(Key::Literal(value))
        } else {
            Err(invalid_key(key))
        };
    }
    let (body, height) = match body.rsplit_once('@') {
        Some((body, height)) => {
            let height = height.parse::<f64>().map_err(|_| invalid_key(key))?;
            if !height.is_finite() {
                return Err(invalid_key(key));
            }
            (body, Some(height))
        }
        None => (body, None),
    };
    let (mirrored, body) = body
        .strip_prefix('|')
        .map_or((false, body), |body| (true, body));
    let (name, operation) = split_operation(body, key)?;
    if (name.is_empty() && !mirrored)
        || name
            .chars()
            .any(|ch| ch.is_whitespace() || matches!(ch, '=' | '|' | '@' | '*' | '/' | '+'))
    {
        return Err(invalid_key(key));
    }
    Ok(Key::Reference {
        glyph: (!name.is_empty()).then(|| GlyphName::from(name)),
        mirrored,
        operation,
        height,
    })
}

fn split_operation<'a>(body: &'a str, key: &str) -> Result<(&'a str, Operation), Error> {
    for (index, character) in body.char_indices().rev() {
        if index > 0 && matches!(character, '+' | '-' | '*' | '/') {
            if let Ok(value) = body[index + 1..].parse::<f64>() {
                if !value.is_finite() || (character == '/' && value == 0.0) {
                    return Err(invalid_key(key));
                }
                let operation = match character {
                    '+' => Operation::Add(value),
                    '-' => Operation::Add(-value),
                    '*' => Operation::Multiply(value),
                    '/' => Operation::Divide(value),
                    _ => Operation::None,
                };
                return Ok((&body[..index], operation));
            }
        }
    }
    Ok((body, Operation::None))
}

/// Validates formula syntax and prevents local keys from leaking into glyph scope.
pub(crate) fn validate_keys(keys: &MetricsKeys, local: bool) -> Result<(), Error> {
    for key in [&keys.left, &keys.right, &keys.width].into_iter().flatten() {
        if !local && key.trim().starts_with("==") {
            return Err(Error::InvalidCommand(
                "local metrics keys must be assigned to a layer".into(),
            ));
        }
        parse_key(key)?;
    }
    Ok(())
}
