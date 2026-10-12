//! Editable COLR/CPAL output for the compiled glyph order.

use std::collections::HashSet;

use fc_model::{Font, GlyphName};
use read_fonts::{types::Tag, FontRef};
use write_fonts::FontBuilder;

use crate::{Error, ExportWarning};

const COLR: Tag = Tag::new(b"COLR");
const CPAL: Tag = Tag::new(b"CPAL");
const MAX_LISTED_NAMES: usize = 8;

/// Whether editable COLR/CPAL, including an explicit deletion, owns the output table.
pub(crate) fn owned(font: &Font) -> bool {
    font.color_tables_owned || font.color.is_some()
}

/// Replace any COLR/CPAL table in `sfnt` with the font's editable data.
///
/// `order` is the actual compiled glyph order as source glyph names. Records
/// and references to glyphs outside that order are omitted from this output
/// only, with a warning; the source document is not modified. Invalid data
/// fails the export rather than producing a partial table.
pub(crate) fn apply(
    sfnt: &[u8],
    font: &Font,
    order: &[GlyphName],
    warnings: &mut Vec<ExportWarning>,
) -> Result<Vec<u8>, Error> {
    if !owned(font) {
        return Ok(sfnt.to_vec());
    }
    let face = FontRef::new(sfnt).map_err(|error| Error::parse("sfnt before COLR/CPAL", error))?;
    // An explicit deletion of a table that is already absent changes nothing.
    if font.color.is_none() && face.table_data(COLR).is_none() && face.table_data(CPAL).is_none() {
        return Ok(sfnt.to_vec());
    }
    let mut builder = FontBuilder::new();
    for record in face.table_directory().table_records() {
        if [COLR, CPAL].contains(&record.tag()) {
            continue;
        }
        let data = face
            .table_data(record.tag())
            .ok_or_else(|| Error::parse("sfnt before COLR/CPAL", record.tag()))?;
        builder.add_raw(record.tag(), data.as_bytes());
    }
    if let Some(source) = &font.color {
        source
            .validate(&font.glyph_order().cloned().collect::<Vec<_>>())
            .map_err(|e| Error::write("COLR/CPAL", e))?;
        let exported: HashSet<&GlyphName> = order.iter().collect();
        let mut color = source.clone();
        let mut missing = std::collections::BTreeSet::new();
        let dropped = color.retain_glyphs(|name| {
            let keep = exported.contains(name);
            if !keep {
                missing.insert(name.clone());
            }
            keep
        });
        if dropped != 0 {
            let mut names: Vec<_> = missing.iter().map(AsRef::<str>::as_ref).collect();
            let more = names.len().saturating_sub(MAX_LISTED_NAMES);
            names.truncate(MAX_LISTED_NAMES);
            let suffix = if more == 0 {
                String::new()
            } else {
                format!(" and {more} more")
            };
            warnings.push(ExportWarning {
                code: "color-references-dropped",
                message: format!(
                    "COLR/CPAL: omitted {dropped} base records or layers that use glyphs not exported: {}{suffix}",
                    names.join(", ")
                ),
            });
        }
        let (colr, cpal) = fc_sfnt::color::write_tables(&color, order)
            .map_err(|e| Error::write("COLR/CPAL", e))?;
        if let Some(colr) = colr {
            builder.add_raw(COLR, colr);
        }
        builder.add_raw(CPAL, cpal);
    }
    Ok(builder.build())
}
