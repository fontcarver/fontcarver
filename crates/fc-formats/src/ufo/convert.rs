//! Conversion between norad UFO objects and the FontCarver model.

use std::{collections::HashSet, path::Path, str::FromStr};

use fc_model::{
    Anchor, Component, Contour, Extra, Font, Glyph, GlyphName, Layer, LayerKind, Master, MasterId,
    Point, PointKind, SourceProvenance,
};
use norad::{AffineTransform, Color, ContourPoint, Identifier, Name, PointType};
use plist::{Dictionary, Value};

use super::{
    extra, info,
    metadata::{self, ObjectMetadata},
    structure,
    value::{dict_to_json, json_to_dict, json_to_plist, plist_to_json},
};
use crate::Error;

const UFO_LIB_KEY: &str = "ufo.lib";
const UFO_GROUPS_KEY: &str = "ufo.groups";
const UFO_METAINFO_KEY: &str = "ufo.metainfo";
const UFO_LAYER_LIB_KEY: &str = "ufo.layer_lib";
const UFO_GLYPH_LIB_KEY: &str = "ufo.lib";
const UFO_LAYER_GLYPH_LIBS_KEY: &str = "ufo.layer_libs";
pub(crate) const UFO_MASTER_KEY: &str = "ufo.master";

pub(crate) fn from_norad(source: &norad::Font, path: &Path) -> Result<Font, Error> {
    let master_id = MasterId::new();
    let (font_info, metrics, private_dict, guides) = info::from_norad(&source.font_info)?;
    let mut master = Master {
        id: master_id.clone(),
        name: source
            .font_info
            .style_name
            .clone()
            .unwrap_or_else(|| "Regular".to_owned()),
        metrics,
        guides: guides.clone(),
        private_dict,
        user_data: serde_json::json!({ "ufo.source_path": path.to_string_lossy() }),
        ..Master::default()
    };
    if let Some(raw) = font_info.user_data.get(info::RAW_FONTINFO_KEY) {
        super::value::insert_json_value(&mut master.user_data, info::RAW_FONTINFO_KEY, raw.clone());
    }
    let mut font = Font {
        info: font_info,
        masters: vec![master],
        guides,
        features: structure::features_from_norad(&source.features),
        kerning_groups: structure::groups_from_norad(&source.groups),
        extra: Extra {
            source: Some(SourceProvenance {
                path: Some(path.to_string_lossy().into_owned()),
                format: "UFO 3".to_owned(),
                ..SourceProvenance::default()
            }),
            ..Extra::default()
        },
        ..Font::default()
    };
    font.masters[0].kerning_h = structure::kerning_from_norad(&source.kerning);

    let mut object_metadata = Vec::new();
    metadata::record_global_guides(&source.font_info, &mut object_metadata)?;
    let glyph_order = glyph_order(source);
    for glyph_name in glyph_order {
        let representative = source.default_layer().get_glyph(&glyph_name).or_else(|| {
            source
                .iter_layers()
                .find_map(|layer| layer.get_glyph(&glyph_name))
        });
        let Some(representative) = representative else {
            continue;
        };
        let mut glyph = Glyph::new(glyph_name.as_str());
        if let Some(axes) = representative.lib.get("com.fontcarver.smartAxes") {
            glyph.smart_axes = serde_json::from_value(plist_to_json(axes)?)?;
        }
        glyph.unicodes = representative.codepoints.iter().map(u32::from).collect();
        glyph.note = representative.note.clone().unwrap_or_default();
        if !representative.lib.is_empty() {
            glyph.user_data = serde_json::json!({
                UFO_GLYPH_LIB_KEY: dict_to_json(&representative.lib)?
            });
        }

        for ufo_layer in source.iter_layers() {
            let Some(ufo_glyph) = ufo_layer.get_glyph(&glyph_name) else {
                continue;
            };
            let layer_name = ufo_layer.name().to_string();
            let kind = if ufo_layer.is_default() {
                LayerKind::Master(master_id.clone())
            } else if layer_name == "public.background" {
                LayerKind::Background(master_id.clone())
            } else {
                LayerKind::Custom(layer_name.clone())
            };
            let layer = layer_from_norad(
                ufo_glyph,
                ufo_layer,
                kind,
                &glyph_name,
                &layer_name,
                &master_id,
                &mut object_metadata,
            )?;
            if !ufo_layer.is_default() && !ufo_glyph.lib.is_empty() {
                super::value::insert_nested_json(
                    &mut glyph.user_data,
                    UFO_LAYER_GLYPH_LIBS_KEY,
                    &layer_name,
                    dict_to_json(&ufo_glyph.lib)?,
                );
            }
            glyph.layers.push(layer);
        }
        if glyph.master_layer(&master_id).is_none() {
            glyph.layers.insert(0, Layer::new_master(master_id.clone()));
        }
        font.glyphs.insert(glyph.name.clone(), glyph);
    }

    for (name, members) in &source.groups {
        if !name.starts_with("public.kern1.") && !name.starts_with("public.kern2.") {
            let groups = font
                .extra
                .lib
                .entry(UFO_GROUPS_KEY.to_owned())
                .or_insert_with(|| serde_json::Value::Object(Default::default()));
            if let Some(groups) = groups.as_object_mut() {
                groups.insert(
                    name.to_string(),
                    serde_json::Value::Array(
                        members
                            .iter()
                            .map(|member| serde_json::Value::String(member.to_string()))
                            .collect(),
                    ),
                );
            }
        }
    }
    assign_glyph_groups(&mut font);

    font.extra
        .lib
        .insert(UFO_LIB_KEY.to_owned(), dict_to_json(&source.lib)?);
    font.extra.lib.insert(
        UFO_METAINFO_KEY.to_owned(),
        plist_to_json(&plist::to_value(&source.meta)?)?,
    );
    if !object_metadata.is_empty() {
        let object_metadata = serde_json::to_value(object_metadata)?;
        font.extra.lib.insert(
            metadata::IDENTIFIERS_KEY.to_owned(),
            object_metadata.clone(),
        );
        if let Some(master) = font.masters.first_mut() {
            super::value::insert_json_value(
                &mut master.user_data,
                metadata::IDENTIFIERS_KEY,
                object_metadata,
            );
        }
    }
    extra::import(&mut font)?;
    // UFO flattens accepted source into a fresh prefix. Generated item IDs and
    // recipes are deliberately not restored: the exported rules are frozen.
    if let Some(policy) = extra::feature_metadata::read_policy(&source.lib)? {
        font.feature_settings.writer_policy = policy;
    }
    Ok(font)
}

