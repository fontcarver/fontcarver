//! Native Glyphs 3 serialization with overlays of safe unknown source fields.

use super::{error, extras, metadata, outlines, plist::*, read, SHADOW};
use crate::Error;
use fc_model::{FeatureBlock, Font, Glyph, KernSide, Kerning, Layer, LayerKind, Master};
use glyphs_reader::Plist;

pub(super) fn font(font: &Font) -> Result<Plist, Error> {
    if font.masters.is_empty() {
        return Err(error("source requires at least one master"));
    }
    if font.info.units_per_em == 0 {
        return Err(error("unitsPerEm must be positive"));
    }
    if font.cid.is_some() || !font.bitmap_strikes.is_empty() {
        return Err(error(
            "Glyphs export does not yet support CID or bitmap sources",
        ));
    }
    read::validate_references(font)?;
    let mut raw = font
        .extra
        .lib
        .get(SHADOW)
        .and_then(serde_json::Value::as_str)
        .map(parse)
        .transpose()?
        .unwrap_or_else(dict);
    let source = raw.clone();
    remove(
        &mut raw,
        &[
            "kerning",
            "kerningLTR",
            "kerningRTL",
            "kerningVertical",
            "glyphs",
            "fontMaster",
            "axes",
            "instances",
            "metrics",
            "features",
            "featurePrefixes",
            "classes",
        ],
    );
    set(&mut raw, ".formatVersion", integer(3));
    set(&mut raw, ".appVersion", string("3300"));
    set(&mut raw, "familyName", string(&font.info.family_name));
    set(&mut raw, "unitsPerEm", integer(font.info.units_per_em));
    set(&mut raw, "versionMajor", integer(font.info.version_major));
    set(&mut raw, "versionMinor", integer(font.info.version_minor));
    optional_string(&mut raw, "note", font.info.note.as_deref());
    write_properties(&mut raw, font)?;
    let axes = font
        .axes
        .iter()
        .map(|axis| {
            let mut raw = find(&source, "axes", "tag", &axis.tag.to_string())
                .cloned()
                .unwrap_or_else(dict);
            set(&mut raw, "tag", string(axis.tag.to_string()));
            set(&mut raw, "name", string(&axis.name));
            set(&mut raw, "hidden", boolean(axis.hidden));
            set(&mut raw, "default", number(axis.default)?);
            metadata::write_axis_limits(&mut raw, font, axis)?;
            Ok(raw)
        })
        .collect::<Result<_, Error>>()?;
    set(&mut raw, "axes", Plist::Array(axes));
    set(
        &mut raw,
        "metrics",
        Plist::Array(metadata::metric_definitions(&source)?),
    );
    set(
        &mut raw,
        "fontMaster",
        Plist::Array(
            font.masters
                .iter()
                .map(|master| write_master(font, master, &source))
                .collect::<Result<_, _>>()?,
        ),
    );
    let mut instances = Vec::new();
    for instance in &font.instances {
        let mut raw = find(&source, "instances", "name", &instance.name)
            .cloned()
            .unwrap_or_else(dict);
        remove(
            &mut raw,
            &[
                "interpolationWeight",
                "interpolationWidth",
                "interpolationCustom",
            ],
        );
        set(&mut raw, "name", string(&instance.name));
        set(&mut raw, "active", boolean(instance.active));
        set(
            &mut raw,
            "axesValues",
            numbers(
                font.axes
                    .iter()
                    .map(|a| instance.location.get(&a.tag).copied().unwrap_or(a.default)),
            )?,
        );
        if instance.is_variable_named_instance {
            set(&mut raw, "type", string("variable"));
        } else {
            remove(&mut raw, &["type"]);
        }
        extras::store(
            &mut raw,
            instance,
            &["style_name", "family_name", "user_data"],
        )?;
        instances.push(raw);
    }
    set(&mut raw, "instances", Plist::Array(instances));
    set(
        &mut raw,
        "glyphs",
        Plist::Array(
            font.glyphs
                .values()
                .map(|glyph| write_glyph(font, glyph, &source))
                .collect::<Result<_, _>>()?,
        ),
    );
    set(&mut raw, "kerningLTR", write_kerning(font, false)?);
    set(&mut raw, "kerningVertical", write_kerning(font, true)?);
    set(
        &mut raw,
        "featurePrefixes",
        Plist::Array(
            font.features
                .prefixes
                .iter()
                .enumerate()
                .map(|(index, block)| write_block(block, &source, "featurePrefixes", index, false))
                .collect::<Result<_, _>>()?,
        ),
    );
    set(
        &mut raw,
        "features",
        Plist::Array(
            font.features
                .features
                .iter()
                .enumerate()
                .map(|(index, block)| write_block(block, &source, "features", index, true))
                .collect::<Result<_, _>>()?,
        ),
    );
    let mut classes = Vec::new();
    for class in &font.features.classes {
        let name = class.name.trim_start_matches('@');
        let mut raw = find(&source, "classes", "name", name)
            .cloned()
            .unwrap_or_else(dict);
        set(&mut raw, "name", string(name));
        let code = class.code.trim();
        set(
            &mut raw,
            "code",
            string(
                code.strip_prefix('[')
                    .and_then(|v| v.strip_suffix(']'))
                    .unwrap_or(&class.code),
            ),
        );
        set(&mut raw, "automatic", boolean(class.automatic));
        set(&mut raw, "disabled", boolean(false));
        extras::store(&mut raw, class, &["id"])?;
        classes.push(raw);
    }
    for class in array(&source, "classes")? {
        if flag(class, "disabled", false)? {
            classes.push(class.clone());
        }
    }
    set(&mut raw, "classes", Plist::Array(classes));
    let mut parameters = array(&source, "customParameters")?
        .iter()
        .filter(|p| {
            !matches!(
                text(p, "name"),
                Some("Axes" | "glyphOrder" | "Variable Font Origin" | "Axis Mappings")
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut order = dict();
    set(&mut order, "name", string("glyphOrder"));
    set(
        &mut order,
        "value",
        Plist::Array(
            font.glyph_order()
                .map(|name| string(name.to_string()))
                .collect(),
        ),
    );
    parameters.push(order);
    if !font.axes.is_empty() {
        let mut origin = dict();
        set(&mut origin, "name", string("Variable Font Origin"));
        set(
            &mut origin,
            "value",
            string(font.default_master().id.to_string()),
        );
        parameters.push(origin);
    }
    let mut mappings = dict();
    for axis in &font.axes {
        if !axis.map.is_empty() {
            let mut mapping = dict();
            for (user, design) in &axis.map {
                set(&mut mapping, &user.to_string(), number(*design)?);
            }
            set(&mut mappings, &axis.tag.to_string(), mapping);
        }
    }
    if mappings.as_dict().is_some_and(|v| !v.is_empty()) {
        let mut parameter = dict();
        set(&mut parameter, "name", string("Axis Mappings"));
        set(&mut parameter, "value", mappings);
        parameters.push(parameter);
    }
    set(&mut raw, "customParameters", Plist::Array(parameters));
    metadata::write_info(&mut raw, &font.info)?;
    extras::store(&mut raw, font, extras::FONT)?;
    Ok(raw)
}

fn write_master(font: &Font, master: &Master, source: &Plist) -> Result<Plist, Error> {
    let mut raw = find(source, "fontMaster", "id", &master.id.to_string())
        .cloned()
        .unwrap_or_else(dict);
    remove(
        &mut raw,
        &[
            "ascender",
            "descender",
            "xHeight",
            "capHeight",
            "italicAngle",
            "weightValue",
            "widthValue",
            "customValue",
            "guideLines",
        ],
    );
    set(&mut raw, "id", string(master.id.to_string()));
    set(&mut raw, "name", string(&master.name));
    set(
        &mut raw,
        "axesValues",
        numbers(
            font.axes
                .iter()
                .map(|a| master.location.get(&a.tag).copied().unwrap_or(a.default)),
        )?,
    );
    let values = metadata::metric_values(source, &raw, &master.metrics)?;
    set(&mut raw, "metricValues", values);
    set(
        &mut raw,
        "guides",
        outlines::write_guides(
            &master.guides,
            find(source, "fontMaster", "id", &master.id.to_string()),
        )?,
    );
    extras::store(&mut raw, master, extras::MASTER)?;
    metadata::write_metrics(&mut raw, &master.metrics)?;
    Ok(raw)
}

fn write_glyph(font: &Font, glyph: &Glyph, source: &Plist) -> Result<Plist, Error> {
    if glyph.name.is_empty() {
        return Err(error("empty glyph name"));
    }
    let mut raw = find(source, "glyphs", "glyphname", &glyph.name)
        .cloned()
        .unwrap_or_else(dict);
    let original = raw.clone();
    remove(
        &mut raw,
        &[
            "unicode",
            "leftKerningGroup",
            "rightKerningGroup",
            "leftMetricsKey",
            "rightMetricsKey",
            "widthMetricsKey",
            "productionName",
        ],
    );
    super::smart::write_axes(&mut raw, glyph)?;
    set(&mut raw, "glyphname", string(glyph.name.to_string()));
    set(&mut raw, "export", boolean(glyph.export));
    if glyph.unicodes.iter().any(|v| char::from_u32(*v).is_none()) {
        return Err(error(format!(
            "invalid Unicode assignment for {}",
            glyph.name
        )));
    }
    if !glyph.unicodes.is_empty() {
        set(
            &mut raw,
            "unicode",
            Plist::Array(glyph.unicodes.iter().map(|n| integer(*n)).collect()),
        );
    }
    optional_string(&mut raw, "production", glyph.production_name.as_deref());
    optional_string(&mut raw, "category", glyph.category.as_deref());
    optional_string(&mut raw, "subCategory", glyph.sub_category.as_deref());
    optional_string(&mut raw, "script", glyph.script.as_deref());
    set(&mut raw, "note", string(&glyph.note));
    if let Some(color) = glyph.color_label {
        set(&mut raw, "color", integer(color));
    } else {
        remove(&mut raw, &["color"]);
    }
    let group1 = group(font, glyph, true)?;
    let group2 = group(font, glyph, false)?;
    optional_string(&mut raw, "kernRight", group1.as_deref());
    optional_string(&mut raw, "kernLeft", group2.as_deref());
    optional_string(&mut raw, "metricLeft", glyph.metrics_keys.left.as_deref());
    optional_string(&mut raw, "metricRight", glyph.metrics_keys.right.as_deref());
    optional_string(&mut raw, "metricWidth", glyph.metrics_keys.width.as_deref());
    let mut layers = Vec::new();
    for layer in &glyph.layers {
        if matches!(layer.kind, LayerKind::Background(_)) {
            continue;
        }
        let (id, associated) = match &layer.kind {
            LayerKind::Master(master) => (master.to_string(), None),
            LayerKind::Smart { master, .. }
            | LayerKind::Brace { master, .. }
            | LayerKind::Bracket { master, .. } => (layer.id.to_string(), Some(master.to_string())),
            LayerKind::Backup { of, .. } => (layer.id.to_string(), Some(of.to_string())),
            LayerKind::Custom(master) => (
                layer.id.to_string(),
                Some(
                    if font.masters.iter().any(|m| m.id.to_string() == *master) {
                        master.clone()
                    } else {
                        font.default_master().id.to_string()
                    },
                ),
            ),
            LayerKind::Background(_) => continue,
        };
        let source_layer = find(&original, "layers", "layerId", &id);
        let mut raw = write_layer(layer, source_layer)?;
        set(&mut raw, "layerId", string(&id));
        optional_string(&mut raw, "associatedMasterId", associated.as_deref());
        super::conditional::write(&mut raw, font, layer)?;
        super::smart::write_layer(&mut raw, glyph, layer)?;
        if let LayerKind::Master(master) = &layer.kind {
            let background = glyph
                .layers
                .iter()
                .find(|l| matches!(&l.kind,LayerKind::Background(id) if id==master));
            if let Some(background) = background {
                set(
                    &mut raw,
                    "background",
                    write_layer(background, source_layer.and_then(|v| v.get("background")))?,
                );
            } else {
                remove(&mut raw, &["background"]);
            }
        }
        layers.push(raw);
    }
    for master in &font.masters {
        if glyph.master_layer(&master.id).is_none() {
            return Err(error(format!(
                "glyph {} is missing master {}",
                glyph.name, master.name
            )));
        }
    }
    set(&mut raw, "layers", Plist::Array(layers));
    extras::store(&mut raw, glyph, extras::GLYPH)?;
    Ok(raw)
}

fn group(font: &Font, glyph: &Glyph, left: bool) -> Result<Option<String>, Error> {
    let groups = if left {
        &font.kerning_groups.side1
    } else {
        &font.kerning_groups.side2
    };
    let memberships = groups
        .iter()
        .filter(|(_, members)| members.contains(&glyph.name))
        .map(|(name, _)| name)
        .collect::<Vec<_>>();
    if memberships.len() > 1 {
        return Err(error(format!(
            "glyph {} belongs to multiple kerning groups on one side",
            glyph.name
        )));
    }
    let explicit = if left {
        &glyph.kern_group_side1
    } else {
        &glyph.kern_group_side2
    };
    if let (Some(explicit), Some(actual)) = (explicit, memberships.first()) {
        if explicit != *actual {
            return Err(error(format!(
                "inconsistent kerning membership for {}",
                glyph.name
            )));
        }
    }
    Ok(memberships
        .first()
        .map(|name| (*name).clone())
        .or_else(|| explicit.clone()))
}

fn write_layer(layer: &Layer, source: Option<&Plist>) -> Result<Plist, Error> {
    if layer.color_layers.as_ref().is_some_and(|v| !v.is_empty()) {
        return Err(error("color layers are not yet supported"));
    }
    let mut raw = source.cloned().unwrap_or_else(dict);
    remove(&mut raw, &["paths", "components", "shapes", "guideLines"]);
    set(&mut raw, "name", string(&layer.name));
    set(&mut raw, "width", number(layer.width)?);
    optional_number(&mut raw, "vertWidth", layer.vertical_width)?;
    optional_number(&mut raw, "vertOrigin", layer.vertical_origin)?;
    let mut paths = Vec::new();
    let mut components = Vec::new();
    if let Some(source) = source {
        paths.extend(array(source, "paths")?.iter());
        components.extend(array(source, "components")?.iter());
        for shape in array(source, "shapes")? {
            if shape.get("ref").is_some() || shape.get("name").is_some() {
                components.push(shape);
            } else {
                paths.push(shape);
            }
        }
    }
    let mut shapes = layer
        .contours
        .iter()
        .enumerate()
        .map(|(i, contour)| outlines::write_contour(contour, paths.get(i).copied()))
        .collect::<Result<Vec<_>, _>>()?;
    shapes.extend(
        layer
            .components
            .iter()
            .enumerate()
            .map(|(i, component)| outlines::write_component(component, components.get(i).copied()))
            .collect::<Result<Vec<_>, _>>()?,
    );
    set(&mut raw, "shapes", Plist::Array(shapes));
    let anchors = outlines::write_anchors(&layer.anchors, &raw)?;
    set(&mut raw, "anchors", anchors);
    set(
        &mut raw,
        "guides",
        outlines::write_guides(&layer.guides, source)?,
    );
    extras::store(&mut raw, layer, extras::LAYER)?;
    Ok(raw)
}

fn write_kerning(font: &Font, vertical: bool) -> Result<Plist, Error> {
    let mut result = dict();
    for master in &font.masters {
        let Kerning(pairs) = if vertical {
            &master.kerning_v
        } else {
            &master.kerning_h
        };
        let mut raw = dict();
        for ((left, right), value) in pairs {
            let key = |side: &KernSide, left| match side {
                KernSide::Glyph(name) => name.to_string(),
                KernSide::Group(name) => format!("@MMK_{}_{name}", if left { "L" } else { "R" }),
            };
            let left = key(left, true);
            let mut rights = raw.get(&left).cloned().unwrap_or_else(dict);
            set(&mut rights, &key(right, false), number(*value)?);
            set(&mut raw, &left, rights);
        }
        set(&mut result, &master.id.to_string(), raw);
    }
    Ok(result)
}

fn write_block(
    block: &FeatureBlock,
    source: &Plist,
    key: &str,
    index: usize,
    feature: bool,
) -> Result<Plist, Error> {
    let mut raw = source
        .get(key)
        .and_then(Plist::as_array)
        .and_then(|a| a.get(index))
        .cloned()
        .unwrap_or_else(dict);
    if feature {
        remove(&mut raw, &["name"]);
    }
    set(
        &mut raw,
        if feature { "tag" } else { "name" },
        string(&block.tag),
    );
    let code = if feature {
        let trimmed = block.code.trim();
        trimmed
            .strip_prefix(&format!("feature {} {{", block.tag))
            .and_then(|s| s.strip_suffix(&format!("}} {};", block.tag)))
            .unwrap_or(&block.code)
    } else {
        &block.code
    };
    set(&mut raw, "code", string(code));
    set(&mut raw, "automatic", boolean(block.automatic));
    set(&mut raw, "disabled", boolean(block.disabled));
    set(&mut raw, "notes", string(&block.notes));
    let snapshot = FeatureBlock {
        code: code.into(),
        ..block.clone()
    };
    extras::store(
        &mut raw,
        &snapshot,
        &["id", "tag", "code", "automatic", "disabled"],
    )?;
    Ok(raw)
}

fn write_properties(raw: &mut Plist, font: &Font) -> Result<(), Error> {
    let fields = [
        ("familyNames", Some(font.info.family_name.as_str())),
        ("copyrights", font.info.copyright.as_deref()),
        ("trademarks", font.info.trademark.as_deref()),
        ("designers", font.info.open_type_name_designer.as_deref()),
        (
            "manufacturers",
            font.info.open_type_name_manufacturer.as_deref(),
        ),
        ("licenses", font.info.open_type_name_license.as_deref()),
    ];
    let mut properties = array(raw, "properties")?.to_vec();
    for (key, value) in fields {
        let old = properties.iter().position(|p| text(p, "key") == Some(key));
        let mut property = old.map(|i| properties.remove(i)).unwrap_or_else(dict);
        if let Some(value) = value {
            let mut values = array(&property, "values")?
                .iter()
                .filter(|v| text(v, "language") != Some("dflt"))
                .cloned()
                .collect::<Vec<_>>();
            let mut item = dict();
            set(&mut item, "language", string("dflt"));
            set(&mut item, "value", string(value));
            values.insert(0, item);
            remove(&mut property, &["value"]);
            set(&mut property, "key", string(key));
            set(&mut property, "values", Plist::Array(values));
            properties.push(property);
        }
    }
    set(raw, "properties", Plist::Array(properties));
    remove(
        raw,
        &[
            "copyright",
            "designer",
            "manufacturer",
            "trademark",
            "license",
        ],
    );
    Ok(())
}

fn optional_string(raw: &mut Plist, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        set(raw, key, string(value));
    } else {
        remove(raw, &[key]);
    }
}
fn optional_number(raw: &mut Plist, key: &str, value: Option<f64>) -> Result<(), Error> {
    if let Some(value) = value {
        set(raw, key, number(value)?);
    } else {
        remove(raw, &[key]);
    }
    Ok(())
}
