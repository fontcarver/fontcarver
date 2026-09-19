//! Rebuild vertical metrics against the final cubic outline bounds.

use std::collections::BTreeMap;

use fc_model::{Font, GlyphName, MasterId};
use kurbo::Rect;
use read_fonts::{
    types::{GlyphId, Tag},
    FontRef, TableProvider,
};
use write_fonts::{from_obj::ToOwnedTable, FontBuilder};

use crate::Error;

pub(crate) fn rebuild_vertical_metrics(
    builder: &mut FontBuilder<'_>,
    face: &FontRef<'_>,
    font: &Font,
    master: &MasterId,
    order: &[GlyphName],
    bounds: &[Option<Rect>],
) -> Result<(), Error> {
    let has_vhea = face.table_data(Tag::new(b"vhea")).is_some();
    let has_vmtx = face.table_data(Tag::new(b"vmtx")).is_some();
    if !has_vhea && !has_vmtx {
        return Ok(());
    }
    if has_vhea != has_vmtx {
        return Err(Error::InvalidSource(
            "OTF vertical metrics require both vhea and vmtx tables".to_owned(),
        ));
    }
    if bounds.len() != order.len() {
        return Err(Error::InvalidSource(
            "OTF vertical metrics do not match the compiled glyph order".to_owned(),
        ));
    }
    let master = font
        .master(master)
        .ok_or_else(|| Error::MissingMaster(master.clone()))?;
    let vmtx = face.vmtx().map_err(|error| Error::parse("vmtx", error))?;
    let mut vhea: write_fonts::tables::vhea::Vhea = face
        .vhea()
        .map_err(|error| Error::parse("vhea", error))?
        .to_owned_table();
    let loca = face
        .loca(None)
        .map_err(|error| Error::parse("loca", error))?;
    let glyf = face.glyf().map_err(|error| Error::parse("glyf", error))?;
    let default_origin = master
        .metrics
        .default_vertical_origin
        .map(round_origin)
        .transpose()?;
    let mut metrics = Vec::with_capacity(order.len() * 4);
    let mut origins = Vec::with_capacity(order.len());
    let mut origin_counts = BTreeMap::<i16, usize>::new();
    let mut max_advance = 0;
    let mut min_top = None::<i16>;
    let mut min_bottom = None::<f64>;
    let mut max_extent = None::<f64>;
    for (index, (name, bounds)) in order.iter().zip(bounds).enumerate() {
        let gid = GlyphId::new(index as u32);
        let advance = vmtx
            .advance(gid)
            .ok_or_else(|| Error::parse("vmtx", format!("missing advance for glyph {name}")))?;
        let modeled_origin = font
            .glyph(name.as_ref())
            .and_then(|glyph| glyph.master_layer(&master.id))
            .and_then(|layer| layer.vertical_origin)
            .map(round_origin)
            .transpose()?
            .or(default_origin);
        let origin = if let Some(origin) = modeled_origin {
            origin
        } else {
            // fontc computes a fallback origin even for a layer that does not
            // explicitly model one. Recover that origin before replacing glyf.
            let y_max = loca
                .get_glyf(gid, &glyf)
                .map_err(|error| Error::parse("glyf vertical bounds", error))?
                .map_or(0, |glyph| i32::from(glyph.y_max()));
            let top = vmtx.side_bearing(gid).ok_or_else(|| {
                Error::parse("vmtx", format!("missing top sidebearing for glyph {name}"))
            })?;
            signed(f64::from(y_max + i32::from(top)), "vertical origin")?
        };
        let y_max = bounds.map_or(0.0, |bounds| bounds.y1.ceil());
        let top = signed(f64::from(origin) - y_max, "top sidebearing")?;
        metrics.extend_from_slice(&advance.to_be_bytes());
        metrics.extend_from_slice(&top.to_be_bytes());
        origins.push(origin);
        *origin_counts.entry(origin).or_default() += 1;
        max_advance = max_advance.max(advance);
        // Empty glyphs contribute advances and origins, but OpenType excludes
        // them from sidebearing and extent extrema.
        if let Some(bounds) = bounds {
            let extent = f64::from(origin) - bounds.y0.floor();
            let bottom = f64::from(advance) - extent;
            min_top = Some(min_top.map_or(top, |old| old.min(top)));
            min_bottom = Some(min_bottom.map_or(bottom, |old| old.min(bottom)));
            max_extent = Some(max_extent.map_or(extent, |old| old.max(extent)));
        }
    }
    vhea.number_of_long_ver_metrics =
        u16::try_from(order.len()).map_err(|_| Error::Limit("vertical metric count"))?;
    vhea.advance_height_max = max_advance.into();
    vhea.min_top_side_bearing = min_top.unwrap_or_default().into();
    vhea.min_bottom_side_bearing =
        signed(min_bottom.unwrap_or_default(), "minimum bottom sidebearing")?.into();
    vhea.y_max_extent = signed(max_extent.unwrap_or_default(), "maximum vertical extent")?.into();
    builder
        .add_table(&vhea)
        .map_err(|error| Error::write("vhea", error))?;
    builder.add_raw(Tag::new(b"vmtx"), metrics);
    let default_origin = default_origin.unwrap_or_else(|| {
        origin_counts
            .into_iter()
            .max_by_key(|&(origin, count)| (count, std::cmp::Reverse(origin)))
            .map_or(0, |(origin, _)| origin)
    });
    let records: Vec<_> = origins
        .into_iter()
        .enumerate()
        .filter(|(_, origin)| *origin != default_origin)
        .collect();
    // write-fonts does not expose a VORG writer. Its fixed-width format is a
    // version 1.0 header followed by sorted (glyph ID, signed origin) records.
    let mut vorg = Vec::with_capacity(8 + records.len() * 4);
    vorg.extend_from_slice(&[0, 1, 0, 0]);
    vorg.extend_from_slice(&default_origin.to_be_bytes());
    let count = u16::try_from(records.len()).map_err(|_| Error::Limit("VORG record count"))?;
    vorg.extend_from_slice(&count.to_be_bytes());
    for (index, origin) in records {
        let gid = u16::try_from(index).map_err(|_| Error::Limit("VORG glyph ID"))?;
        vorg.extend_from_slice(&gid.to_be_bytes());
        vorg.extend_from_slice(&origin.to_be_bytes());
    }
    builder.add_raw(Tag::new(b"VORG"), vorg);
    Ok(())
}

fn round_origin(value: f64) -> Result<i16, Error> {
    // Match OpenType rounding used by fontc, including negative half values.
    signed((value + 0.5).floor(), "vertical origin")
}

fn signed(value: f64, field: &'static str) -> Result<i16, Error> {
    if !value.is_finite() || !(-32768.0..=32767.0).contains(&value) {
        return Err(Error::InvalidSource(format!(
            "{field} {value} exceeds OpenType signed 16-bit limits"
        )));
    }
    Ok(value as i16)
}
