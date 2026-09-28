//! SVG output ownership and glyph-order adaptation.
use crate::{Error, ExportWarning};
use fc_model::{Font, GlyphName};
use read_fonts::{types::Tag, FontRef};
use std::collections::HashSet;
use write_fonts::FontBuilder;
const SVG: Tag = Tag::new(b"SVG ");

pub(crate) fn owned(font: &Font) -> bool {
    font.svg_table_owned || font.svg.is_some()
}

pub(crate) fn apply(
    sfnt: &[u8],
    font: &Font,
    order: &[GlyphName],
    warnings: &mut Vec<ExportWarning>,
) -> Result<Vec<u8>, Error> {
    if !owned(font) {
        return Ok(sfnt.to_vec());
    }
    let face = FontRef::new(sfnt).map_err(|e| Error::parse("sfnt before SVG", e))?;
    if font.svg.is_none() && face.table_data(SVG).is_none() {
        return Ok(sfnt.to_vec());
    }
    let mut builder = FontBuilder::new();
    for record in face.table_directory().table_records() {
        if record.tag() == SVG {
            continue;
        }
        let data = face
            .table_data(record.tag())
            .ok_or_else(|| Error::parse("sfnt before SVG", record.tag()))?;
        builder.add_raw(record.tag(), data.as_bytes());
    }
    if let Some(source) = &font.svg {
        source
            .validate(&font.glyph_order().cloned().collect::<Vec<_>>())
            .map_err(|e| Error::write("SVG", e))?;
        let names: HashSet<_> = order.iter().collect();
        let mut svg = source.clone();
        let removed = svg.retain_glyphs(|name| names.contains(name));
        if removed > 0 {
            warnings.push(ExportWarning {
                code: "svg-references-dropped",
                message: format!("SVG: omitted {removed} bindings for glyphs not exported"),
            });
        }
        if !svg.documents.is_empty() {
            builder.add_raw(
                SVG,
                fc_sfnt::svg::write_table(&svg, order).map_err(|e| Error::write("SVG", e))?,
            );
        }
    }
    Ok(builder.build())
}
