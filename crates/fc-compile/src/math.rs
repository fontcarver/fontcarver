//! Editable MATH output for the compiled glyph order.

use std::collections::HashSet;

use fc_model::{Font, GlyphName};
use read_fonts::{types::Tag, FontRef};
use write_fonts::FontBuilder;

use crate::{Error, ExportWarning};

const MATH: Tag = Tag::new(b"MATH");
const MAX_LISTED_NAMES: usize = 8;

/// Whether editable MATH, including an explicit deletion, owns the output table.
pub(crate) fn owned(font: &Font) -> bool {
    font.math_table_owned || font.math.is_some()
}

/// Replace any MATH table in `sfnt` with the font's editable data.
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
    let face = FontRef::new(sfnt).map_err(|error| Error::parse("sfnt before MATH", error))?;
    let mut builder = FontBuilder::new();
    for record in face.table_directory().table_records() {
        if record.tag() == MATH {
            continue;
        }
        let data = face
            .table_data(record.tag())
            .ok_or_else(|| Error::parse("sfnt before MATH", record.tag()))?;
        builder.add_raw(record.tag(), data.as_bytes());
    }
    if let Some(source) = &font.math {
        let exported: HashSet<&GlyphName> = order.iter().collect();
        let mut math = source.clone();
        let mut missing = std::collections::BTreeSet::new();
        let dropped = math.retain_glyphs(|name| {
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
                code: "math-references-dropped",
                message: format!(
                    "MATH: omitted {dropped} records, variants or assemblies that use glyphs not exported: {}{suffix}",
                    names.join(", ")
                ),
            });
        }
        let bytes =
            fc_sfnt::math::write_table(&math, order).map_err(|e| Error::write("MATH", e))?;
        builder.add_raw(MATH, bytes);
    }
    Ok(builder.build())
}
