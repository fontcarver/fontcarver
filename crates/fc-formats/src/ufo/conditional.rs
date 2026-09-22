//! Native designspace rules and the editable single-axis bracket-layer subset.
// Specification: https://fonttools.readthedocs.io/en/latest/designspaceLib/xml.html#rules-element
use std::collections::{BTreeMap, HashSet};

use fc_model::{Font, GlyphName, LayerId, LayerKind};
use norad::designspace::{Condition, ConditionSet, Rule, RuleProcessing, Rules, Substitution};

use crate::Error;

pub(super) const RULES_KEY: &str = "ufo.designspace.rules";
const GENERATED_KEY: &str = "com.fontcarver.bracketAliases";

fn invalid(message: impl Into<String>) -> Error {
    Error::UnsupportedPlist(message.into())
}

/// Keep unsupported native rules intact; consumers must not silently compile them away.
pub(super) fn read(font: &mut Font, rules: &Rules, lib: &plist::Dictionary) -> Result<(), Error> {
    if rules.rules.is_empty() {
        return Ok(());
    }
    let mut candidate = font.clone();
    match import(&mut candidate, rules, lib) {
        Ok(()) => {
            *font = candidate;
        }
        Err(error) => {
            tracing::warn!(%error, "native designspace rules preserved without conversion");
            font.extra
                .lib
                .insert(RULES_KEY.into(), serde_json::to_value(rules)?);
        }
    }
    Ok(())
}

fn import(font: &mut Font, rules: &Rules, lib: &plist::Dictionary) -> Result<(), Error> {
    if rules.processing != RuleProcessing::First
        || lib
            .get("com.github.fonttools.varLib.featureVarsFeatureTag")
            .is_some_and(|tag| tag.as_string() != Some("rvrn"))
    {
        return Err(invalid(
            "only rules applied through rvrn can be converted to bracket layers",
        ));
    }
    let generated: BTreeMap<String, String> = lib
        .get(GENERATED_KEY)
        .map(|value| {
            let dict = value
                .as_dictionary()
                .ok_or_else(|| invalid("invalid bracket alias metadata"))?;
            dict.iter()
                .map(|(alias, base)| {
                    Ok((
                        alias.clone(),
                        base.as_string()
                            .ok_or_else(|| invalid("invalid bracket alias base"))?
                            .to_owned(),
                    ))
                })
                .collect::<Result<_, Error>>()
        })
        .transpose()?
        .unwrap_or_default();
    let inputs: HashSet<_> = rules
        .rules
        .iter()
        .flat_map(|r| &r.substitutions)
        .map(|s| s.name.as_str())
        .collect();
    if rules
        .rules
        .iter()
        .flat_map(|r| &r.substitutions)
        .any(|s| inputs.contains(s.with.as_str()))
    {
        return Err(invalid(
            "chained native substitutions require an explicit feature source",
        ));
    }
    let mut converted_aliases = HashSet::new();
    let mut additions = Vec::new();
    let mut work = 0usize;
    for rule in &rules.rules {
        for set in &rule.condition_sets {
            let mut axis = None;
            let (mut min, mut max) = (None::<f64>, None::<f64>);
            for condition in &set.conditions {
                let current = font
                    .axes
                    .iter()
                    .find(|axis| axis.name == condition.name)
                    .ok_or_else(|| invalid("native rule refers to an unknown axis"))?;
                if axis.is_some_and(|axis| axis != current.tag) {
                    return Err(invalid(
                        "compound multi-axis native rules cannot be converted to bracket layers",
                    ));
                }
                axis = Some(current.tag);
                if let Some(value) = condition.minimum {
                    min = Some(min.map_or(f64::from(value), |old| old.max(f64::from(value))));
                }
                if let Some(value) = condition.maximum {
                    max = Some(max.map_or(f64::from(value), |old| old.min(f64::from(value))));
                }
            }
            let axis = axis.ok_or_else(|| {
                invalid("unconditional native rules cannot be converted to bracket layers")
            })?;
            for substitution in &rule.substitutions {
                let base = GlyphName::from(substitution.name.as_str());
                if font.glyph(&base).is_none() {
                    return Err(invalid("native substitution source is missing"));
                }
                let alternate = font
                    .glyph(substitution.with.as_str())
                    .ok_or_else(|| invalid("native substitution alternate is missing"))?;
                if alternate
                    .layers
                    .iter()
                    .any(|l| matches!(l.kind, LayerKind::Brace { .. } | LayerKind::Bracket { .. }))
                    || !alternate.smart_axes.is_empty()
                {
                    return Err(invalid(
                        "native alternate requires supplemental source semantics",
                    ));
                }
                if generated
                    .get(substitution.with.as_str())
                    .is_some_and(|name| name == base.as_ref())
                {
                    converted_aliases.insert(substitution.with.as_str().to_owned());
                }
                for source in &alternate.layers {
                    let LayerKind::Master(master) = &source.kind else {
                        continue;
                    };
                    work = work.saturating_add(
                        source
                            .contours
                            .iter()
                            .map(|c| c.points.len())
                            .sum::<usize>()
                            + source.components.len()
                            + 1,
                    );
                    if work > 8_000_000 {
                        return Err(invalid("native conditional source work limit"));
                    }
                    let mut layer = source.clone();
                    layer.id = LayerId::new();
                    layer.kind = LayerKind::Bracket {
                        master: master.clone(),
                        axis,
                        min,
                        max,
                    };
                    layer.name = rule
                        .name
                        .clone()
                        .unwrap_or_else(|| substitution.with.to_string());
                    for component in &mut layer.components {
                        if let Some(original) = generated.get(component.base.as_ref()) {
                            component.base = original.as_str().into();
                        }
                    }
                    additions.push((base.clone(), layer));
                }
            }
        }
    }
    for (base, layer) in additions {
        let glyph = font
            .glyph_mut(&base)
            .ok_or_else(|| invalid("conditional glyph is missing"))?;
        if glyph
            .layers
            .iter()
            .any(|existing| existing.kind == layer.kind)
        {
            return Err(invalid("duplicate native conditional source"));
        }
        glyph.layers.push(layer);
    }
    // Only our explicitly recorded, unencoded implementation glyphs can be removed.
    for name in &converted_aliases {
        let glyph = font
            .glyph(name)
            .ok_or_else(|| invalid("generated alternate is missing"))?;
        if !glyph.unicodes.is_empty()
            || !glyph.variation_selectors.is_empty()
            || font
                .glyphs
                .values()
                .filter(|g| !converted_aliases.contains(g.name.as_ref()))
                .flat_map(|g| &g.layers)
                .flat_map(|l| &l.components)
                .any(|c| c.base.as_ref() == name)
        {
            return Err(invalid(
                "generated alternate has acquired external references",
            ));
        }
    }
    for name in converted_aliases {
        font.remove_glyph(&name);
    }
    font.validate_designspace()
        .map_err(|e| invalid(e.to_string()))?;
    Ok(())
}

