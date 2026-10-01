//! Preserve cubic source curves while encoding CFF2 variation deltas.
use crate::{Error, ExportWarning};
use fc_geom::variation::{glyph_sources, interpolated_path, normalized_location};
use fc_model::{DesignLocation, Font, GlyphName};
use fc_sfnt::write::{
    cff2_outline, private_layout, Cff2Delta, Cff2Glyph, Cff2Outline, Cff2Private,
};
use fontdrasil::{
    types::Tag,
    variations::{RoundingBehaviour, VariationModel},
};
use kurbo::{Rect, Shape};
use read_fonts::{FontRef, TableProvider};
use std::collections::{HashMap, HashSet};
use write_fonts::{from_obj::ToOwnedTable, FontBuilder};

fn invalid(error: impl std::fmt::Display) -> Error {
    Error::InvalidSource(error.to_string())
}

pub(crate) fn convert(
    sfnt: &[u8],
    font: &Font,
    order: &[GlyphName],
    warnings: &mut Vec<ExportWarning>,
) -> Result<Vec<u8>, Error> {
    let axis_order: Vec<_> = font.axes.iter().map(|a| Tag::new(&a.tag.0)).collect();
    let default = font
        .complete_location(&DesignLocation::new())
        .map_err(invalid)?;
    let mut glyphs = Vec::new();
    let mut bounds = Vec::new();
    for name in order {
        if name.as_ref() == ".notdef" && font.glyph(name).is_none() {
            glyphs.push(Cff2Glyph {
                outline: Cff2Outline::default(),
                deltas: vec![],
            });
            bounds.push(None);
            continue;
        }
        let mut locations = HashMap::new();
        let mut visited = HashSet::new();
        collect_locations(font, name, &mut visited, &mut locations, 0)?;
        let path = interpolated_path(font, name.as_ref(), &default).map_err(invalid)?;
        bounds.push((!path.elements().is_empty()).then(|| path.bounding_box()));
        let outline = cff2_outline(&path).map_err(invalid)?;
        let mut values = HashMap::new();
        for (normalized, location) in locations {
            let instance =
                cff2_outline(&interpolated_path(font, name.as_ref(), &location).map_err(invalid)?)
                    .map_err(invalid)?;
            if instance.operators != outline.operators {
                return Err(invalid(format!(
                    "{name}: decomposed CFF2 sources are incompatible"
                )));
            }
            values.insert(normalized, instance.operands);
        }
        let model = VariationModel::new(values.keys().cloned().collect(), axis_order.clone());
        let deltas = model
            .deltas_with_rounding::<f64, f64>(&values, RoundingBehaviour::None)
            .map_err(invalid)?;
        let deltas = deltas
            .into_iter()
            .filter(|(region, _)| !region.is_default())
            .map(|(region, values)| Cff2Delta {
                region: axis_order
                    .iter()
                    .map(|tag| {
                        region.get(tag).map_or([0.0; 3], |tent| {
                            [tent.min.to_f64(), tent.peak.to_f64(), tent.max.to_f64()]
                        })
                    })
                    .collect(),
                values,
            })
            .collect();
        glyphs.push(Cff2Glyph { outline, deltas });
    }
    let private = private_values(font, &axis_order, warnings)?;
    let cff =
        fc_sfnt::write::write_cff2(font.info.units_per_em, font.axes.len(), &glyphs, &private)
            .map_err(invalid)?;
    let face = FontRef::new(sfnt).map_err(|e| Error::parse("variable CFF2 source", e))?;
    let mut builder = FontBuilder::new();
    for record in face.table_directory().table_records() {
        if [
            *b"glyf", *b"loca", *b"gvar", *b"CFF ", *b"CFF2", *b"maxp", *b"head", *b"post",
            *b"vhea", *b"vmtx", *b"VORG", *b"fpgm", *b"prep", *b"cvt ", *b"gasp",
        ]
        .contains(&record.tag().to_be_bytes())
        {
            continue;
        }
        let data = face
            .table_data(record.tag())
            .ok_or_else(|| Error::parse("CFF2 source table", record.tag()))?;
        builder.add_raw(record.tag(), data.as_bytes());
    }
    let mut head: write_fonts::tables::head::Head = face
        .head()
        .map_err(|e| Error::parse("head", e))?
        .to_owned_table();
    let bound = bounds
        .iter()
        .flatten()
        .copied()
        .reduce(|a, b| a.union(b))
        .unwrap_or(Rect::ZERO);
    let signed = |value: f64| -> Result<i16, Error> {
        if !value.is_finite() || !(-32768.0..=32767.0).contains(&value) {
            Err(invalid("CFF2 bounds exceed signed 16-bit range"))
        } else {
            Ok(value as i16)
        }
    };
    head.x_min = signed(bound.x0.floor())?;
    head.y_min = signed(bound.y0.floor())?;
    head.x_max = signed(bound.x1.ceil())?;
    head.y_max = signed(bound.y1.ceil())?;
    head.index_to_loc_format = 0;
    builder
        .add_table(&head)
        .map_err(|e| Error::write("head", e))?;
    let mut maxp = vec![0, 0, 0x50, 0];
    maxp.extend_from_slice(&(order.len() as u16).to_be_bytes());
    builder.add_raw(Tag::new(b"maxp"), maxp);
    let mut post = face
        .table_data(Tag::new(b"post"))
        .and_then(|d| d.as_bytes().get(..32))
        .ok_or_else(|| invalid("missing post header"))?
        .to_vec();
    post[..4].copy_from_slice(&[0, 3, 0, 0]);
    builder.add_raw(Tag::new(b"post"), post);
    crate::otf_vertical::rebuild_vertical_metrics(
        &mut builder,
        &face,
        font,
        &font.design_default_master().map_err(invalid)?.id,
        order,
        &bounds,
    )?;
    builder.add_raw(Tag::new(b"CFF2"), cff);
    Ok(builder.build())
}

