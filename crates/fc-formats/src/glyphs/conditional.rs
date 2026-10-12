//! Native Glyphs conditional-layer attributes, with explicit representability checks.
use super::{error, plist::*};
use crate::Error;
use fc_model::{BracketCondition, Font, Layer, LayerKind, MasterId};
use glyphs_reader::Plist;

pub(super) fn read(
    raw: &Plist,
    font: &Font,
    master: &MasterId,
    version: f64,
) -> Result<Option<LayerKind>, Error> {
    if let Some(rules) = raw.get("attr").and_then(|attr| attr.get("axisRules")) {
        let rules = rules
            .as_array()
            .ok_or_else(|| error("axisRules must be an array"))?;
        if rules.len() != font.axes.len() {
            return Err(error("conditional rules do not match the font axes"));
        }
        // Glyphs stores one rule per axis; several bounded axes form the
        // compound condition `[80<wght, 25<wdth<50]`.
        let mut ranges = Vec::new();
        for (rule, axis) in rules.iter().zip(&font.axes) {
            if rule.as_dict().is_none() {
                return Err(error("conditional axis rule must be a dictionary"));
            }
            let min = optional_number(rule, "min")?;
            let max = optional_number(rule, "max")?;
            if min.is_some() || max.is_some() {
                ranges.push(BracketCondition {
                    axis: axis.tag,
                    min,
                    max,
                });
            }
        }
        if ranges.is_empty() {
            return Err(error("conditional layer has no axis bounds"));
        }
        let condition = Some(
            LayerKind::from_bracket_conditions(master.clone(), &ranges)
                .map_err(|failure| error(failure.to_string()))?,
        );
        if raw
            .get("attr")
            .and_then(|attr| attr.get("coordinates"))
            .is_some()
        {
            return Err(error("a layer cannot be both intermediate and conditional"));
        }
        return Ok(condition);
    }
    if version != 2.0 {
        return Ok(None);
    }
    let name = text(raw, "name").unwrap_or("");
    let Some((head, tail)) = name.rsplit_once(']') else {
        return Ok(None);
    };
    if !tail.trim().is_empty() {
        return Ok(None);
    }
    let condition = head
        .rsplit_once('[')
        .map(|(_, value)| (true, value))
        .or_else(|| head.rsplit_once(']').map(|(_, value)| (false, value)));
    let Some((lower, value)) = condition else {
        return Ok(None);
    };
    let value = value
        .trim()
        .parse::<f64>()
        .map_err(|_| error("invalid Glyphs 2 bracket bound"))?;
    if !value.is_finite() {
        return Err(error("nonfinite bracket bound"));
    }
    let axis = font
        .axes
        .first()
        .ok_or_else(|| error("bracket layer requires an axis"))?;
    Ok(Some(LayerKind::Bracket {
        master: master.clone(),
        axis: axis.tag,
        min: lower.then_some(value),
        max: (!lower).then_some(value),
        extra: Vec::new(),
    }))
}

pub(super) fn write(raw: &mut Plist, font: &Font, layer: &Layer) -> Result<(), Error> {
    let mut attr = raw.get("attr").cloned().unwrap_or_else(dict);
    remove(&mut attr, &["coordinates", "axisRules"]);
    match &layer.kind {
        LayerKind::Brace { master, location } => {
            let location = font
                .intermediate_location(master, location)
                .map_err(|e| error(e.to_string()))?;
            set(
                &mut attr,
                "coordinates",
                numbers(font.axes.iter().map(|a| location[&a.tag]))?,
            );
        }
        LayerKind::Bracket { .. } => {
            let conditions = layer
                .kind
                .bracket_conditions()
                .ok_or_else(|| error("invalid conditional layer axis or bounds"))?;
            if conditions.iter().any(|condition| {
                !font.axes.iter().any(|a| a.tag == condition.axis)
                    || condition.min.zip(condition.max).is_some_and(|(a, b)| a > b)
            }) || conditions
                .iter()
                .all(|condition| condition.min.is_none() && condition.max.is_none())
            {
                return Err(error("invalid conditional layer axis or bounds"));
            }
            let rules = font
                .axes
                .iter()
                .map(|a| {
                    let mut rule = dict();
                    if let Some(condition) =
                        conditions.iter().find(|condition| condition.axis == a.tag)
                    {
                        if let Some(value) = condition.min {
                            set(&mut rule, "min", number(value)?);
                        }
                        if let Some(value) = condition.max {
                            set(&mut rule, "max", number(value)?);
                        }
                    }
                    Ok(rule)
                })
                .collect::<Result<Vec<_>, Error>>()?;
            set(&mut attr, "axisRules", Plist::Array(rules));
        }
        _ => {}
    }
    set(raw, "attr", attr);
    Ok(())
}
