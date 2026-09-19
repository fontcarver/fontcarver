//! Replace compiler-generated TrueType outlines with native model CFF curves.

use fc_model::{Font, GlyphName, MasterId};
use kurbo::Rect;
use read_fonts::{types::Tag, FontRef, TableProvider};
use write_fonts::{from_obj::ToOwnedTable, FontBuilder};

use crate::Error;

pub(crate) fn prepare(
    font: &Font,
    master: &MasterId,
    production_names: bool,
) -> Result<(Vec<GlyphName>, fc_sfnt::write::CffOutput), Error> {
    let mut order = vec![GlyphName::from(".notdef")];
    order.extend(
        font.glyphs
            .values()
            .filter(|glyph| glyph.export && glyph.name.as_ref() != ".notdef")
            .map(|glyph| glyph.name.clone()),
    );
    let cff = fc_sfnt::write::write_cff(font, master, &order, production_names)
        .map_err(|error| Error::InvalidSource(error.to_string()))?;
    Ok((order, cff))
}

pub(crate) fn convert(
    sfnt: &[u8],
    font: &Font,
    master: &MasterId,
    order: &[GlyphName],
    production_names: bool,
    prepared: (Vec<GlyphName>, fc_sfnt::write::CffOutput),
) -> Result<Vec<u8>, Error> {
    crate::otf_preservation::check_geometry_independent_layout(sfnt)?;
    crate::otf_preservation_aat::check(sfnt)?;
    let cff = if prepared.0 == order {
        prepared.1
    } else {
        fc_sfnt::write::write_cff(font, master, order, production_names)
            .map_err(|error| Error::InvalidSource(error.to_string()))?
    };
    let face = FontRef::new(sfnt).map_err(|error| Error::parse("OTF source sfnt", error))?;
    let mut builder = FontBuilder::new();
    // These tables describe replaced outlines, hinting, variation, or point indices.
    const OMIT: [[u8; 4]; 27] = [
        *b"glyf", *b"loca", *b"CFF ", *b"CFF2", *b"maxp", *b"head", *b"hhea", *b"hmtx", *b"post",
        *b"fpgm", *b"prep", *b"cvt ", *b"gasp", *b"hdmx", *b"LTSH", *b"VDMX", *b"gvar", *b"cvar",
        *b"fvar", *b"avar", *b"HVAR", *b"VVAR", *b"MVAR", *b"DSIG", *b"vhea", *b"vmtx", *b"VORG",
    ];
    for record in face.table_directory().table_records() {
        if !OMIT.contains(&record.tag().to_be_bytes()) {
            let data = face
                .table_data(record.tag())
                .ok_or_else(|| Error::parse("OTF source table", record.tag()))?;
            builder.add_raw(record.tag(), data.as_bytes());
        }
    }
    let bbox = cff
        .bounds
        .iter()
        .flatten()
        .copied()
        .reduce(|a, b| a.union(b))
        .unwrap_or(Rect::ZERO);
    let mut head: write_fonts::tables::head::Head = face
        .head()
        .map_err(|error| Error::parse("head", error))?
        .to_owned_table();
    head.x_min = signed(bbox.x0.floor(), "xMin")?;
    head.y_min = signed(bbox.y0.floor(), "yMin")?;
    head.x_max = signed(bbox.x1.ceil(), "xMax")?;
    head.y_max = signed(bbox.y1.ceil(), "yMax")?;
    head.index_to_loc_format = 0;
    builder
        .add_table(&head)
        .map_err(|error| Error::write("head", error))?;
    let mut hhea: write_fonts::tables::hhea::Hhea = face
        .hhea()
        .map_err(|error| Error::parse("hhea", error))?
        .to_owned_table();
    hhea.number_of_h_metrics =
        u16::try_from(order.len()).map_err(|_| Error::Limit("glyph count"))?;
    hhea.advance_width_max = cff
        .advances
        .iter()
        .copied()
        .max()
        .unwrap_or_default()
        .into();
    let mut hmtx = Vec::with_capacity(order.len() * 4);
    let mut min_left = None::<i16>;
    let mut min_right = None::<f64>;
    let mut max_extent = None::<i16>;
    for (advance, bounds) in cff.advances.iter().zip(&cff.bounds) {
        let left = bounds.map_or(0.0, |bounds| bounds.x0.floor());
        let left = signed(left, "left sidebearing")?;
        hmtx.extend_from_slice(&advance.to_be_bytes());
        hmtx.extend_from_slice(&left.to_be_bytes());
        if let Some(bounds) = bounds {
            let right = f64::from(*advance) - bounds.x1.ceil();
            let extent = signed(bounds.x1.ceil(), "horizontal extent")?;
            min_left = Some(min_left.map_or(left, |old| old.min(left)));
            min_right = Some(min_right.map_or(right, |old| old.min(right)));
            max_extent = Some(max_extent.map_or(extent, |old| old.max(extent)));
        }
    }
    hhea.min_left_side_bearing = min_left.unwrap_or_default().into();
    hhea.min_right_side_bearing =
        signed(min_right.unwrap_or_default(), "minimum right sidebearing")?.into();
    hhea.x_max_extent = max_extent.unwrap_or_default().into();
    builder
        .add_table(&hhea)
        .map_err(|error| Error::write("hhea", error))?;
    builder.add_raw(Tag::new(b"hmtx"), hmtx);
    let mut maxp = vec![0, 0, 0x50, 0];
    maxp.extend_from_slice(&hhea.number_of_h_metrics.to_be_bytes());
    builder.add_raw(Tag::new(b"maxp"), maxp);
    let source_post = face
        .table_data(Tag::new(b"post"))
        .ok_or_else(|| Error::parse("post", "missing table"))?;
    let mut post = source_post
        .as_bytes()
        .get(..32)
        .ok_or_else(|| Error::parse("post", "truncated header"))?
        .to_vec();
    post[..4].copy_from_slice(&[0, 3, 0, 0]);
    builder.add_raw(Tag::new(b"post"), post);
    crate::otf_vertical::rebuild_vertical_metrics(
        &mut builder,
        &face,
        font,
        master,
        order,
        &cff.bounds,
    )?;
    builder.add_raw(Tag::new(b"CFF "), cff.bytes);
    Ok(builder.build())
}

fn signed(value: f64, field: &'static str) -> Result<i16, Error> {
    if !value.is_finite() || !(-32768.0..=32767.0).contains(&value) {
        return Err(Error::InvalidSource(format!(
            "{field} {value} exceeds OpenType signed 16-bit limits"
        )));
    }
    Ok(value as i16)
}
