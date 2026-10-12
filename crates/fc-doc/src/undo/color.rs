use fc_model::{ColorPalette, ColorTable, ColrColorStop, ColrLayer, ColrPaint, PaletteColor};
use std::mem::size_of;

pub(super) fn estimated_bytes(color: &ColorTable) -> usize {
    size_of::<ColorTable>()
        + color.graph.as_ref().map_or(0, |graph| {
            graph.glyphs.keys().map(|n| n.len() + 80).sum::<usize>()
                + graph.clips.keys().map(|n| n.len() + 96).sum::<usize>()
                + graph
                    .paints
                    .iter()
                    .map(|paint| {
                        size_of::<ColrPaint>()
                            + match paint {
                                ColrPaint::Layers { paints } => paints.len() * 4,
                                ColrPaint::Linear { color_line, .. }
                                | ColrPaint::Radial { color_line, .. }
                                | ColrPaint::Sweep { color_line, .. } => {
                                    color_line.stops.len() * size_of::<ColrColorStop>()
                                }
                                ColrPaint::Glyph { glyph, .. } | ColrPaint::ColrGlyph { glyph } => {
                                    glyph.len()
                                }
                                _ => 0,
                            }
                    })
                    .sum::<usize>()
        })
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
