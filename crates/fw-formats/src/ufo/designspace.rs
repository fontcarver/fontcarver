//! Designspace import and export.

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
    path::PathBuf,
    str::FromStr,
};

use fw_model::{Axis, DesignLocation, Font, Instance, LayerKind, Master, MasterId, Tag};
use norad::designspace::{
    Axis as NoradAxis, AxisMapping, DesignSpaceDocument, Dimension, Instance as NoradInstance,
    Source,
};
use tracing::warn;

use super::{convert, value, writer, UfoWriteOptions};
use crate::Error;

const RULES_KEY: &str = "ufo.designspace.rules";
const LIB_KEY: &str = "ufo.designspace.lib";
const AXIS_MAPPINGS_KEY: &str = "ufo.designspace.axis_mappings";

pub(crate) fn read(path: &Path) -> Result<Font, Error> {
    let document = DesignSpaceDocument::load(path)?;
    let axes = document
        .axes
        .iter()
        .map(axis_from_norad)
        .collect::<Result<Vec<_>, _>>()?;
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    let mut source_indexes = document
        .sources
        .iter()
        .enumerate()
        .filter(|(_, source)| source.layer.is_none())
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let default_index = source_indexes
        .iter()
        .position(|index| source_is_default(&document.sources[*index], &document.axes));
    if let Some(default_index) = default_index {
        source_indexes.swap(0, default_index);
    }
    if source_indexes.is_empty() {
        return Err(Error::MissingSourceMaster(path.display().to_string()));
    }

    let mut result: Option<Font> = None;
    let mut masters_by_file = HashMap::<String, MasterId>::new();
    for index in source_indexes {
        let source = &document.sources[index];
        let source_path = resolve_source(base, &source.filename);
        let ufo = norad::Font::load_requested_data(&source_path, norad::DataRequest::all())?;
        let mut imported = convert::from_norad(&ufo, &source_path)?;
        let old_id = imported.default_master().id.clone();
        let mut master = imported.masters.remove(0);
        master.name = source
            .name
            .clone()
            .or_else(|| source.stylename.clone())
            .filter(|name| !name.is_empty())
            .unwrap_or(master.name);
        master.location = location_from_norad(&source.location, &axes, &document.axes);
        replace_master_id(&mut imported, &old_id, &master.id);
        masters_by_file.insert(normalized_filename(&source.filename), master.id.clone());

        if let Some(font) = &mut result {
            merge_master(font, imported, master);
        } else {
            imported.masters = vec![master];
            result = Some(imported);
        }
    }
    let mut font = result.ok_or_else(|| Error::MissingSourceMaster(path.display().to_string()))?;
    font.axes = axes;
    font.instances = document
        .instances
        .iter()
        .map(|instance| instance_from_norad(instance, &font.axes, &document.axes))
        .collect();

    for source in document
        .sources
        .iter()
        .filter(|source| source.layer.is_some())
    {
        import_brace_layer(&mut font, source, base, &document.axes, &masters_by_file)?;
    }

    if !document.rules.rules.is_empty() {
        warn!(
            count = document.rules.rules.len(),
            "designspace rules are preserved but not interpreted"
        );
        font.extra
            .lib
            .insert(RULES_KEY.to_owned(), serde_json::to_value(&document.rules)?);
    }
    if !document.lib.is_empty() {
        font.extra
            .lib
            .insert(LIB_KEY.to_owned(), value::dict_to_json(&document.lib)?);
    }
    if let Some(axis_mappings) = &document.axis_mappings {
        font.extra.lib.insert(
            AXIS_MAPPINGS_KEY.to_owned(),
            serde_json::to_value(axis_mappings)?,
        );
    }
    if let Some(family) = document
        .sources
        .iter()
        .find_map(|source| source.familyname.clone())
    {
        font.info.family_name = family;
    }
    Ok(font)
}

