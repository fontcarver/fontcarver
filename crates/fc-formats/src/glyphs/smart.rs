//! Native smart axes, pole selections and per-use values in Glyphs sources.
use super::{error, plist::*};
use crate::Error;
use fc_model::{Glyph, Layer, LayerKind, MasterId, SmartAxis};
use glyphs_reader::Plist;
use std::collections::BTreeMap;

fn poles(raw: &Plist, version: f64) -> Option<&Plist> {
    if version == 2. {
        raw.get("userData")
            .and_then(|data| data.get("PartSelection"))
    } else {
        raw.get("partSelection")
    }
}
pub(super) fn read_axes(raw: &Plist, glyph: &mut Glyph, version: f64) -> Result<(), Error> {
    glyph.smart_axes.clear();
    let masters: Vec<_> = array(raw, "layers")?
        .iter()
        .filter(|layer| layer.get("associatedMasterId").is_none())
        .collect();
    for value in array(raw, "partsSettings")? {
        let name = required(value, "name")?.to_owned();
        let min = numeric(value, "bottomValue", 0.)?;
        let max = numeric(value, "topValue", 100.)?;
        let mut default = None;
        for master in &masters {
            let pole = poles(master, version)
                .and_then(|p| p.get(&name))
                .map(as_number)
                .transpose()?
                .unwrap_or(1.);
            let position = match pole {
                1. => min,
                2. => max,
                _ => return Err(error("smart poles must be 1 or 2")),
            };
            if default.is_some_and(|old| old != position) {
                return Err(error(
                    "smart default poles must agree across global masters",
                ));
            }
            default = Some(position);
        }
        glyph.smart_axes.push(SmartAxis {
            name,
            min,
            default: default.unwrap_or(min),
            max,
        });
    }
    glyph
        .validate_smart_axes()
        .map_err(|e| error(e.to_string()))
}
pub(super) fn read_layer(
    raw: &Plist,
    glyph: &Glyph,
    master: &MasterId,
    version: f64,
) -> Result<Option<LayerKind>, Error> {
    let Some(poles) = poles(raw, version) else {
        return Ok(None);
    };
    let poles = poles
        .as_dict()
        .ok_or_else(|| error("smart poles must be a dictionary"))?;
    let mut location = BTreeMap::new();
    for (name, pole) in poles {
        let axis = glyph
            .smart_axes
            .iter()
            .find(|a| a.name == name.as_str())
            .ok_or_else(|| error("unknown smart axis in pole selection"))?;
        let value = match as_number(pole)? {
            1. => axis.min,
            2. => axis.max,
            _ => return Err(error("smart poles must be 1 or 2")),
        };
        location.insert(axis.name.clone(), value);
    }
    if raw.get("associatedMasterId").is_none() {
        return Ok(None);
    };
    if raw
        .get("attr")
        .is_some_and(|attr| attr.get("coordinates").is_some() || attr.get("axisRules").is_some())
    {
        return Err(error(
            "a source cannot combine smart poles and global variation conditions",
        ));
    }
    Ok(Some(LayerKind::Smart {
        master: master.clone(),
        location,
    }))
}
pub(super) fn write_axes(raw: &mut Plist, glyph: &Glyph) -> Result<(), Error> {
    glyph
        .validate_smart_axes()
        .map_err(|e| error(e.to_string()))?;
    let mut axes = Vec::new();
    for axis in &glyph.smart_axes {
        if axis.default != axis.min && axis.default != axis.max {
            return Err(error("Glyphs smart defaults must be at a local axis endpoint; use FCF or UFO for interior defaults"));
        }
        let mut entry = dict();
        set(&mut entry, "name", string(&axis.name));
        set(&mut entry, "bottomValue", number(axis.min)?);
        set(&mut entry, "topValue", number(axis.max)?);
        axes.push(entry);
    }
    set(raw, "partsSettings", Plist::Array(axes));
    Ok(())
}
pub(super) fn write_layer(raw: &mut Plist, glyph: &Glyph, layer: &Layer) -> Result<(), Error> {
    remove(raw, &["partSelection"]);
    if let Some(mut data) = raw.get("userData").cloned() {
        remove(&mut data, &["PartSelection"]);
        set(raw, "userData", data);
    }
    if glyph.smart_axes.is_empty() {
        return Ok(());
    }
    let position = match &layer.kind {
        LayerKind::Master(_) => None,
        LayerKind::Smart { location, .. } => Some(location),
        _ => return Ok(()),
    };
    let mut selection = dict();
    for axis in &glyph.smart_axes {
        let value = position
            .and_then(|p| p.get(&axis.name))
            .copied()
            .unwrap_or(axis.default);
        let pole = if value == axis.min {
            1
        } else if value == axis.max {
            2
        } else {
            return Err(error(
                "Glyphs smart sources must use endpoint poles; use FCF or UFO for interior sources",
            ));
        };
        set(&mut selection, &axis.name, integer(pole));
    }
    set(raw, "partSelection", selection);
    Ok(())
}
