use fc_model::{ColorPalette, ColorTable, ColrLayer, PaletteColor};
use std::mem::size_of;

pub(super) fn estimated_bytes(color: &ColorTable) -> usize {
    size_of::<ColorTable>()
        + color
            .palettes
            .iter()
            .map(|p| size_of::<ColorPalette>() + p.colors.len() * size_of::<PaletteColor>())
            .sum::<usize>()
        + color.entry_labels.len() * size_of::<Option<u16>>()
        + color
            .glyphs
            .iter()
            .map(|(base, layers)| {
                base.len()
                    + 64
                    + layers
                        .iter()
                        .map(|l| size_of::<ColrLayer>() + l.glyph.len())
                        .sum::<usize>()
            })
            .sum::<usize>()
}
