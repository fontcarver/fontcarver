//! HarfRust-based shaping validation helpers.

use harfrust::{script, ShapeOptions, ShaperData, UnicodeBuffer};
use read_fonts::FontRef;

use crate::Error;

/// Shapes Latin text with default features and returns glyph advances.
pub fn shaping_fingerprint(font_bytes: &[u8], text: &str) -> Result<Vec<(u32, i32, i32)>, Error> {
    let font = FontRef::from_index(font_bytes, 0)
        .map_err(|error| Error::parse("font for shaping", error))?;
    let data = ShaperData::new(&font);
    let shaper = data.shaper(&font).build();
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.set_script(script::LATIN);
    buffer.guess_segment_properties();
    let shaped = shaper.shape(buffer, ShapeOptions::new());
    Ok(shaped
        .glyph_infos()
        .iter()
        .zip(shaped.glyph_positions())
        .map(|(info, position)| (info.glyph_id, position.x_advance, position.y_advance))
        .collect())
}
