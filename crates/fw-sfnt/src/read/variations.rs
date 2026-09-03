use fw_model::{Axis, DesignLocation, Font, Instance, Tag};
use read_fonts::{FontRef, TableProvider};
use skrifa::MetadataProvider;

use crate::Error;

pub(crate) fn apply(face: &FontRef<'_>, font: &mut Font) -> Result<(), Error> {
    if face
        .table_data(read_fonts::types::Tag::new(b"fvar"))
        .is_none()
    {
        return Ok(());
    }
    let raw_axes = face.axes();
    font.axes = raw_axes
        .iter()
        .map(|axis| Axis {
            tag: model_tag(axis.tag()),
            name: super::names::name_by_id(face, axis.name_id().to_u16()).unwrap_or_default(),
            min: f64::from(axis.min_value()),
            default: f64::from(axis.default_value()),
            max: f64::from(axis.max_value()),
            hidden: axis.is_hidden(),
            map: Vec::new(),
        })
        .collect();
    let fvar = face.fvar().map_err(|error| Error::parse("fvar", error))?;
    if usize::from(fvar.axis_count()) != font.axes.len() {
        return Err(Error::Parse {
            table: "fvar",
            reason: "axis records are truncated".to_owned(),
        });
    }
    if face
        .table_data(read_fonts::types::Tag::new(b"avar"))
        .is_some()
    {
        let avar = face.avar().map_err(|error| Error::parse("avar", error))?;
        for (axis, map) in font.axes.iter_mut().zip(avar.axis_segment_maps().iter()) {
            let map = map.map_err(|error| Error::parse("avar", error))?;
            axis.map = map
                .axis_value_maps()
                .iter()
                .map(|value| {
                    (
                        f64::from(value.from_coordinate().to_f32()),
                        f64::from(value.to_coordinate().to_f32()),
                    )
                })
                .collect();
        }
    }
    font.instances = face
        .named_instances()
        .iter()
        .map(|instance| {
            let name = super::names::name_by_id(face, instance.subfamily_name_id().to_u16())
                .unwrap_or_default();
            let location = font
                .axes
                .iter()
                .zip(instance.user_coords())
                .map(|(axis, coordinate)| (axis.tag, f64::from(coordinate)))
                .collect();
            Instance {
                name: name.clone(),
                location,
                style_name: name,
                active: true,
                is_variable_named_instance: true,
                ..Instance::default()
            }
        })
        .collect();
    Ok(())
}

pub(crate) fn default_location(axes: &[Axis]) -> DesignLocation {
    axes.iter().map(|axis| (axis.tag, axis.default)).collect()
}

fn model_tag(tag: read_fonts::types::Tag) -> Tag {
    Tag(tag.to_be_bytes())
}