pub(crate) fn to_norad(font: &Font, master: &Master) -> Result<norad::Font, Error> {
    let object_metadata = metadata::decode(
        master
            .user_data
            .get(metadata::IDENTIFIERS_KEY)
            .or_else(|| master.user_data.get(metadata::LEGACY_IDENTIFIERS_KEY))
            .or_else(|| font.extra.lib.get(metadata::IDENTIFIERS_KEY))
            .or_else(|| font.extra.lib.get(metadata::LEGACY_IDENTIFIERS_KEY)),
    )?;
    let guide_ids = font
        .guides
        .iter()
        .enumerate()
        .map(|(index, _)| {
            metadata::lookup(&object_metadata, None, None, "guideline", index, None)
                .and_then(|entry| entry.identifier.clone())
        })
        .collect::<Vec<_>>();

    let mut target = norad::Font::new();
    if let Some(raw) = font.extra.lib.get(UFO_METAINFO_KEY) {
        let raw = json_to_plist(raw)?;
        target.meta = plist::from_value(&raw)?;
        target.meta.format_version = norad::FormatVersion::V3;
    }
    let guides = if master.guides.is_empty() {
        &font.guides
    } else {
        &master.guides
    };
    target.font_info = info::to_norad(&font.info, master, guides, &guide_ids)?;
    for (index, guide) in target.font_info.guidelines.iter_mut().flatten().enumerate() {
        if let Some(lib) = metadata::lookup(&object_metadata, None, None, "guideline", index, None)
            .map(ObjectMetadata::decoded_lib)
            .transpose()?
            .flatten()
        {
            guide.replace_lib(lib);
        }
    }
    target.features = structure::features_to_norad(&font.features);
    target.groups = structure::groups_to_norad(font)?;
    target.kerning = structure::kerning_to_norad(&master.kerning_h)?;
    target.lib = font
        .extra
        .lib
        .get(UFO_LIB_KEY)
        .map(json_to_dict)
        .transpose()?
        .unwrap_or_default();
    target.lib.insert(
        "public.glyphOrder".to_owned(),
        Value::Array(
            font.glyphs
                .keys()
                .map(|name| Value::String(name.to_string()))
                .collect(),
        ),
    );
    if let Some(extra) = extra::build(font, master)? {
        target.lib.insert(extra::EXTRA_KEY.to_owned(), extra);
    }
    extra::feature_metadata::write_policy(&font.feature_settings.writer_policy, &mut target.lib);

    let layer_names = collect_layer_names(font, &master.id);
    for layer_name in &layer_names {
        if layer_name != "public.default" {
            target
                .layers
                .get_or_create_layer(layer_name)
                .map_err(|_| Error::InvalidName(layer_name.clone()))?;
        }
    }

    for glyph in font.glyphs.values() {
        validate_name(glyph.name.as_ref())?;
        for layer in glyph
            .layers
            .iter()
            .filter(|layer| layer_belongs(font, layer, &master.id))
        {
            let layer_name = model_layer_name(layer, &master.id);
            let mut ufo_glyph = glyph_to_norad(glyph, layer, &layer_name, &object_metadata)?;
            if let Some(lib) = glyph_lib(glyph, &layer_name)? {
                ufo_glyph.lib = lib;
            }
            ufo_glyph.lib.remove("com.fontcarver.smartAxes");
            if layer_name == "public.default" && !glyph.smart_axes.is_empty() {
                ufo_glyph.lib.insert(
                    "com.fontcarver.smartAxes".into(),
                    json_to_plist(&serde_json::to_value(&glyph.smart_axes)?)?,
                );
            }
            ufo_glyph.lib.remove("com.fontcarver.sourceLayer");
            if matches!(
                layer.kind,
                LayerKind::Smart { .. } | LayerKind::Bracket { .. }
            ) {
                ufo_glyph.lib.insert(
                    "com.fontcarver.sourceLayer".into(),
                    Value::String(serde_json::json!({"kind":layer.kind,"id":layer.id}).to_string()),
                );
            }
            ufo_glyph.lib.remove("com.fontcarver.componentData");
            if layer.components.iter().any(|component| {
                component.anchor.is_some()
                    || component.auto_align
                    || component.use_my_metrics
                    || component.round_to_grid
                    || component.smart.is_some()
                    || component.point_match.is_some()
            }) {
                ufo_glyph.lib.insert(
                    "com.fontcarver.componentData".into(),
                    Value::String(extra::build_component_data(&layer.components).to_string()),
                );
            }
            let ufo_layer = target
                .layers
                .get_mut(&layer_name)
                .ok_or_else(|| Error::InvalidName(layer_name.clone()))?;
            if let Some(layer_lib) = layer_lib(layer)? {
                ufo_layer.lib = layer_lib;
            }
            ufo_layer.insert_glyph(ufo_glyph);
        }
    }
    extra::copy_data_and_images(font, master, &mut target)?;
    Ok(target)
}

