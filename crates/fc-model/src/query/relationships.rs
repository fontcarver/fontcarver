//! Reverse component and Unicode lookups.

use std::collections::HashMap;

use crate::{Font, Glyph, GlyphName};

/// Builds a reverse component map from each base to glyphs that reference it.
pub fn dependents(font: &Font) -> HashMap<GlyphName, Vec<GlyphName>> {
    let mut result: HashMap<GlyphName, Vec<GlyphName>> = HashMap::new();
    for (name, glyph) in &font.glyphs {
        for layer in &glyph.layers {
            for component in &layer.components {
                let dependents = result.entry(component.base.clone()).or_default();
                if !dependents.contains(name) {
                    dependents.push(name.clone());
                }
            }
            for component in layer
                .color_layers
                .iter()
                .flatten()
                .flat_map(|color_layer| &color_layer.components)
            {
                let dependents = result.entry(component.base.clone()).or_default();
                if !dependents.contains(name) {
                    dependents.push(name.clone());
                }
            }
        }
    }
    if let Some(color) = &font.color {
        for (name, layers) in &color.glyphs {
            for layer in layers {
                if layer.glyph == *name {
                    continue;
                }
                let dependents = result.entry(layer.glyph.clone()).or_default();
                if !dependents.contains(name) {
                    dependents.push(name.clone());
                }
            }
        }
    }
    result
}

/// Returns the first glyph in glyph order assigned to a Unicode scalar value.
pub fn glyph_for_unicode(font: &Font, cp: u32) -> Option<&Glyph> {
    font.glyphs
        .values()
        .find(|glyph| glyph.unicodes.contains(&cp))
}
