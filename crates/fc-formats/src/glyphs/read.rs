//! Native field conversion; stored editor metadata never overrides native fields.

use super::{error, extras, metadata, outlines, plist::*};
use crate::Error;
use fc_model::{
    Axis, FeatureBlock, FeatureClass, Font, Glyph, Instance, KernSide, Kerning, Layer, LayerKind,
    Master, MasterId, Tag,
};
use glyphs_reader::Plist;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn font(raw: &Plist) -> Result<Font, Error> {
    let version = numeric(raw, ".formatVersion", 2.)?;
    if version != 2. && version != 3. {
        return Err(error("only Glyphs 2 and 3 sources are supported"));
    }
    let raw_masters = array(raw, "fontMaster")?;
    if raw_masters.is_empty() || raw_masters.len() > 256 {
        return Err(error("source needs 1–256 masters"));
    }
    let mut ids = BTreeSet::new();
    for master in raw_masters {
        if !ids.insert(required(master, "id")?) {
            return Err(error("duplicate master ID"));
        }
    }
    check_supported(raw)?;
    // The upstream normalizer contains assertions on malformed historical data.
    // Bound parsing first, then convert any assertion into an actionable error.
    let native =
        std::panic::catch_unwind(|| glyphs_reader::Font::load_from_string(&serialize(raw)))
            .map_err(|_| error("malformed source rejected by Glyphs normalization"))?
            .map_err(|err| error(format!("invalid Glyphs source: {err:?}")))?;
    let mut font: Font = extras::restore(raw, extras::FONT)?;
    font.info.family_name = native
        .get_default_name("familyNames")
        .or_else(|| text(raw, "familyName"))
        .unwrap_or("Untitled")
        .into();
    let upem = numeric(raw, "unitsPerEm", 1000.)?;
    if !(1. ..=65535.).contains(&upem) || upem.fract() != 0. {
        return Err(error("unitsPerEm must be an integer between 1 and 65535"));
    }
    font.info.units_per_em = upem as u16;
    font.info.version_major = native.version_major.max(0) as u32;
    font.info.version_minor = native.version_minor;
    font.info.copyright = native.get_default_name("copyrights").map(str::to_owned);
    font.info.trademark = native.get_default_name("trademarks").map(str::to_owned);
    font.info.open_type_name_designer = native.get_default_name("designers").map(str::to_owned);
    font.info.open_type_name_manufacturer =
        native.get_default_name("manufacturers").map(str::to_owned);
    font.info.open_type_name_license = native.get_default_name("licenses").map(str::to_owned);
    font.info.note = text(raw, "note").map(str::to_owned);
    metadata::read_info(&mut font.info, &native);
    font.masters.clear();
    font.axes.clear();
    let explicit_axes = raw.get("axes").is_some()
        || array(raw, "customParameters")?
            .iter()
            .any(|parameter| text(parameter, "name") == Some("Axes"));
    let brace_axis_count = native
        .glyphs
        .values()
        .flat_map(|glyph| &glyph.layers)
        .map(|layer| layer.attributes.coordinates.len())
        .max()
        .unwrap_or(0);
    for (index, axis) in native.axes.iter().enumerate() {
        let tag: Tag = axis
            .tag
            .parse()
            .map_err(|_| error(format!("axis tag {} must have four characters", axis.tag)))?;
        let coordinates = native
            .masters
            .iter()
            .map(|m| m.axes_values.get(index).map_or(0., |v| v.into_inner()))
            .collect::<Vec<_>>();
        if coordinates.iter().any(|v| !v.is_finite()) {
            return Err(error("nonfinite master location"));
        }
        let min = coordinates.iter().copied().fold(f64::INFINITY, f64::min);
        let max = coordinates
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        if version == 2. && min == max && !explicit_axes && index >= brace_axis_count {
            continue;
        }
        let mut map: Vec<(f64, f64)> = native
            .axis_mappings
            .get(&axis.name)
            .filter(|mapping| !mapping.is_identity())
            .map(|mapping| {
                mapping
                    .iter()
                    .map(|(a, b)| (a.into_inner(), b.into_inner()))
                    .collect()
            })
            .unwrap_or_default();
        map.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut model = Axis {
            tag,
            name: axis.name.clone(),
            min: metadata::user_coordinate(&map, min)?,
            default: metadata::user_coordinate(
                &map,
                coordinates
                    .get(native.default_master_idx)
                    .copied()
                    .unwrap_or(min),
            )?,
            max: metadata::user_coordinate(&map, max)?,
            hidden: axis.hidden.unwrap_or(false),
            map,
        };
        if let Some(raw_axis) = find(raw, "axes", "tag", &axis.tag) {
            model.default = numeric(raw_axis, "default", model.default)?;
            // Source bounds are implicit; retain explicit user-space bounds in
            // editor metadata, bounded by the actual native master positions.
            let stored: metadata::AxisLimits =
                extras::restore(raw_axis, &["min", "max", "native_locations", "native_map"])?;
            let mut current_locations = coordinates.clone();
            current_locations.sort_by(f64::total_cmp);
            if stored.native_locations == current_locations && stored.native_map == model.map {
                model.min = stored.min.min(model.min);
                model.max = stored.max.max(model.max);
            }
        }
        font.axes.push(model);
    }
    for source in &native.masters {
        let raw_master = find(raw, "fontMaster", "id", &source.id)
            .ok_or_else(|| error("normalizer lost a master"))?;
        let mut master: Master = extras::restore(raw_master, extras::MASTER)?;
        master.id = MasterId(source.id.as_str().into());
        master.name = source.name.clone();
        master.location.clear();
        for axis in &font.axes {
            let index = native
                .axes
                .iter()
                .position(|a| a.tag == axis.tag.to_string())
                .ok_or_else(|| error("missing axis"))?;
            master.location.insert(
                axis.tag,
                source
                    .axes_values
                    .get(index)
                    .map_or(axis.default, |v| v.into_inner()),
            );
        }
        master.metrics.ascender = source.ascender().unwrap_or(0.);
        master.metrics.descender = source.descender().unwrap_or(0.);
        master.metrics.x_height = source.x_height().unwrap_or(0.);
        master.metrics.cap_height = source.cap_height().unwrap_or(0.);
        master.metrics.italic_angle = source.italic_angle().unwrap_or(0.);
        metadata::read_metrics(
            &mut master.metrics,
            &source.custom_parameters,
            &native.custom_parameters,
        );
        master.guides = outlines::guides(raw_master)?;
        let key = if version == 2. {
            "kerning"
        } else {
            "kerningLTR"
        };
        master.kerning_h = kerning(raw.get(key).and_then(|v| v.get(&source.id)))?;
        master.kerning_v = kerning(raw.get("kerningVertical").and_then(|v| v.get(&source.id)))?;
        font.masters.push(master);
    }
    if native.default_master_idx > 0 && native.default_master_idx < font.masters.len() {
        let default = font.masters.remove(native.default_master_idx);
        font.masters.insert(0, default);
    }
    font.info.style_name = font.default_master().name.clone();
    for source in &native.instances {
        let mut instance: Instance = if let Some(raw) = find(raw, "instances", "name", &source.name)
        {
            extras::restore(raw, &["style_name", "family_name", "user_data"])?
        } else {
            Instance::default()
        };
        instance.name = source.name.clone();
        if instance.style_name.is_empty() {
            instance.style_name = source.name.clone();
        }
        instance.active = source.active;
        instance.is_variable_named_instance =
            matches!(source.type_, glyphs_reader::InstanceType::Variable);
        for axis in &font.axes {
            if let Some(index) = native
                .axes
                .iter()
                .position(|a| a.tag == axis.tag.to_string())
            {
                instance.location.insert(
                    axis.tag,
                    source
                        .axes_values
                        .get(index)
                        .map_or(axis.default, |v| v.into_inner()),
                );
            }
        }
        font.instances.push(instance);
    }
    let raw_glyphs = array(raw, "glyphs")?;
    if raw_glyphs.len() > 65535 {
        return Err(error("source exceeds 65535 glyphs"));
    }
    let mut names = BTreeSet::new();
    for raw_glyph in raw_glyphs {
        if !names.insert(required(raw_glyph, "glyphname")?) {
            return Err(error("duplicate glyph name"));
        }
    }
    let by_name = raw_glyphs
        .iter()
        .filter_map(|glyph| text(glyph, "glyphname").map(|name| (name, glyph)))
        .collect::<BTreeMap<_, _>>();
    for name in &native.glyph_order {
        let source = by_name
            .get(name.as_str())
            .copied()
            .ok_or_else(|| error("glyphOrder references missing glyph"))?;
        let glyph = glyph(source, &font, version)?;
        if let Some(group) = &glyph.kern_group_side1 {
            font.kerning_groups
                .side1
                .entry(group.clone())
                .or_default()
                .insert(glyph.name.clone());
        }
        if let Some(group) = &glyph.kern_group_side2 {
            font.kerning_groups
                .side2
                .entry(group.clone())
                .or_default()
                .insert(glyph.name.clone());
        }
        font.glyphs.insert(glyph.name.clone(), glyph);
    }
    font.features.prefixes = blocks(raw, "featurePrefixes", false)?;
    font.features.features = blocks(raw, "features", true)?;
    font.features.classes = array(raw, "classes")?
        .iter()
        .filter(|item| item.get("disabled").and_then(Plist::as_i64) != Some(1))
        .map(|item| {
            Ok(FeatureClass {
                name: required(item, "name")?.into(),
                code: text(item, "code").unwrap_or("").into(),
                automatic: flag(item, "automatic", false)?,
                ..extras::restore(item, &["id"])?
            })
        })
        .collect::<Result<_, Error>>()?;
    validate_references(&font)?;
    metadata::reconcile_generation(raw, &mut font)?;
    Ok(font)
}