fn layer_from_norad(
    glyph: &norad::Glyph,
    source_layer: &norad::Layer,
    kind: LayerKind,
    glyph_name: &str,
    layer_name: &str,
    master_id: &MasterId,
    metadata_entries: &mut Vec<ObjectMetadata>,
) -> Result<Layer, Error> {
    let mut layer = Layer {
        kind,
        name: layer_name.to_owned(),
        width: glyph.width,
        vertical_width: (glyph.height != 0.0).then_some(glyph.height),
        contours: glyph
            .contours
            .iter()
            .map(|contour| Contour {
                closed: contour.is_closed(),
                points: contour.points.iter().map(point_from_norad).collect(),
                name: None,
                id: contour.identifier().map(ToString::to_string),
            })
            .collect(),
        components: glyph
            .components
            .iter()
            .map(|component| Component {
                base: GlyphName::from(component.base.to_string()),
                transform: kurbo_affine(component.transform),
                ..Component::default()
            })
            .collect(),
        anchors: glyph
            .anchors
            .iter()
            .map(|anchor| Anchor {
                name: anchor
                    .name
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_default(),
                x: anchor.x,
                y: anchor.y,
            })
            .collect(),
        guides: glyph
            .guidelines
            .iter()
            .map(info::guide_from_norad)
            .collect(),
        background_image: glyph.image.as_ref().map(|image| fc_model::Image {
            file_name: image.file_name().to_string_lossy().into_owned(),
            transform: kurbo_affine(image.transform),
            color: image.color.map(|color| color.to_rgba_string()),
            data: None,
        }),
        ..Layer::default()
    };
    if !source_layer.lib.is_empty() {
        layer.user_data =
            serde_json::json!({ UFO_LAYER_LIB_KEY: dict_to_json(&source_layer.lib)? });
    }
    if matches!(layer.kind, LayerKind::Custom(_)) {
        super::value::insert_json_value(
            &mut layer.user_data,
            UFO_MASTER_KEY,
            serde_json::Value::String(master_id.to_string()),
        );
    }
    metadata::record_glyph_objects(glyph, glyph_name, layer_name, metadata_entries)?;
    if let Some(data) = glyph.lib.get("com.fontcarver.sourceLayer") {
        let data: serde_json::Value = serde_json::from_str(data.as_string().ok_or_else(|| {
            Error::UnsupportedPlist("source layer metadata must be JSON text".into())
        })?)?;
        let mut kind: LayerKind = serde_json::from_value(data["kind"].clone())?;
        match &mut kind {
            LayerKind::Smart { master, .. } | LayerKind::Bracket { master, .. }
                if matches!(layer.kind, LayerKind::Custom(_)) =>
            {
                *master = master_id.clone()
            }
            _ => {
                return Err(Error::UnsupportedPlist(
                    "invalid supplemental source role".into(),
                ))
            }
        }
        layer.kind = kind;
        if let Some(id) = data.get("id") {
            layer.id = serde_json::from_value(id.clone())?;
        }
    }
    if let Some(data) = glyph.lib.get("com.fontcarver.componentData") {
        extra::import_component_data(
            &mut layer.components,
            &serde_json::from_str(data.as_string().ok_or_else(|| {
                Error::UnsupportedPlist("component metadata must be JSON text".into())
            })?)?,
        )?;
    }
    Ok(layer)
}

