//! Importers and exporters for source font formats.

#![warn(missing_docs)]

pub mod bitmap;
mod error;
pub mod glyphs;
pub mod metrics;
pub mod sfd;
pub mod type1;
pub mod type42;
pub mod ufo;
pub mod vector;

pub use error::Error;

/// Refuse a lossy source export before opening or replacing any destination.
fn check_layer_effects(font: &fc_model::Font) -> Result<(), Error> {
    check_effect_stacks(font)?;
    if font
        .glyphs
        .values()
        .flat_map(|glyph| &glyph.layers)
        .any(|layer| {
            !layer.constructions.is_empty()
                || !layer.node_links.is_empty()
                || !layer.linked_guides.is_empty()
                || layer
                    .contours
                    .iter()
                    .flat_map(|contour| &contour.points)
                    .any(|point| point.super_smooth)
        })
    {
        return Err(Error::Source {
            format: "source export",
            reason: "live constructions and node constraints require native FCF or UFO save; bake or remove them before exporting this source format".into(),
        });
    }
    Ok(())
}

fn check_effect_stacks(font: &fc_model::Font) -> Result<(), Error> {
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