pub(crate) fn write(font: &Font, dir: &Path) -> Result<PathBuf, Error> {
    fs::create_dir_all(dir).map_err(|source| Error::io(dir, source))?;
    let family = filename_fragment(&font.info.family_name, "Untitled");
    let mut sources = Vec::new();
    for (index, master) in font.masters.iter().enumerate() {
        let master_name = filename_fragment(&master.name, &format!("Master{}", index + 1));
        let file_name = format!("{family}-{master_name}.ufo");
        let ufo_path = dir.join(&file_name);
        super::write_ufo(font, &master.id, &ufo_path, &UfoWriteOptions::default())?;
        sources.push(Source {
            familyname: (!font.info.family_name.is_empty()).then(|| font.info.family_name.clone()),
            stylename: (!master.name.is_empty()).then(|| master.name.clone()),
            name: (!master.name.is_empty()).then(|| master.name.clone()),
            filename: file_name.clone(),
            layer: None,
            location: location_to_norad(&master.location, &font.axes),
        });
        let mut brace_layers = HashSet::new();
        for layer in font.glyphs.values().flat_map(|glyph| &glyph.layers).filter(
            |layer| matches!(&layer.kind, LayerKind::Brace { master: id, .. } if id == &master.id),
        ) {
            let layer_name = convert::model_layer_name(layer, &master.id);
            if !brace_layers.insert(layer_name.clone()) {
                continue;
            }
            let LayerKind::Brace { location, .. } = &layer.kind else {
                continue;
            };
            sources.push(Source {
                familyname: (!font.info.family_name.is_empty())
                    .then(|| font.info.family_name.clone()),
                stylename: None,
                name: Some(layer_name.clone()),
                filename: file_name.clone(),
                layer: Some(layer_name),
                location: location_to_norad(location, &font.axes),
            });
        }
    }

    let axes = font.axes.iter().map(axis_to_norad).collect();
    let instances = font
        .instances
        .iter()
        .map(|instance| instance_to_norad(instance, &font.axes))
        .collect();
    let rules = font
        .extra
        .lib
        .get(RULES_KEY)
        .cloned()
        .map(serde_json::from_value)
        .transpose()?
        .unwrap_or_default();
    let axis_mappings = font
        .extra
        .lib
        .get(AXIS_MAPPINGS_KEY)
        .cloned()
        .map(serde_json::from_value)
        .transpose()?;
    let lib = font
        .extra
        .lib
        .get(LIB_KEY)
        .map(value::json_to_dict)
        .transpose()?
        .unwrap_or_default();
    let document = DesignSpaceDocument {
        format: 5.0,
        axes,
        axis_mappings,
        rules,
        sources,
        instances,
        lib,
    };
    let path = dir.join(format!("{family}.designspace"));
    let temporary = dir.join(format!(
        ".{family}.designspace-stage-{}",
        std::process::id()
    ));
    document.save(&temporary)?;
    let write_result = writer::write_file_incrementally(&temporary, &path);
    let cleanup_result =
        fs::remove_file(&temporary).map_err(|source| Error::io(&temporary, source));
    write_result?;
    cleanup_result?;
    Ok(path)
}

fn import_brace_layer(
    font: &mut Font,
    source: &Source,
    base: &Path,
    axes: &[NoradAxis],
    masters_by_file: &HashMap<String, MasterId>,
) -> Result<(), Error> {
    let Some(layer_name) = source.layer.as_deref() else {
        return Ok(());
    };
    let master_id = masters_by_file
        .get(&normalized_filename(&source.filename))
        .cloned()
        .ok_or_else(|| Error::MissingSourceMaster(source.filename.clone()))?;
    let path = resolve_source(base, &source.filename);
    let ufo = norad::Font::load_requested_data(&path, norad::DataRequest::all())?;
    let imported = convert::from_norad(&ufo, &path)?;
    let location = location_from_norad(&source.location, &font.axes, axes);
    for (name, mut imported_glyph) in imported.glyphs {
        let selected = imported_glyph
            .layers
            .iter()
            .position(|layer| layer.name == layer_name);
        let Some(selected) = selected else {
            continue;
        };
        let mut selected = imported_glyph.layers.remove(selected);
        selected.kind = LayerKind::Brace {
            master: master_id.clone(),
            location: location.clone(),
        };
        if let Some(glyph) = font.glyphs.get_mut(name.as_ref()) {
            glyph.layers.push(selected);
        } else {
            let mut glyph = imported_glyph;
            glyph.layers = vec![selected];
            font.glyphs.insert(name, glyph);
        }
    }
    Ok(())
}

fn merge_master(font: &mut Font, mut imported: Font, master: Master) {
    font.masters.push(master);
    for (name, mut imported_glyph) in imported.glyphs.drain(..) {
        if let Some(glyph) = font.glyphs.get_mut(name.as_ref()) {
            glyph.layers.append(&mut imported_glyph.layers);
        } else {
            font.glyphs.insert(name, imported_glyph);
        }
    }
}

fn replace_master_id(font: &mut Font, old: &MasterId, new: &MasterId) {
    for layer in font
        .glyphs
        .values_mut()
        .flat_map(|glyph| glyph.layers.iter_mut())
    {
        match &mut layer.kind {
            LayerKind::Master(id) | LayerKind::Background(id) if id == old => *id = new.clone(),
            LayerKind::Brace { master, .. } | LayerKind::Bracket { master, .. }
                if master == old =>
            {
                *master = new.clone();
            }
            LayerKind::Backup { of, .. } if of == old => *of = new.clone(),
            _ => {}
        }
    }
}

fn axis_from_norad(axis: &NoradAxis) -> Result<Axis, Error> {
    let tag = Tag::from_str(&axis.tag).map_err(|_| Error::InvalidAxisTag(axis.tag.clone()))?;
    let values = axis.values.as_deref().unwrap_or_default();
    Ok(Axis {
        tag,
        name: axis.name.clone(),
        min: axis
            .minimum
            .or_else(|| min_value(values))
            .unwrap_or(axis.default) as f64,
        default: f64::from(axis.default),
        max: axis
            .maximum
            .or_else(|| max_value(values))
            .unwrap_or(axis.default) as f64,
        hidden: axis.hidden,
        map: axis
            .map
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|mapping| (f64::from(mapping.input), f64::from(mapping.output)))
            .collect(),
    })
}