fn glyph_to_norad(
    glyph: &Glyph,
    layer: &Layer,
    layer_name: &str,
    metadata_entries: &[ObjectMetadata],
) -> Result<norad::Glyph, Error> {
    let mut result = norad::Glyph::new(glyph.name.as_ref());
    result.width = layer.width;
    result.height = layer.vertical_width.unwrap_or_default();
    if layer_name == "public.default" {
        result.codepoints.set(
            glyph
                .unicodes
                .iter()
                .filter_map(|value| char::from_u32(*value)),
        );
        result.note = (!glyph.note.is_empty()).then(|| glyph.note.clone());
    }
    result.contours = layer
        .contours
        .iter()
        .enumerate()
        .map(|(index, contour)| {
            let id = contour.id.as_deref().or_else(|| {
                object_id(
                    metadata_entries,
                    glyph.name.as_ref(),
                    layer_name,
                    "contour",
                    index,
                    None,
                )
            });
            let contour_identifier = identifier(id)?;
            let points = contour
                .points
                .iter()
                .enumerate()
                .map(|(point_index, point)| {
                    let metadata = metadata::lookup(
                        metadata_entries,
                        Some(glyph.name.as_ref()),
                        Some(layer_name),
                        "point",
                        index,
                        Some(point_index),
                    );
                    let mut result = ContourPoint::new(
                        point.x,
                        point.y,
                        point_kind_to_norad(point.kind),
                        point.smooth,
                        point
                            .name
                            .as_deref()
                            .map(Name::new)
                            .transpose()
                            .map_err(|_| {
                                Error::InvalidName(point.name.clone().unwrap_or_default())
                            })?,
                        identifier(metadata.and_then(|entry| entry.identifier.as_deref()))?,
                    );
                    if let Some(lib) = metadata
                        .map(ObjectMetadata::decoded_lib)
                        .transpose()?
                        .flatten()
                    {
                        result.replace_lib(lib);
                    }
                    Ok(result)
                })
                .collect::<Result<_, Error>>()?;
            let mut result = norad::Contour::new(points, contour_identifier);
            if let Some(lib) = metadata::lookup(
                metadata_entries,
                Some(glyph.name.as_ref()),
                Some(layer_name),
                "contour",
                index,
                None,
            )
            .map(ObjectMetadata::decoded_lib)
            .transpose()?
            .flatten()
            {
                result.replace_lib(lib);
            }
            Ok(result)
        })
        .collect::<Result<_, Error>>()?;
    result.components = layer
        .components
        .iter()
        .enumerate()
        .map(|(index, component)| {
            validate_name(component.base.as_ref())?;
            let metadata = metadata::lookup(
                metadata_entries,
                Some(glyph.name.as_ref()),
                Some(layer_name),
                "component",
                index,
                None,
            );
            let mut result = norad::Component::new(
                Name::new(component.base.as_ref())
                    .map_err(|_| Error::InvalidName(component.base.to_string()))?,
                norad_affine(component.transform),
                identifier(metadata.and_then(|entry| entry.identifier.as_deref()))?,
            );
            if let Some(lib) = metadata
                .map(ObjectMetadata::decoded_lib)
                .transpose()?
                .flatten()
            {
                result.replace_lib(lib);
            }
            Ok(result)
        })
        .collect::<Result<_, Error>>()?;
    result.anchors = layer
        .anchors
        .iter()
        .enumerate()
        .map(|(index, anchor)| {
            let metadata = metadata::lookup(
                metadata_entries,
                Some(glyph.name.as_ref()),
                Some(layer_name),
                "anchor",
                index,
                None,
            );
            let name = (!anchor.name.is_empty())
                .then(|| Name::new(&anchor.name))
                .transpose()
                .map_err(|_| Error::InvalidName(anchor.name.clone()))?;
            let color = metadata
                .and_then(|entry| entry.color.as_deref())
                .map(Color::from_str)
                .transpose()
                .map_err(|_| Error::UnsupportedPlist("invalid anchor color".to_owned()))?;
            let mut result = norad::Anchor::new(
                anchor.x,
                anchor.y,
                name,
                color,
                identifier(metadata.and_then(|entry| entry.identifier.as_deref()))?,
            );
            if let Some(lib) = metadata
                .map(ObjectMetadata::decoded_lib)
                .transpose()?
                .flatten()
            {
                result.replace_lib(lib);
            }
            Ok(result)
        })
        .collect::<Result<_, Error>>()?;
    result.guidelines = layer
        .guides
        .iter()
        .enumerate()
        .map(|(index, guide)| {
            let metadata = metadata::lookup(
                metadata_entries,
                Some(glyph.name.as_ref()),
                Some(layer_name),
                "guideline",
                index,
                None,
            );
            let mut result = info::guide_to_norad(
                guide,
                metadata.and_then(|entry| entry.identifier.as_deref()),
            )?;
            if let Some(lib) = metadata
                .map(ObjectMetadata::decoded_lib)
                .transpose()?
                .flatten()
            {
                result.replace_lib(lib);
            }
            Ok(result)
        })
        .collect::<Result<_, Error>>()?;
    result.image = layer
        .background_image
        .as_ref()
        .map(|image| {
            let color = image
                .color
                .as_deref()
                .map(Color::from_str)
                .transpose()
                .map_err(|_| Error::UnsupportedPlist("invalid image color".to_owned()))?;
            norad::Image::new(
                image.file_name.clone().into(),
                color,
                norad_affine(image.transform),
            )
            .map_err(|error| Error::UnsupportedPlist(error.to_string()))
        })
        .transpose()?;
    Ok(result)
}