pub(super) fn prepare(font: &Font) -> Result<(Font, Rules, plist::Dictionary), Error> {
    let has_brackets = font
        .glyphs
        .values()
        .flat_map(|g| &g.layers)
        .any(|l| matches!(l.kind, LayerKind::Bracket { .. }));
    if !has_brackets {
        let rules = font
            .extra
            .lib
            .get(RULES_KEY)
            .cloned()
            .map(serde_json::from_value)
            .transpose()?
            .unwrap_or_default();
        return Ok((font.clone(), rules, plist::Dictionary::new()));
    }
    if font.extra.lib.contains_key(RULES_KEY) {
        return Err(invalid(
            "resolve preserved native rules before adding bracket layers",
        ));
    }
    let (expanded, regions) =
        fc_model::query::expand_brackets(font).map_err(|e| invalid(e.to_string()))?;
    let mut aliases = plist::Dictionary::new();
    let mut rules = Vec::new();
    for (index, region) in regions.into_iter().enumerate() {
        let axis = font
            .axes
            .iter()
            .find(|a| a.tag == region.axis)
            .ok_or_else(|| invalid("conditional axis is missing"))?;
        let substitutions = region
            .substitutions
            .into_iter()
            .map(|(base, alias)| {
                aliases.insert(alias.to_string(), plist::Value::String(base.to_string()));
                Ok(Substitution {
                    name: norad::Name::new(base.as_ref()).map_err(|e| invalid(e.to_string()))?,
                    with: norad::Name::new(alias.as_ref()).map_err(|e| invalid(e.to_string()))?,
                })
            })
            .collect::<Result<_, Error>>()?;
        rules.push(Rule {
            name: Some(format!("FontCarver bracket {}", index + 1)),
            condition_sets: vec![ConditionSet {
                conditions: vec![Condition {
                    name: axis.name.clone(),
                    minimum: region.min.map(|v| v as f32),
                    maximum: region.max.map(|v| v as f32),
                }],
            }],
            substitutions,
        });
    }
    let mut lib = plist::Dictionary::new();
    lib.insert(GENERATED_KEY.into(), plist::Value::Dictionary(aliases));
    Ok((
        expanded,
        Rules {
            processing: RuleProcessing::First,
            rules,
        },
        lib,
    ))
}
