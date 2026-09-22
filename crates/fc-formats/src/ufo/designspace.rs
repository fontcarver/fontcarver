//! Designspace import and export.

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
    path::PathBuf,
    str::FromStr,
};

use fc_model::{Axis, DesignLocation, Font, Instance, LayerKind, Master, MasterId, Tag};
use norad::designspace::{
    Axis as NoradAxis, AxisMapping, DesignSpaceDocument, Dimension, Instance as NoradInstance,
    Source,
};

use super::{convert, extra::feature_metadata, value, writer, UfoWriteOptions};
use crate::Error;

const LIB_KEY: &str = "ufo.designspace.lib";
const AXIS_MAPPINGS_KEY: &str = "ufo.designspace.axis_mappings";
const AXES_KEY: &str = "ufo.designspace.axes";
const INSTANCE_METADATA_KEY: &str = "ufo.designspace.instance";

pub(crate) fn read(path: &Path) -> Result<Font, Error> {
    let document = DesignSpaceDocument::load(path)?;
    let writer_policy = feature_metadata::read_policy(&document.lib)?;
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
    let mut ufos_by_file = HashMap::<String, norad::Font>::new();
    for index in source_indexes {
        let source = &document.sources[index];
        let source_path = resolve_source(base, &source.filename);
        let normalized_filename = normalized_filename(&source.filename);
        let mut ufo = if let Some(ufo) = ufos_by_file.get(&normalized_filename) {
            ufo.clone()
        } else {
            let ufo = norad::Font::load_requested_data(&source_path, norad::DataRequest::all())?;
            ufos_by_file.insert(normalized_filename.clone(), ufo.clone());
            ufo
        };
        // Designspace configuration takes precedence over each UFO's lib.
        if let Some(policy) = &writer_policy {
            feature_metadata::write_policy(policy, &mut ufo.lib);
        }
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
        masters_by_file.insert(normalized_filename, master.id.clone());

        if let Some(font) = &mut result {
            if font.feature_settings.writer_policy != imported.feature_settings.writer_policy {
                return Err(Error::UnsupportedPlist(
                    "designspace masters have different feature writer policies".to_owned(),
                ));
            }
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
        import_brace_layer(&mut font, source, &document.axes, &masters_by_file)?;
    }

    super::conditional::read(&mut font, &document.rules, &document.lib)?;
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
    let axis_metadata = document
        .axes
        .iter()
        .filter(|axis| axis.values.is_some() || !axis.label_names.is_empty())
        .map(|axis| {
            (
                axis.tag.clone(),
                serde_json::json!({
                    "values": axis.values,
                    "label_names": axis.label_names,
                }),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    if !axis_metadata.is_empty() {
        font.extra.lib.insert(
            AXES_KEY.to_owned(),
            serde_json::Value::Object(axis_metadata),
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
    if font.axes.is_empty() {
        let master = font.default_master();
        let master_name = filename_fragment(&master.name, "Regular");
        let ufo_path = dir.join(format!("{family}-{master_name}.ufo"));
        super::write_ufo(font, &master.id, &ufo_path, &UfoWriteOptions::default())?;
        return Err(Error::NoAxes);
    }
    let (expanded, rules, rule_lib) = super::conditional::prepare(font)?;
    let font = &expanded;
    let mut sources = Vec::new();
    let mut file_name_counts = HashMap::<String, usize>::new();
    for (index, master) in font.masters.iter().enumerate() {
        let master_name = filename_fragment(&master.name, &format!("Master{}", index + 1));
        let base_name = format!("{family}-{master_name}");
        let count = file_name_counts.entry(base_name.clone()).or_default();
        *count += 1;
        let file_name = if *count == 1 {
            format!("{base_name}.ufo")
        } else {
            format!("{base_name}-{count}.ufo")
        };
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
                location: location_to_norad(
                    &font
                        .intermediate_location(&master.id, location)
                        .map_err(|error| Error::UnsupportedPlist(error.to_string()))?,
                    &font.axes,
                ),
            });
        }
    }

    let axes = font
        .axes
        .iter()
        .map(|axis| axis_to_norad(axis, font))
        .collect::<Result<_, _>>()?;
    let instances = font
        .instances
        .iter()
        .map(|instance| instance_to_norad(instance, &font.axes))
        .collect();
    let axis_mappings = font
        .extra
        .lib
        .get(AXIS_MAPPINGS_KEY)
        .cloned()
        .map(serde_json::from_value)
        .transpose()?;
    let mut lib = font
        .extra
        .lib
        .get(LIB_KEY)
        .map(value::json_to_dict)
        .transpose()?
        .unwrap_or_default();
    lib.remove("com.fontcarver.bracketAliases");
    lib.extend(rule_lib);
    feature_metadata::write_policy(&font.feature_settings.writer_policy, &mut lib);
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
    let location = location_from_norad(&source.location, &font.axes, axes);
    for layer in font
        .glyphs
        .values_mut()
        .flat_map(|glyph| glyph.layers.iter_mut())
    {
        let belongs_to_master = layer
            .user_data
            .get(convert::UFO_MASTER_KEY)
            .and_then(serde_json::Value::as_str)
            .is_some_and(|owner| owner == master_id.to_string());
        if layer.name == layer_name
            && belongs_to_master
            && matches!(layer.kind, LayerKind::Custom(_))
        {
            layer.kind = LayerKind::Brace {
                master: master_id.clone(),
                location: location.clone(),
            };
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
            LayerKind::Smart { master, .. }
            | LayerKind::Brace { master, .. }
            | LayerKind::Bracket { master, .. }
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

fn axis_to_norad(axis: &Axis, font: &Font) -> Result<NoradAxis, Error> {
    let metadata = font
        .extra
        .lib
        .get(AXES_KEY)
        .and_then(|axes| axes.get(axis.tag.to_string()));
    let values = metadata
        .and_then(|metadata| metadata.get("values"))
        .filter(|values| !values.is_null())
        .cloned()
        .map(serde_json::from_value)
        .transpose()?;
    let label_names = metadata
        .and_then(|metadata| metadata.get("label_names"))
        .cloned()
        .map(serde_json::from_value)
        .transpose()?
        .unwrap_or_default();
    Ok(NoradAxis {
        name: axis.name.clone(),
        tag: axis.tag.to_string(),
        default: axis.default as f32,
        hidden: axis.hidden,
        minimum: values.is_none().then_some(axis.min as f32),
        maximum: values.is_none().then_some(axis.max as f32),
        values,
        map: (!axis.map.is_empty()).then(|| {
            axis.map
                .iter()
                .map(|&(input, output)| AxisMapping {
                    input: input as f32,
                    output: output as f32,
                })
                .collect()
        }),
        label_names,
    })
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
    map_user_value(axis, axis.default)
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
            xvalue: Some(
                location
                    .get(&axis.tag)
                    .copied()
                    .unwrap_or_else(|| axis.user_to_design(axis.default)) as f32,
            ),
            yvalue: None,
        })
        .collect()
}

fn instance_from_norad(
    instance: &NoradInstance,
    axes: &[Axis],
    source_axes: &[NoradAxis],
) -> Instance {
    let mut user_data = if instance.lib.is_empty() {
        serde_json::Value::Null
    } else {
        value::dict_to_json(&instance.lib).unwrap_or(serde_json::Value::Null)
    };
    let metadata = serde_json::json!({
        "filename": instance.filename,
        "postscriptfontname": instance.postscriptfontname,
        "stylemapfamilyname": instance.stylemapfamilyname,
        "stylemapstylename": instance.stylemapstylename,
    });
    if metadata
        .as_object()
        .is_some_and(|metadata| metadata.values().any(|value| !value.is_null()))
    {
        value::insert_json_value(&mut user_data, INSTANCE_METADATA_KEY, metadata);
    }
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
        user_data,
    }
}

fn instance_to_norad(instance: &Instance, axes: &[Axis]) -> NoradInstance {
    let metadata = instance.user_data.get(INSTANCE_METADATA_KEY);
    let mut lib = instance.user_data.clone();
    if let Some(lib) = lib.as_object_mut() {
        lib.remove(INSTANCE_METADATA_KEY);
    }
    NoradInstance {
        familyname: instance.family_name.clone(),
        stylename: (!instance.style_name.is_empty()).then(|| instance.style_name.clone()),
        name: (!instance.name.is_empty()).then(|| instance.name.clone()),
        filename: instance_metadata_string(metadata, "filename"),
        postscriptfontname: instance_metadata_string(metadata, "postscriptfontname"),
        stylemapfamilyname: instance_metadata_string(metadata, "stylemapfamilyname"),
        stylemapstylename: instance_metadata_string(metadata, "stylemapstylename"),
        location: location_to_norad(&instance.location, axes),
        lib: lib
            .as_object()
            .and_then(|lib| (!lib.is_empty()).then_some(()))
            .and_then(|_| value::json_to_dict(&lib).ok())
            .unwrap_or_default(),
    }
}

fn instance_metadata_string(metadata: Option<&serde_json::Value>, key: &str) -> Option<String> {
    metadata
        .and_then(|metadata| metadata.get(key))
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
}

fn map_user_value(axis: &NoradAxis, value: f32) -> f32 {
    let model = Axis {
        map: axis
            .map
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|mapping| (f64::from(mapping.input), f64::from(mapping.output)))
            .collect(),
        ..Axis::default()
    };
    model.user_to_design(f64::from(value)) as f32
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