fn point_from_norad(point: &ContourPoint) -> Point {
    Point {
        x: point.x,
        y: point.y,
        kind: match point.typ {
            PointType::Move => PointKind::Move,
            PointType::Line => PointKind::Line,
            PointType::Curve => PointKind::Curve,
            PointType::QCurve => PointKind::QCurve,
            PointType::OffCurve => PointKind::OffCurve,
        },
        smooth: point.smooth,
        name: point.name.as_ref().map(ToString::to_string),
        ..Point::default()
    }
}

fn point_kind_to_norad(kind: PointKind) -> PointType {
    match kind {
        PointKind::Move => PointType::Move,
        PointKind::Line => PointType::Line,
        PointKind::Curve => PointType::Curve,
        PointKind::QCurve => PointType::QCurve,
        PointKind::OffCurve => PointType::OffCurve,
    }
}

fn glyph_order(source: &norad::Font) -> Vec<String> {
    let mut result = Vec::new();
    let mut seen = HashSet::new();
    if let Some(order) = source
        .lib
        .get("public.glyphOrder")
        .and_then(Value::as_array)
    {
        for name in order.iter().filter_map(Value::as_string) {
            if seen.insert(name.to_owned()) {
                result.push(name.to_owned());
            }
        }
    }
    for layer in source.iter_layers() {
        for glyph in layer.iter() {
            let name = glyph.name().to_string();
            if seen.insert(name.clone()) {
                result.push(name);
            }
        }
    }
    result
}

