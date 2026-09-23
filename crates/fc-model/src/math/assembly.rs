//! Bounded glyph assembly layout for previews.
//!
//! Implemented from the OpenType MATH GlyphAssembly description: extender
//! parts may be omitted or repeated, and adjacent connectors overlap by at
//! least the font's minimum connector overlap where the connectors allow it.

use super::MathAssembly;
use crate::GlyphName;
use serde::{Deserialize, Serialize};

/// One positioned part of an assembled shape.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PlacedMathPart {
    /// Referenced glyph name.
    pub glyph: GlyphName,
    /// Start position along the extension direction.
    pub offset: f64,
    /// Full part advance along the extension direction.
    pub advance: f64,
}

/// An assembly arranged for a requested size.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MathAssemblyLayout {
    /// Parts from left to right or bottom to top.
    pub parts: Vec<PlacedMathPart>,
    /// Resulting extent; smaller than the request only when `limited` is set.
    pub size: f64,
    /// How many times each extender part is used.
    pub repeats: u32,
    /// Some adjacent connectors are shorter than the minimum overlap.
    pub short_connectors: bool,
    /// The part limit stopped growth before the requested size was reached.
    pub limited: bool,
}

impl MathAssembly {
    /// Arrange parts to reach at least `target`, using at most `max_parts`.
    ///
    /// All extenders use the same repeat count, starting from zero. Overlaps
    /// are interpolated between each connection's minimum and maximum so the
    /// result matches the target when possible. Returns `None` for an empty
    /// assembly or when even the fewest possible parts exceed `max_parts`.
    #[must_use]
    pub fn layout(
        &self,
        target: f64,
        min_overlap: f64,
        max_parts: usize,
    ) -> Option<MathAssemblyLayout> {
        let extenders = self.parts.iter().filter(|part| part.extender).count();
        let fixed = self.parts.len() - extenders;
        let mut repeats = usize::from(fixed == 0);
        let mut best = None;
        loop {
            let count = fixed + extenders * repeats;
            if count == 0 || count > max_parts {
                return best.map(|layout: MathAssemblyLayout| MathAssemblyLayout {
                    limited: layout.size < target,
                    ..layout
                });
            }
            let layout = self.arrange(repeats, target, min_overlap.max(0.0));
            let reached = layout.size >= target;
            best = Some(layout);
            if reached || extenders == 0 {
                return best;
            }
            repeats += 1;
        }
    }

    fn arrange(&self, repeats: usize, target: f64, min_overlap: f64) -> MathAssemblyLayout {
        let sequence: Vec<_> = self
            .parts
            .iter()
            .flat_map(|part| {
                let count = if part.extender { repeats } else { 1 };
                std::iter::repeat_n(part, count)
            })
            .collect();
        let mut short_connectors = false;
        let connections: Vec<(f64, f64)> = sequence
            .windows(2)
            .map(|pair| {
                let most = pair[0].end_connector.min(pair[1].start_connector).max(0.0);
                short_connectors |= most < min_overlap;
                (min_overlap.min(most), most)
            })
            .collect();
        let advances: f64 = sequence.iter().map(|part| part.full_advance).sum();
        let largest = advances - connections.iter().map(|(least, _)| least).sum::<f64>();
        let slack: f64 = connections.iter().map(|(least, most)| most - least).sum();
        let shrink = if slack > 0.0 {
            ((largest - target) / slack).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let mut offset = 0.0;
        let mut parts = Vec::with_capacity(sequence.len());
        for (index, part) in sequence.iter().enumerate() {
            parts.push(PlacedMathPart {
                glyph: part.glyph.clone(),
                offset,
                advance: part.full_advance,
            });
            if let Some((least, most)) = connections.get(index) {
                offset += part.full_advance - (least + shrink * (most - least));
            }
        }
        let size = parts.last().map_or(0.0, |last| last.offset + last.advance);
        MathAssemblyLayout {
            parts,
            size,
            repeats: u32::try_from(repeats).unwrap_or(u32::MAX),
            short_connectors,
            limited: false,
        }
    }
}
