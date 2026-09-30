//! Database-driven metadata changes preserve all source geometry.
use fc_glyphdata::{parse_overlay, resolve_info, GlyphDataOverlay};
use fc_model::{Font, FontInfo, GlyphName};
use std::collections::BTreeSet;

use crate::Error;

/// Font-info user-data key for the document's versioned glyph database overlay.
pub const GLYPH_DATA_OVERLAY_KEY: &str = "com.fontcarver.glyphData";

/// Read and validate the persisted document overlay without changing the model.
pub fn glyph_data_overlay(info: &FontInfo) -> Result<GlyphDataOverlay, Error> {
    match info.user_data.get(GLYPH_DATA_OVERLAY_KEY) {
        Some(value) => parse_overlay(&value.to_string()).map_err(Error::InvalidCommand),
        None => parse_overlay(r#"{"version":1,"glyphs":{}}"#).map_err(Error::InvalidCommand),
    }
}

pub(super) fn update(font: &mut Font, glyphs: &[GlyphName]) -> Result<(), Error> {
    if glyphs.is_empty() || glyphs.len() > 65_536 {
        return Err(Error::InvalidCommand(
            "Update Glyph Info requires between one and 65536 glyphs".into(),
        ));
    }
    let overlay = glyph_data_overlay(&font.info)?;
    let mut changes = Vec::new();
    let mut seen = BTreeSet::new();
    for name in glyphs {
        if !seen.insert(name) {
            continue;
        }
        let glyph = font
            .glyph(name)
            .ok_or_else(|| Error::MissingGlyph(name.clone()))?;
        if glyph.locked {
            return Err(Error::InvalidCommand(format!("{name} is locked")));
        }
        changes.push((name, resolve_info(name.as_ref(), &glyph.unicodes, &overlay)));
    }
    for (name, info) in changes {
        let glyph = font
            .glyph_mut(name)
            .ok_or_else(|| Error::MissingGlyph(name.clone()))?;
        glyph.unicodes = info.unicodes;
        glyph.category = info.category;
        glyph.sub_category = info.sub_category;
        glyph.script = info.script;
        glyph.production_name = info.production_name;
    }
    Ok(())
}
