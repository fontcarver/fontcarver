use fw_model::{FontInfo, GaspRecord, Master, MasterMetrics};
use read_fonts::{FontRef, TableProvider};

use crate::Error;

pub(crate) fn apply_global(face: &FontRef<'_>, info: &mut FontInfo) -> Result<(), Error> {
    let hhea = face.hhea().map_err(|error| Error::parse("hhea", error))?;
    info.open_type_hhea_caret_slope_rise = Some(i32::from(hhea.caret_slope_rise()));
    info.open_type_hhea_caret_slope_run = Some(i32::from(hhea.caret_slope_run()));
    info.open_type_hhea_caret_offset = Some(i32::from(hhea.caret_offset()));
    if face
        .table_data(read_fonts::types::Tag::new(b"vhea"))
        .is_some()
    {
        let vhea = face.vhea().map_err(|error| Error::parse("vhea", error))?;
        info.open_type_vhea_caret_slope_rise = Some(i32::from(vhea.caret_slope_rise()));
        info.open_type_vhea_caret_slope_run = Some(i32::from(vhea.caret_slope_run()));
        info.open_type_vhea_caret_offset = Some(i32::from(vhea.caret_offset()));
    }
    if face
        .table_data(read_fonts::types::Tag::new(b"gasp"))
        .is_some()
    {
        let gasp = face.gasp().map_err(|error| Error::parse("gasp", error))?;
        info.open_type_gasp_range_records = gasp
            .gasp_ranges()
            .iter()
            .map(|range| GaspRecord {
                range_max_ppem: range.range_max_ppem(),
                behavior: set_bits(u32::from(range.range_gasp_behavior().bits())),
            })
            .collect();
    }
    Ok(())
}

pub(crate) fn master(face: &FontRef<'_>, info: &FontInfo) -> Result<Master, Error> {
    let hhea = face.hhea().map_err(|error| Error::parse("hhea", error))?;
    let os2 = super::os2::table(face)?;
    let post = if face
        .table_data(read_fonts::types::Tag::new(b"post"))
        .is_some()
    {
        Some(face.post().map_err(|error| Error::parse("post", error))?)
    } else {
        None
    };
    let mut metrics = MasterMetrics {
        ascender: os2.as_ref().map_or_else(
            || f64::from(hhea.ascender().to_i16()),
            |table| f64::from(table.s_typo_ascender()),
        ),
        descender: os2.as_ref().map_or_else(
            || f64::from(hhea.descender().to_i16()),
            |table| f64::from(table.s_typo_descender()),
        ),
        x_height: os2
            .as_ref()
            .and_then(|table| table.sx_height())
            .map(f64::from)
            .unwrap_or(0.0),
        cap_height: os2
            .as_ref()
            .and_then(|table| table.s_cap_height())
            .map(f64::from)
            .unwrap_or(0.0),
        italic_angle: post
            .as_ref()
            .map(|table| table.italic_angle().to_f64())
            .unwrap_or(0.0),
        hhea_ascender: Some(f64::from(hhea.ascender().to_i16())),
        hhea_descender: Some(f64::from(hhea.descender().to_i16())),
        hhea_line_gap: Some(f64::from(hhea.line_gap().to_i16())),
        ..MasterMetrics::default()
    };
    if let Some(os2) = os2 {
        metrics.typo_ascender = Some(f64::from(os2.s_typo_ascender()));
        metrics.typo_descender = Some(f64::from(os2.s_typo_descender()));
        metrics.typo_line_gap = Some(f64::from(os2.s_typo_line_gap()));
        metrics.win_ascent = Some(f64::from(os2.us_win_ascent()));
        metrics.win_descent = Some(f64::from(os2.us_win_descent()));
    }
    if face
        .table_data(read_fonts::types::Tag::new(b"vhea"))
        .is_some()
    {
        let vhea = face.vhea().map_err(|error| Error::parse("vhea", error))?;
        metrics.vhea_ascender = Some(f64::from(vhea.ascender().to_i16()));
        metrics.vhea_descender = Some(f64::from(vhea.descender().to_i16()));
        metrics.vhea_line_gap = Some(f64::from(vhea.line_gap().to_i16()));
        metrics.default_vertical_advance = Some(f64::from(vhea.advance_height_max().to_u16()));
    }
    if face
        .table_data(read_fonts::types::Tag::new(b"VORG"))
        .is_some()
    {
        let vorg = face.vorg().map_err(|error| Error::parse("VORG", error))?;
        metrics.default_vertical_origin = Some(f64::from(vorg.default_vert_origin_y()));
    }
    Ok(Master {
        name: info.style_name.clone(),
        metrics,
        ..Master::default()
    })
}

fn set_bits(bits: u32) -> Vec<u8> {
    (0..u32::BITS)
        .filter(|bit| bits & (1 << bit) != 0)
        .map(|bit| bit as u8)
        .collect()
}