/// Authors the CFF2 Private DICT from the masters' PostScript private values.
///
/// CFF2 has no stem or hint-mask operators, but its Private DICT keeps the
/// blue zones, stem snaps and standard widths, and every numeric value may vary.
/// Values are taken from the design default master and varied with the same
/// interpolation model as the outlines, as differences of the absolute values;
/// `fc_sfnt::write::write_cff2` converts an array operator's deltas into the
/// relative basis its DICT operands are stored in. Masters that declare different array
/// lengths or a different set of optional values cannot be interpolated
/// elementwise; that is reported and the default master's values are written
/// without variation rather than guessed at.
fn private_values(
    font: &Font,
    axis_order: &[Tag],
    warnings: &mut Vec<ExportWarning>,
) -> Result<Cff2Private, Error> {
    let default = font.design_default_master().map_err(invalid)?;
    let values = default.private_dict.clone();
    let layout = private_layout(&values);
    let plain = Cff2Private {
        values: values.clone(),
        regions: Vec::new(),
        deltas: Vec::new(),
    };
    if layout.is_empty() || font.masters.len() < 2 || font.axes.is_empty() {
        return Ok(plain);
    }
    let mut sources = HashMap::new();
    for master in &font.masters {
        let other = private_layout(&master.private_dict);
        if other.len() != layout.len() {
            warnings.push(ExportWarning {
                code: "cff2-private-not-varied",
                message: format!("Master `{}` declares a different set of PostScript private values than the default master, so the CFF2 private dictionary was written from the default master without variation", master.name),
            });
            return Ok(plain);
        }
        let location = font.complete_location(&master.location).map_err(invalid)?;
        sources.insert(normalized_location(&font.axes, &location), other);
    }
    if sources.len() < 2 {
        return Ok(plain);
    }
    let model = VariationModel::new(sources.keys().cloned().collect(), axis_order.to_vec());
    let deltas = model
        .deltas_with_rounding::<f64, f64>(&sources, RoundingBehaviour::None)
        .map_err(invalid)?;
    let mut regions = Vec::new();
    let mut rows = Vec::new();
    for (region, row) in deltas {
        if region.is_default() || row.iter().all(|value| *value == 0.0) {
            continue;
        }
        regions.push(
            axis_order
                .iter()
                .map(|tag| {
                    region.get(tag).map_or([0.0; 3], |tent| {
                        [tent.min.to_f64(), tent.peak.to_f64(), tent.max.to_f64()]
                    })
                })
                .collect(),
        );
        rows.push(row);
    }
    Ok(Cff2Private {
        values,
        regions,
        deltas: rows,
    })
}

fn collect_locations(
    font: &Font,
    name: &GlyphName,
    visited: &mut HashSet<GlyphName>,
    locations: &mut HashMap<fontdrasil::coords::NormalizedLocation, DesignLocation>,
    depth: usize,
) -> Result<(), Error> {
    if depth >= 64 {
        return Err(invalid("CFF2 component depth limit"));
    }
    if visited.contains(name) {
        return Ok(());
    }
    if visited.len() > 65535 {
        return Err(invalid("CFF2 component source work limit"));
    }
    visited.insert(name.clone());
    let glyph = font
        .glyph(name)
        .ok_or_else(|| invalid(format!("missing glyph {name}")))?;
    for (location, layer) in glyph_sources(font, glyph).map_err(invalid)? {
        locations.insert(normalized_location(&font.axes, &location), location);
        if locations.len() > 1024 {
            return Err(invalid("CFF2 glyph exceeds 1024 source locations"));
        }
        for component in &layer.components {
            collect_locations(font, &component.base, visited, locations, depth + 1)?;
        }
    }
    Ok(())
}
