//! Resolution of glyph metrics formulas.

use std::collections::HashSet;

use fw_model::{Font, GlyphName, MasterId};
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

/// Resolves `=name`, mirrored, offset, and literal metrics keys recursively.
pub fn resolve(
    font: &Font,
    glyph: &GlyphName,
    master: &MasterId,
) -> Result<ResolvedMetrics, Error> {
    let glyph_data = font
        .glyph(glyph)
        .ok_or_else(|| Error::MissingGlyph(glyph.clone()))?;
    let source = if glyph_data.metrics_keys.left.is_some()
        || glyph_data.metrics_keys.right.is_some()
        || glyph_data.metrics_keys.width.is_some()
    {
        MetricsSource::Key
    } else {
        MetricsSource::Explicit
    };
    let mut visited = HashSet::new();
    let lsb = resolve_metric(font, glyph, master, Metric::Left, &mut visited)?;
    visited.clear();
    let rsb = resolve_metric(font, glyph, master, Metric::Right, &mut visited)?;
    visited.clear();
    let width = resolve_metric(font, glyph, master, Metric::Width, &mut visited)?;
    Ok(ResolvedMetrics {
        lsb,
        rsb,
        width,
        source,
    })
}

fn resolve_metric(
    font: &Font,
    glyph: &GlyphName,
    master: &MasterId,
    metric: Metric,
    visited: &mut HashSet<(GlyphName, Metric)>,
) -> Result<f64, Error> {
    if !visited.insert((glyph.clone(), metric)) {
        return Err(Error::MetricsKeyCycle);
    }
    let glyph_data = font
        .glyph(glyph)
        .ok_or_else(|| Error::MissingGlyph(glyph.clone()))?;
    let key = match metric {
        Metric::Left => &glyph_data.metrics_keys.left,
        Metric::Right => &glyph_data.metrics_keys.right,
        Metric::Width => &glyph_data.metrics_keys.width,
    };
    let result = if let Some(key) = key {
        match parse_key(key)? {
            Key::Literal(value) => value,
            Key::Reference {
                glyph: reference,
                mirrored,
                offset,
            } => {
                let referenced_metric = if mirrored {
                    match metric {
                        Metric::Left => Metric::Right,
                        Metric::Right => Metric::Left,
                        Metric::Width => Metric::Width,
                    }
                } else {
                    metric
                };
                resolve_metric(font, &reference, master, referenced_metric, visited)? + offset
            }
        }
    } else {
        explicit_metric(font, glyph, master, metric)?
    };
    visited.remove(&(glyph.clone(), metric));
    Ok(result)
}

fn explicit_metric(
    font: &Font,
    glyph: &GlyphName,
    master: &MasterId,
    metric: Metric,
) -> Result<f64, Error> {
    let glyph_data = font
        .glyph(glyph)
        .ok_or_else(|| Error::MissingGlyph(glyph.clone()))?;
    let layer = glyph_data.master_layer(master).ok_or_else(|| {
        Error::InvalidCommand(format!(
            "glyph `{glyph}` has no layer for master `{master}`"
        ))
    })?;
    let bounds = fw_model::layer_bbox(font, glyph, master)?;
    Ok(match metric {
        Metric::Left => bounds.map_or(0.0, |bounds| bounds.x0),
        Metric::Right => bounds.map_or(layer.width, |bounds| layer.width - bounds.x1),
        Metric::Width => layer.width,
    })
}

enum Key {
    Literal(f64),
    Reference {
        glyph: GlyphName,
        mirrored: bool,
        offset: f64,
    },
}

fn parse_key(key: &str) -> Result<Key, Error> {
    let body = key
        .trim()
        .strip_prefix('=')
        .ok_or_else(|| Error::InvalidCommand(format!("invalid metrics key `{key}`")))?;
    if let Ok(value) = body.parse::<f64>() {
        return finite_literal(value, key);
    }
    let (mirrored, body) = body
        .strip_prefix('|')
        .map_or((false, body), |body| (true, body));
    let (name, offset) = split_offset(body);
    if name.is_empty() {
        return Err(Error::InvalidCommand(format!(
            "invalid metrics key `{key}`"
        )));
    }
    Ok(Key::Reference {
        glyph: GlyphName::from(name),
        mirrored,
        offset,
    })
}

fn finite_literal(value: f64, key: &str) -> Result<Key, Error> {
    if value.is_finite() {
        Ok(Key::Literal(value))
    } else {
        Err(Error::InvalidCommand(format!(
            "invalid metrics key `{key}`"
        )))
    }
}

fn split_offset(body: &str) -> (&str, f64) {
    for (index, character) in body.char_indices().rev() {
        if index > 0 && matches!(character, '+' | '-') {
            if let Ok(value) = body[index..].parse::<f64>() {
                if value.is_finite() {
                    return (&body[..index], value);
                }
            }
        }
    }
    (body, 0.0)
}
