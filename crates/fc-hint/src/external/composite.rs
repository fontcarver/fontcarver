//! Validation of the single internal glyph added by composite hinting.

use read_fonts::{
    tables::{
        cmap::{CmapIterLimits, CmapSubtable, MapVariant},
        glyf::Glyph,
    },
    types::GlyphId,
    FontRef, TableProvider,
};

use crate::Error;

/// Checks the appended helper's outline, instructions, loca entry and invisibility.
pub(super) fn valid_helper(face: &FontRef<'_>, original_count: u16) -> Result<bool, Error> {
    let invalid = |error: read_fonts::ReadError| Error::External(error.to_string());
    let helper_id = GlyphId::new(u32::from(original_count));
    let glyf = face.glyf().map_err(invalid)?;
    let loca = face.loca(None).map_err(invalid)?;
    let Some(Glyph::Simple(helper)) = loca.get_glyf(helper_id, &glyf).map_err(invalid)? else {
        return Ok(false);
    };
    if helper.num_points() != 1
        || helper.end_pts_of_contours().len() != 1
        || helper.instructions().is_empty()
        || !helper
            .points()
            .next()
            .is_some_and(|point| point.x == 0 && point.y == 0 && point.on_curve)
    {
        return Ok(false);
    }
    let cmap = face.cmap().map_err(invalid)?;
    let limits = CmapIterLimits {
        max_char: u32::MAX,
        glyph_count: u32::from(original_count) + 1,
    };
    let mut remaining = 2_000_000usize;
    for record in cmap.encoding_records() {
        let subtable = record.subtable(cmap.offset_data()).map_err(invalid)?;
        let exposed = match &subtable {
            CmapSubtable::Format0(table) => table
                .glyph_id_array()
                .iter()
                .any(|gid| u16::from(*gid) == original_count),
            CmapSubtable::Format14(table) => {
                let mut exposed = false;
                for (_, _, mapping) in table.iter_with_limits(limits) {
                    remaining = remaining.checked_sub(1).ok_or_else(|| {
                        Error::External(
                            "composite helper cmap validation exceeded its budget".into(),
                        )
                    })?;
                    exposed |= mapping == MapVariant::Variant(helper_id);
                }
                exposed
            }
            // These legacy formats lack a mapping iterator. Refuse to trust an
            // extra glyph whose exclusion cannot be verified.
            CmapSubtable::Format2(_) | CmapSubtable::Format8(_) => return Ok(false),
            _ => {
                let mut exposed = false;
                for (_, gid) in subtable.iter_with_limits(limits) {
                    remaining = remaining.checked_sub(1).ok_or_else(|| {
                        Error::External(
                            "composite helper cmap validation exceeded its budget".into(),
                        )
                    })?;
                    exposed |= gid == helper_id;
                }
                exposed
            }
        };
        if exposed {
            return Ok(false);
        }
    }
    Ok(true)
}
