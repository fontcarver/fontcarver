//! Expand conditional source layers into alternate glyphs and substitution rules.
use crate::{BracketCondition, Error, Font, GlyphName, LayerKind};
use std::collections::BTreeMap;

/// A substitution region in design coordinates, with generated source glyphs.
#[derive(Clone, Debug, PartialEq)]
pub struct BracketRule {
    /// Conjunction of per-axis ranges that activates this region.
    pub conditions: Vec<BracketCondition>,
    /// Original glyph and generated alternate, including component dependents.
    pub substitutions: BTreeMap<GlyphName, GlyphName>,
}

/// A canonical condition set: sorted by axis tag, so equality identifies a region.
type Bounds = Vec<BracketCondition>;

/// Two condition sets overlap when every shared axis range intersects and no
/// axis present in only one of them excludes the other.
///
/// An axis a set does not constrain is unbounded in that set, so two sets are
/// disjoint exactly when some axis range fails to intersect.
fn overlapping(left: &[BracketCondition], right: &[BracketCondition]) -> bool {
    left.iter().all(|condition| {
        right
            .iter()
            .find(|other| other.axis == condition.axis)
            .is_none_or(|other| condition.overlaps(other))
    })
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
            if let Some(bounds) = layer.kind.bracket_conditions() {
                if seen.contains(&bounds) {
                    continue;
                }
                for old in &seen {
                    if overlapping(old, &bounds) {
                        return Err(Error::InvalidDesignSpace(format!(
                            "{} has overlapping bracket conditions",
                            glyph.name
                        )));
                    }
                }
                seen.push(bounds.clone());
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
            if regions.iter().any(|old| overlapping(old, &condition)) {
                return Err(Error::InvalidDesignSpace(format!("{base}: overlapping conditional component dependencies require a combined alternate")));
            }
            regions.push(condition.clone());
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
                glyph.layers.retain(|layer| {
                    layer.kind.bracket_conditions().as_deref() == Some(condition.as_slice())
                });
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
            conditions: condition,
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
