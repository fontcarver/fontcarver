//! Expand conditional source layers into alternate glyphs and substitution rules.
use crate::{Error, Font, GlyphName, LayerKind, Tag};
use std::collections::BTreeMap;

/// A substitution region in design coordinates, with generated source glyphs.
#[derive(Clone, Debug, PartialEq)]
pub struct BracketRule {
    /// Axis controlling this region.
    pub axis: Tag,
    /// Inclusive lower design bound.
    pub min: Option<f64>,
    /// Inclusive upper design bound.
    pub max: Option<f64>,
    /// Original glyph and generated alternate, including component dependents.
    pub substitutions: BTreeMap<GlyphName, GlyphName>,
}

#[derive(Clone, Copy, PartialEq)]
struct Bounds {
    axis: Tag,
    min: Option<f64>,
    max: Option<f64>,
}

/// Build a compilation copy with explicit alternates, preserving the input document.
/// Overlapping regions, missing default sources and excessive expansion fail explicitly.
pub fn expand_brackets(font: &Font) -> Result<(Font, Vec<BracketRule>), Error> {
    font.validate_designspace()
        .map_err(|e| Error::InvalidDesignSpace(e.to_string()))?;
    let mut groups: Vec<(Bounds, Vec<GlyphName>)> = Vec::new();
    for glyph in font.glyphs.values() {
        let mut seen = Vec::<Bounds>::new();
        for layer in &glyph.layers {
            if let LayerKind::Bracket { axis, min, max, .. } = &layer.kind {
                let bounds = Bounds {
                    axis: *axis,
                    min: *min,
                    max: *max,
                };
                if seen.contains(&bounds) {
                    continue;
                }
                for old in &seen {
                    if old.axis != bounds.axis
                        || old.min.unwrap_or(f64::NEG_INFINITY)
                            <= bounds.max.unwrap_or(f64::INFINITY)
                            && bounds.min.unwrap_or(f64::NEG_INFINITY)
                                <= old.max.unwrap_or(f64::INFINITY)
                    {
                        return Err(Error::InvalidDesignSpace(format!(
                            "{} has overlapping bracket conditions",
                            glyph.name
                        )));
                    }
                }
                seen.push(bounds);
                if let Some((_, names)) = groups.iter_mut().find(|(key, _)| *key == bounds) {
                    names.push(glyph.name.clone());
                } else {
                    groups.push((bounds, vec![glyph.name.clone()]));
                }
            }
        }
    }
    if groups.is_empty() {
        return Ok((font.clone(), Vec::new()));
    }
    let default = &font
        .design_default_master()
        .map_err(|e| Error::InvalidDesignSpace(e.to_string()))?
        .id;
    let mut result = font.clone();
    let mut rules = Vec::new();
    let mut work = 0usize;
    let mut effective_regions = BTreeMap::<GlyphName, Vec<Bounds>>::new();
    for (index, (condition, names)) in groups.into_iter().enumerate() {
        let mut aliases = BTreeMap::<GlyphName, GlyphName>::new();
        let mut pending = names.clone();
        loop {
            for name in pending.drain(..) {
                let mut suffix = index;
                let alias = loop {
                    let candidate = GlyphName::from(format!("{name}.fcBracket{suffix}"));
                    if !result.glyphs.contains_key(&candidate)
                        && !aliases.values().any(|n| *n == candidate)
                    {
                        break candidate;
                    }
                    suffix += 1;
                };
                aliases.insert(name, alias);
            }
            for glyph in font.glyphs.values() {
                if !aliases.contains_key(&glyph.name)
                    && glyph
                        .layers
                        .iter()
                        .filter(|layer| {
                            matches!(
                                layer.kind,
                                LayerKind::Master(_)
                                    | LayerKind::Brace { .. }
                                    | LayerKind::Bracket { .. }
                            )
                        })
                        .any(|layer| {
                            layer
                                .components
                                .iter()
                                .any(|c| aliases.contains_key(&c.base))
                        })
                {
                    pending.push(glyph.name.clone());
                }
            }
            if pending.is_empty() {
                break;
            }
        }
        if result.glyphs.len().saturating_add(aliases.len()) > 65_535 {
            return Err(Error::InvalidDesignSpace(
                "conditional expansion exceeds 65535 glyphs".into(),
            ));
        }
        for base in aliases.keys() {
            let regions = effective_regions.entry(base.clone()).or_default();
            if regions.iter().any(|old| {
                old.axis != condition.axis
                    || old.min.unwrap_or(f64::NEG_INFINITY)
                        <= condition.max.unwrap_or(f64::INFINITY)
                        && condition.min.unwrap_or(f64::NEG_INFINITY)
                            <= old.max.unwrap_or(f64::INFINITY)
            }) {
                return Err(Error::InvalidDesignSpace(format!("{base}: overlapping conditional component dependencies require a combined alternate")));
            }
            regions.push(condition);
        }
        for (base, alias) in &aliases {
            if let Some(glyph) = font.glyph(base) {
                work = work.saturating_add(
                    glyph
                        .layers
                        .iter()
                        .map(|layer| {
                            layer
                                .contours
                                .iter()
                                .map(|contour| contour.points.len())
                                .sum::<usize>()
                                + layer.components.len()
                                + 1
                        })
                        .sum::<usize>(),
                );
                if work > 8_000_000 {
                    return Err(Error::InvalidDesignSpace(
                        "conditional expansion work limit".into(),
                    ));
                }
            }
            let mut glyph = font
                .glyph(base)
                .cloned()
                .ok_or_else(|| Error::InvalidDesignSpace("alternate glyph is missing".into()))?;
            glyph.name = alias.clone();
            glyph.production_name = Some(alias.to_string());
            glyph.unicodes.clear();
            glyph.variation_selectors.clear();
            if names.contains(base) {
                glyph.layers.retain(|layer|matches!(&layer.kind,LayerKind::Bracket{axis,min,max,..} if (*axis,*min,*max)==(condition.axis,condition.min,condition.max)));
                for layer in &mut glyph.layers {
                    if let LayerKind::Bracket { master, .. } = &layer.kind {
                        layer.kind = LayerKind::Master(master.clone());
                    }
                }
                if glyph.master_layer(default).is_none() {
                    return Err(Error::InvalidDesignSpace(format!(
                        "{base}: add a bracket layer for the default master before variable export"
                    )));
                }
            } else {
                glyph
                    .layers
                    .retain(|l| matches!(l.kind, LayerKind::Master(_) | LayerKind::Brace { .. }));
            }
            for layer in &mut glyph.layers {
                for component in &mut layer.components {
                    if let Some(alternate) = aliases.get(&component.base) {
                        component.base = alternate.clone();
                    }
                }
            }
            result
                .insert_glyph(glyph)
                .map_err(|e| Error::InvalidDesignSpace(e.to_string()))?;
        }
        rules.push(BracketRule {
            axis: condition.axis,
            min: condition.min,
            max: condition.max,
            substitutions: aliases,
        });
    }
    for glyph in result.glyphs.values_mut() {
        glyph
            .layers
            .retain(|l| !matches!(l.kind, LayerKind::Bracket { .. }));
    }
    Ok((result, rules))
}
