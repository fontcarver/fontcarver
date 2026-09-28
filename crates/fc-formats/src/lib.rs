//! Importers and exporters for source font formats.

#![warn(missing_docs)]

pub mod bitmap;
mod error;
pub mod glyphs;
pub mod sfd;
pub mod type1;
pub mod ufo;

pub use error::Error;

/// Refuse a lossy source export before opening or replacing any destination.
fn check_layer_effects(font: &fc_model::Font) -> Result<(), Error> {
    if font
        .glyphs
        .values()
        .flat_map(|g| &g.layers)
        .any(|l| !l.effects.is_empty() || l.has_smart_corners())
    {
        return Err(Error::Source { format: "source export", reason: "live corners and layer effects require native FCF save; bake or remove them before exporting this source format".into() });
    }
    Ok(())
}
