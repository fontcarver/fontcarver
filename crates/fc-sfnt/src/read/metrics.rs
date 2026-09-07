use fc_model::{FontInfo, GaspRecord, Master, MasterMetrics};
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
    let os2_heights = os2.as_ref().filter(|table| table.version() >= 2);
    let mut metrics = MasterMetrics {
        ascender: os2.as_ref().map_or_else(
            || f64::from(hhea.ascender().to_i16()),
            |table| f64::from(table.s_typo_ascender()),
        ),
        descender: os2.as_ref().map_or_else(
            || f64::from(hhea.descender().to_i16()),
            |table| f64::from(table.s_typo_descender()),
        ),
        x_height: os2_heights
            .and_then(|table| table.sx_height())
            .filter(|height| *height != 0)
            .map(f64::from)
            .unwrap_or(0.0),
        cap_height: os2_heights
            .and_then(|table| table.s_cap_height())
            .filter(|height| *height != 0)
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

pub(crate) fn apply_height_fallbacks(font: &mut fc_model::Font) {
    let Some(master) = font.masters.first() else {
        return;
    };
    let master_id = master.id.clone();
    let x_height = (master.metrics.x_height == 0.0)
        .then(|| glyph_top_for_unicode(font, 0x0078, &master_id))
        .flatten();
    let cap_height = (master.metrics.cap_height == 0.0)
        .then(|| glyph_top_for_unicode(font, 0x0048, &master_id))
        .flatten();
    if let Some(metrics) = font.masters.first_mut().map(|master| &mut master.metrics) {
        if let Some(x_height) = x_height {
            metrics.x_height = x_height;
        }
        if let Some(cap_height) = cap_height {
            metrics.cap_height = cap_height;
        }
    }
}

fn glyph_top_for_unicode(
    font: &fc_model::Font,
    codepoint: u32,
    master: &fc_model::MasterId,
) -> Option<f64> {
    let glyph = font
        .glyphs
        .values()
        .find(|glyph| glyph.unicodes.contains(&codepoint))?;
    fc_model::layer_bbox(font, &glyph.name, master)
        .ok()
        .flatten()
        .map(|bounds| bounds.y1)
}

fn set_bits(bits: u32) -> Vec<u8> {
    (0..u32::BITS)
        .filter(|bit| bits & (1 << bit) != 0)
        .map(|bit| bit as u8)
        .collect()
}

#[cfg(test)]
mod tests {
    use fc_model::{Contour, Font, Glyph, Layer, Point, PointKind};

    use super::apply_height_fallbacks;

    #[test]
    fn height_fallbacks_follow_unicode_when_glyphs_are_renamed() {
        let mut font = Font::new_empty("Renamed Metrics", 1_000);
        add_box(&mut font, "latin-small-letter-x.alt", 0x0078, 432.0);
        add_box(&mut font, "latin-capital-letter-h.alt", 0x0048, 711.0);

        apply_height_fallbacks(&mut font);

        assert_eq!(font.masters[0].metrics.x_height, 432.0);
        assert_eq!(font.masters[0].metrics.cap_height, 711.0);
    }

    fn add_box(font: &mut Font, name: &str, unicode: u32, top: f64) {
        let master = font.default_master().id.clone();
        let mut glyph = Glyph::new(name);
        glyph.unicodes.push(unicode);
        let mut layer = Layer::new_master(master);
        layer.contours.push(Contour {
            closed: true,
            points: vec![
                point(0.0, 0.0),
                point(100.0, 0.0),
                point(100.0, top),
                point(0.0, top),
            ],
            ..Contour::default()
        });
        glyph.layers.push(layer);
        font.insert_glyph(glyph).expect("test glyph is valid");
    }

    fn point(x: f64, y: f64) -> Point {
        Point {
            x,
            y,
            kind: PointKind::Line,
            ..Point::default()
        }
    }
}