fn glyph(raw: &Plist, font: &Font, version: f64) -> Result<Glyph, Error> {
    let mut glyph: Glyph = extras::restore(raw, extras::GLYPH)?;
    glyph.name = required(raw, "glyphname")?.into();
    glyph.export = flag(raw, "export", true)?;
    glyph.production_name = text(raw, "production")
        .or_else(|| text(raw, "productionName"))
        .map(str::to_owned);
    glyph.category = text(raw, "category").map(str::to_owned);
    glyph.sub_category = text(raw, "subCategory").map(str::to_owned);
    glyph.script = text(raw, "script").map(str::to_owned);
    glyph.note = text(raw, "note").unwrap_or("").into();
    glyph.color_label = raw
        .get("color")
        .and_then(Plist::as_i64)
        .and_then(|n| u8::try_from(n).ok());
    glyph.kern_group_side1 = text(raw, "kernRight")
        .or_else(|| text(raw, "rightKerningGroup"))
        .map(str::to_owned);
    glyph.kern_group_side2 = text(raw, "kernLeft")
        .or_else(|| text(raw, "leftKerningGroup"))
        .map(str::to_owned);
    glyph.metrics_keys.left = text(raw, "metricLeft")
        .or_else(|| text(raw, "leftMetricsKey"))
        .map(str::to_owned);
    glyph.metrics_keys.right = text(raw, "metricRight")
        .or_else(|| text(raw, "rightMetricsKey"))
        .map(str::to_owned);
    glyph.metrics_keys.width = text(raw, "metricWidth")
        .or_else(|| text(raw, "widthMetricsKey"))
        .map(str::to_owned);
    if let Some(value) = raw.get("unicode") {
        let values = match value {
            Plist::Array(values) => values.clone(),
            other => vec![other.clone()],
        };
        for value in values {
            let value = match &value {
                Plist::String(s) => s.clone(),
                Plist::Integer(n) => n.to_string(),
                _ => return Err(error("invalid Unicode assignment")),
            };
            for value in value.split(',') {
                let n = u32::from_str_radix(value.trim(), if version == 2. { 16 } else { 10 })
                    .map_err(|_| error("invalid Unicode assignment"))?;
                if char::from_u32(n).is_none() {
                    return Err(error("Unicode assignment is not a scalar value"));
                }
                glyph.unicodes.push(n);
            }
        }
    }
    let mut layer_ids = BTreeSet::new();
    for raw_layer in array(raw, "layers")? {
        let id = required(raw_layer, "layerId")?;
        if !layer_ids.insert(id) {
            return Err(error(format!("duplicate layer {id}")));
        }
        let master = text(raw_layer, "associatedMasterId").unwrap_or(id);
        let master_id = font
            .masters
            .iter()
            .find(|m| m.id.to_string() == master)
            .map(|m| m.id.clone())
            .ok_or_else(|| error(format!("layer references missing master {master}")))?;
        let kind = if id == master {
            LayerKind::Master(master_id.clone())
        } else {
            LayerKind::Custom(master.into())
        };
        let mut layer = parse_layer(raw_layer, kind)?;
        if id != master {
            layer.id = fc_model::LayerId(id.into());
        }
        if let Some(coordinates) = raw_layer.get("attr").and_then(|a| a.get("coordinates")) {
            let values = coordinates
                .as_array()
                .ok_or_else(|| error("brace coordinates must be an array"))?;
            if values.len() != font.axes.len() {
                return Err(error("brace coordinates do not match axes"));
            }
            layer.kind = LayerKind::Brace {
                master: master_id.clone(),
                location: font
                    .axes
                    .iter()
                    .zip(values)
                    .map(|(axis, value)| Ok((axis.tag, as_number(value)?)))
                    .collect::<Result<_, Error>>()?,
            };
        } else if version == 2. && id != master {
            if let Some((_, rest)) = layer.name.split_once('{') {
                if let Some((coordinates, _)) = rest.split_once('}') {
                    let values = coordinates
                        .split(',')
                        .map(|value| {
                            value
                                .trim()
                                .parse::<f64>()
                                .map_err(|_| error("invalid Glyphs 2 brace coordinates"))
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    if values.len() != font.axes.len() || values.iter().any(|v| !v.is_finite()) {
                        return Err(error(
                            "Glyphs 2 brace coordinates do not match the font axes",
                        ));
                    }
                    layer.kind = LayerKind::Brace {
                        master: master_id.clone(),
                        location: font
                            .axes
                            .iter()
                            .zip(values)
                            .map(|(axis, value)| (axis.tag, value))
                            .collect(),
                    };
                }
            }
        }
        if let Some(background) = raw_layer.get("background") {
            glyph
                .layers
                .push(parse_layer(background, LayerKind::Background(master_id))?);
        }
        glyph.layers.push(layer);
    }
    for master in &font.masters {
        if glyph.master_layer(&master.id).is_none() {
            return Err(error(format!(
                "glyph {} is missing master {}",
                glyph.name, master.name
            )));
        }
    }
    Ok(glyph)
}

fn parse_layer(raw: &Plist, kind: LayerKind) -> Result<Layer, Error> {
    let mut result: Layer = extras::restore(raw, extras::LAYER)?;
    result.kind = kind;
    result.name = text(raw, "name").unwrap_or("").into();
    result.width = numeric(raw, "width", 0.)?;
    result.vertical_width = optional_number(raw, "vertWidth")?;
    result.vertical_origin = optional_number(raw, "vertOrigin")?;
    for path in array(raw, "paths")? {
        result.contours.push(outlines::contour(path)?);
    }
    for component in array(raw, "components")? {
        result.components.push(outlines::component(component)?);
    }
    for shape in array(raw, "shapes")? {
        if shape.get("ref").is_some() || shape.get("name").is_some() {
            result.components.push(outlines::component(shape)?);
        } else {
            result.contours.push(outlines::contour(shape)?);
        }
    }
    result.anchors = outlines::anchors(raw)?;
    result.guides = outlines::guides(raw)?;
    Ok(result)
}

fn blocks(raw: &Plist, key: &str, feature: bool) -> Result<Vec<FeatureBlock>, Error> {
    array(raw, key)?
        .iter()
        .map(|item| {
            Ok(FeatureBlock {
                tag: text(item, if feature { "tag" } else { "name" })
                    .or_else(|| text(item, "name"))
                    .unwrap_or("")
                    .into(),
                code: text(item, "code").unwrap_or("").into(),
                automatic: flag(item, "automatic", false)?,
                disabled: flag(item, "disabled", false)?,
                notes: text(item, "notes").unwrap_or("").into(),
                ..extras::restore(item, &["id"])?
            })
        })
        .collect()
}

pub(super) fn kerning(raw: Option<&Plist>) -> Result<Kerning, Error> {
    let mut result = Kerning::default();
    let Some(raw) = raw else { return Ok(result) };
    let pairs = raw
        .as_dict()
        .ok_or_else(|| error("kerning must be a dictionary"))?;
    for (left, rights) in pairs {
        let rights = rights
            .as_dict()
            .ok_or_else(|| error("kerning right sides must be a dictionary"))?;
        for (right, value) in rights {
            result.0.insert(
                (kern_side(left, true)?, kern_side(right, false)?),
                as_number(value)?,
            );
        }
    }
    Ok(result)
}

fn kern_side(value: &str, left: bool) -> Result<KernSide, Error> {
    let prefix = if left { "@MMK_L_" } else { "@MMK_R_" };
    if let Some(group) = value.strip_prefix(prefix) {
        Ok(KernSide::Group(group.into()))
    } else if value.starts_with('@') {
        Err(error(format!("unexpected kerning group {value}")))
    } else {
        Ok(KernSide::Glyph(value.into()))
    }
}

pub(super) fn check_supported(raw: &Plist) -> Result<(), Error> {
    if raw
        .get("kerningRTL")
        .and_then(Plist::as_dict)
        .is_some_and(|v| !v.is_empty())
    {
        return Err(error(
            "right-to-left kerning is not yet represented by the model",
        ));
    }
    for glyph in array(raw, "glyphs")? {
        if !array(glyph, "partsSettings")?.is_empty() {
            return Err(error("smart component axes are not yet supported"));
        }
        for layer in array(glyph, "layers")? {
            let name = text(layer, "name").unwrap_or("");
            if layer.get("associatedMasterId").is_some()
                && (name.contains('[') || name.contains(']'))
            {
                return Err(error("conditional bracket layers are not yet supported"));
            }
            if let Some(attr) = layer.get("attr") {
                for key in ["axisRules", "color", "colorPalette", "svg", "sbixSize"] {
                    if attr.get(key).is_some() {
                        return Err(error(format!("layer attribute {key} is not yet supported")));
                    }
                }
            }
            for hint in array(layer, "hints")? {
                if hint.get("name").is_some() {
                    return Err(error(
                        "corner and cap component hints are not yet supported",
                    ));
                }
            }
            for key in ["shapes", "components", "paths"] {
                for shape in array(layer, key)? {
                    if shape
                        .get("piece")
                        .and_then(Plist::as_dict)
                        .is_some_and(|v| !v.is_empty())
                    {
                        return Err(error("smart components are not yet supported"));
                    }
                    if shape.get("attr").and_then(Plist::as_dict).is_some_and(|a| {
                        [
                            "fill",
                            "fillColor",
                            "stroke",
                            "strokeColor",
                            "strokeWidth",
                            "gradient",
                            "mask",
                            "reversePaths",
                            "shadow",
                            "shadowIn",
                            "strokeHeight",
                            "strokePos",
                        ]
                        .iter()
                        .any(|k| a.contains_key(*k))
                    }) {
                        return Err(error(
                            "color and paint shape attributes are not yet supported",
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

pub(super) fn validate_references(font: &Font) -> Result<(), Error> {
    for glyph in font.glyphs.values() {
        for layer in &glyph.layers {
            for component in &layer.components {
                if font.glyph(&component.base).is_none() {
                    return Err(error(format!(
                        "glyph {} references missing component {}",
                        glyph.name, component.base
                    )));
                }
            }
        }
    }
    fn visit(
        font: &Font,
        name: &str,
        stack: &mut BTreeSet<String>,
        done: &mut BTreeSet<String>,
        depth: usize,
    ) -> Result<(), Error> {
        if depth > 64 {
            return Err(error("component depth exceeds 64"));
        }
        if done.contains(name) {
            return Ok(());
        }
        if !stack.insert(name.into()) {
            return Err(error(format!("component cycle through {name}")));
        }
        if let Some(glyph) = font.glyph(name) {
            for layer in &glyph.layers {
                for component in &layer.components {
                    visit(font, &component.base, stack, done, depth + 1)?;
                }
            }
        }
        stack.remove(name);
        done.insert(name.into());
        Ok(())
    }
    let mut done = BTreeSet::new();
    for name in font.glyphs.keys() {
        visit(font, name, &mut BTreeSet::new(), &mut done, 0)?;
    }
    for master in &font.masters {
        for kerning in [&master.kerning_h, &master.kerning_v] {
            for ((left, right), value) in &kerning.0 {
                if !value.is_finite() {
                    return Err(error("nonfinite kerning adjustment"));
                }
                for (side, groups) in [
                    (left, &font.kerning_groups.side1),
                    (right, &font.kerning_groups.side2),
                ] {
                    let valid = match side {
                        KernSide::Glyph(name) => font.glyph(name).is_some(),
                        KernSide::Group(name) => groups.contains_key(name),
                    };
                    if !valid {
                        return Err(error("kerning references missing glyph or group"));
                    }
                }
            }
        }
    }
    Ok(())
}