fn axis_to_norad(axis: &Axis) -> NoradAxis {
    NoradAxis {
        name: axis.name.clone(),
        tag: axis.tag.to_string(),
        default: axis.default as f32,
        hidden: axis.hidden,
        minimum: Some(axis.min as f32),
        maximum: Some(axis.max as f32),
        values: None,
        map: (!axis.map.is_empty()).then(|| {
            axis.map
                .iter()
                .map(|&(input, output)| AxisMapping {
                    input: input as f32,
                    output: output as f32,
                })
                .collect()
        }),
        label_names: Vec::new(),
    }
}

fn source_is_default(source: &Source, axes: &[NoradAxis]) -> bool {
    axes.iter().all(|axis| {
        let expected = map_axis_default(axis);
        let actual = source
            .location
            .iter()
            .find(|dimension| dimension.name == axis.name)
            .and_then(|dimension| {
                dimension
                    .xvalue
                    .or_else(|| dimension.uservalue.map(|value| map_user_value(axis, value)))
            })
            .unwrap_or(expected);
        (actual - expected).abs() <= f32::EPSILON * 8.0
    })
}

fn map_axis_default(axis: &NoradAxis) -> f32 {
    axis.map
        .as_deref()
        .unwrap_or_default()
        .iter()
        .find(|mapping| (mapping.input - axis.default).abs() <= f32::EPSILON)
        .map_or(axis.default, |mapping| mapping.output)
}

fn location_from_norad(
    dimensions: &[Dimension],
    axes: &[Axis],
    source_axes: &[NoradAxis],
) -> DesignLocation {
    axes.iter()
        .filter_map(|axis| {
            let source_axis = source_axes
                .iter()
                .find(|candidate| candidate.name == axis.name)?;
            let value = dimensions
                .iter()
                .find(|dimension| dimension.name == axis.name)
                .and_then(|dimension| {
                    dimension.xvalue.or_else(|| {
                        dimension
                            .uservalue
                            .map(|value| map_user_value(source_axis, value))
                    })
                })
                .unwrap_or_else(|| map_axis_default(source_axis));
            Some((axis.tag, f64::from(value)))
        })
        .collect()
}

fn location_to_norad(location: &DesignLocation, axes: &[Axis]) -> Vec<Dimension> {
    axes.iter()
        .map(|axis| Dimension {
            name: axis.name.clone(),
            uservalue: None,
            xvalue: Some(location.get(&axis.tag).copied().unwrap_or(axis.default) as f32),
            yvalue: None,
        })
        .collect()
}

fn instance_from_norad(
    instance: &NoradInstance,
    axes: &[Axis],
    source_axes: &[NoradAxis],
) -> Instance {
    Instance {
        name: instance
            .name
            .clone()
            .or_else(|| instance.stylename.clone())
            .unwrap_or_default(),
        location: location_from_norad(&instance.location, axes, source_axes),
        style_name: instance.stylename.clone().unwrap_or_default(),
        family_name: instance.familyname.clone(),
        active: true,
        is_variable_named_instance: true,
        user_data: if instance.lib.is_empty() {
            serde_json::Value::Null
        } else {
            value::dict_to_json(&instance.lib).unwrap_or(serde_json::Value::Null)
        },
    }
}

fn instance_to_norad(instance: &Instance, axes: &[Axis]) -> NoradInstance {
    NoradInstance {
        familyname: instance.family_name.clone(),
        stylename: (!instance.style_name.is_empty()).then(|| instance.style_name.clone()),
        name: (!instance.name.is_empty()).then(|| instance.name.clone()),
        filename: None,
        postscriptfontname: None,
        stylemapfamilyname: None,
        stylemapstylename: None,
        location: location_to_norad(&instance.location, axes),
        lib: instance
            .user_data
            .as_object()
            .and_then(|_| value::json_to_dict(&instance.user_data).ok())
            .unwrap_or_default(),
    }
}

fn map_user_value(axis: &NoradAxis, value: f32) -> f32 {
    axis.map
        .as_deref()
        .unwrap_or_default()
        .iter()
        .find(|mapping| (mapping.input - value).abs() <= f32::EPSILON)
        .map_or(value, |mapping| mapping.output)
}

fn resolve_source(base: &Path, filename: &str) -> PathBuf {
    base.join(filename.replace('\\', "/"))
}

fn normalized_filename(filename: &str) -> String {
    filename.replace('\\', "/")
}

fn filename_fragment(value: &str, fallback: &str) -> String {
    let value = value
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let value = value.trim_matches(['-', '.']);
    if value.is_empty() {
        fallback.to_owned()
    } else {
        value.to_owned()
    }
}

fn min_value(values: &[f32]) -> Option<f32> {
    values.iter().copied().reduce(f32::min)
}

fn max_value(values: &[f32]) -> Option<f32> {
    values.iter().copied().reduce(f32::max)
}