fn collect_layer_names(font: &Font, master: &MasterId) -> Vec<String> {
    let mut names = vec!["public.default".to_owned()];
    let mut seen = HashSet::from(["public.default".to_owned()]);
    for layer in font
        .glyphs
        .values()
        .flat_map(|glyph| glyph.layers.iter())
        .filter(|layer| layer_belongs(font, layer, master))
    {
        let name = model_layer_name(layer, master);
        if seen.insert(name.clone()) {
            names.push(name);
        }
    }
    names
}

pub(crate) fn layer_belongs(font: &Font, layer: &Layer, master: &MasterId) -> bool {
    match &layer.kind {
        LayerKind::Master(id) | LayerKind::Background(id) => id == master,
        LayerKind::Smart { master: id, .. }
        | LayerKind::Brace { master: id, .. }
        | LayerKind::Bracket { master: id, .. } => id == master,
        LayerKind::Backup { of, .. } => of == master,
        LayerKind::Custom(_) => layer
            .user_data
            .get(UFO_MASTER_KEY)
            .and_then(serde_json::Value::as_str)
            .map_or_else(
                || master == &font.default_master().id,
                |owner| owner == master.to_string(),
            ),
    }
}

pub(crate) fn model_layer_name(layer: &Layer, master: &MasterId) -> String {
    match &layer.kind {
        LayerKind::Master(id) if id == master => "public.default".to_owned(),
        LayerKind::Background(_) => "public.background".to_owned(),
        LayerKind::Custom(name) if !name.is_empty() => name.clone(),
        _ if !layer.name.is_empty() => layer.name.clone(),
        LayerKind::Smart { .. } => format!("smart.{}", layer.id),
        LayerKind::Brace { .. } => format!("brace.{}", layer.id),
        LayerKind::Bracket { .. } => format!("bracket.{}", layer.id),
        LayerKind::Backup { .. } => format!("backup.{}", layer.id),
        LayerKind::Master(_) | LayerKind::Custom(_) => "public.default".to_owned(),
    }
}

fn glyph_lib(glyph: &Glyph, layer_name: &str) -> Result<Option<Dictionary>, Error> {
    let value = if layer_name == "public.default" {
        glyph.user_data.get(UFO_GLYPH_LIB_KEY)
    } else {
        glyph
            .user_data
            .get(UFO_LAYER_GLYPH_LIBS_KEY)
            .and_then(|value| value.get(layer_name))
    };
    value.map(json_to_dict).transpose()
}

fn layer_lib(layer: &Layer) -> Result<Option<Dictionary>, Error> {
    layer
        .user_data
        .get(UFO_LAYER_LIB_KEY)
        .map(json_to_dict)
        .transpose()
}

fn assign_glyph_groups(font: &mut Font) {
    for (name, members) in &font.kerning_groups.side1 {
        for member in members {
            if let Some(glyph) = font.glyphs.get_mut(member) {
                glyph.kern_group_side1 = Some(name.clone());
            }
        }
    }
    for (name, members) in &font.kerning_groups.side2 {
        for member in members {
            if let Some(glyph) = font.glyphs.get_mut(member) {
                glyph.kern_group_side2 = Some(name.clone());
            }
        }
    }
}

fn object_id<'a>(
    entries: &'a [ObjectMetadata],
    glyph: &str,
    layer: &str,
    kind: &str,
    index: usize,
    point: Option<usize>,
) -> Option<&'a str> {
    metadata::lookup(entries, Some(glyph), Some(layer), kind, index, point)
        .and_then(|entry| entry.identifier.as_deref())
}

fn identifier(value: Option<&str>) -> Result<Option<Identifier>, Error> {
    value
        .map(Identifier::new)
        .transpose()
        .map_err(|_| Error::InvalidIdentifier(value.unwrap_or_default().to_owned()))
}

fn valid_name(value: &str) -> Result<Name, Error> {
    Name::new(value).map_err(|_| Error::InvalidName(value.to_owned()))
}

fn validate_name(value: &str) -> Result<(), Error> {
    valid_name(value).map(|_| ())
}

fn kurbo_affine(value: AffineTransform) -> kurbo::Affine {
    kurbo::Affine::new([
        value.x_scale,
        value.xy_scale,
        value.yx_scale,
        value.y_scale,
        value.x_offset,
        value.y_offset,
    ])
}

fn norad_affine(value: kurbo::Affine) -> AffineTransform {
    let values = value.as_coeffs();
    AffineTransform {
        x_scale: values[0],
        xy_scale: values[1],
        yx_scale: values[2],
        y_scale: values[3],
        x_offset: values[4],
        y_offset: values[5],
    }
}
