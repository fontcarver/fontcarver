//! Editable COLR v0/v1 compositions and CPAL palettes.

use crate::GlyphName;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

/// Maximum expanded palette entries and layers, limiting shared-table amplification.
pub const MAX_COLOR_ITEMS: usize = 1_000_000;

/// An unpremultiplied sRGB color with an eight-bit alpha channel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PaletteColor {
    /// Red channel.
    pub red: u8,
    /// Green channel.
    pub green: u8,
    /// Blue channel.
    pub blue: u8,
    /// Opacity: zero is transparent, 255 is opaque.
    pub alpha: u8,
}

/// One palette; all palettes in a font have the same number of entries.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ColorPalette {
    /// Entries addressed by every glyph's layer palette indices.
    pub colors: Vec<PaletteColor>,
    /// CPAL v1 suitability flags (light background = 1, dark background = 2).
    #[serde(default)]
    pub flags: u32,
    /// Optional CPAL v1 label in the font's name table.
    #[serde(default)]
    pub label: Option<u16>,
}

/// A monochrome outline reference painted as one COLR v0 layer.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ColrLayer {
    /// Outline glyph name; its own COLR composition is not followed.
    pub glyph: GlyphName,
    /// Palette entry, or null for the text foreground color (0xFFFF).
    #[serde(default)]
    pub palette_index: Option<u16>,
}

/// Font-wide palettes, COLR v0 fallbacks and optional static COLRv1 paint graphs.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ColorTable {
    /// Static v1 paint graph, coexisting with optional v0 fallback layers.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(optional = nullable))]
    pub graph: Option<crate::ColrGraph>,
    /// CPAL version, zero or one. Version one retains palette metadata.
    #[serde(default)]
    pub palette_version: u16,
    /// At least one equally sized, nonempty palette.
    pub palettes: Vec<ColorPalette>,
    /// CPAL v1 entry name IDs, empty when the optional array is absent.
    #[serde(default)]
    pub entry_labels: Vec<Option<u16>>,
    /// Nonempty layer stacks, in paint order from bottom to top.
    #[serde(default)]
    pub glyphs: BTreeMap<GlyphName, Vec<ColrLayer>>,
}

impl ColorTable {
    /// Validate structure, references and binary field limits before editing/writing.
    pub fn validate(&self, glyph_order: &[GlyphName]) -> Result<(), String> {
        let entries = self.palettes.first().map_or(0, |p| p.colors.len());
        if self.palette_version > 1 {
            return Err("CPAL version must be 0 or 1".into());
        }
        if self.palettes.is_empty()
            || self.palettes.len() > 65535
            || entries == 0
            || entries > 65535
        {
            return Err("CPAL requires 1..65535 palettes and entries".into());
        }
        if self.palettes.len().saturating_mul(entries) > MAX_COLOR_ITEMS {
            return Err("CPAL expanded palette limit exceeded".into());
        }
        let valid_label = |label: &Option<u16>| label.is_none_or(|id| (256..=32767).contains(&id));
        for palette in &self.palettes {
            if palette.colors.len() != entries
                || palette.flags & !3 != 0
                || !valid_label(&palette.label)
            {
                return Err(
                    "CPAL palettes need equal lengths, valid flags and label IDs 256..32767".into(),
                );
            }
            if self.palette_version == 0 && (palette.flags != 0 || palette.label.is_some()) {
                return Err("CPAL palette metadata requires version 1".into());
            }
        }
        if !self.entry_labels.is_empty()
            && (self.palette_version != 1
                || self.entry_labels.len() != entries
                || !self.entry_labels.iter().all(valid_label))
        {
            return Err("CPAL entry labels require version 1 and one valid label per entry".into());
        }
        let names: HashSet<_> = glyph_order.iter().collect();
        if names.len() != glyph_order.len() || glyph_order.len() > 65535 {
            return Err("COLR requires unique glyph names and at most 65535 glyphs".into());
        }
        let mut count = 0usize;
        for (base, layers) in &self.glyphs {
            count = count.saturating_add(layers.len());
            if !names.contains(base)
                || layers.is_empty()
                || layers.len() > 65535
                || count > MAX_COLOR_ITEMS
            {
                return Err(format!("COLR invalid base glyph or layer count: {base}"));
            }
            for layer in layers {
                if !names.contains(&layer.glyph)
                    || layer
                        .palette_index
                        .is_some_and(|i| usize::from(i) >= entries)
                {
                    return Err(format!(
                        "COLR unknown outline or palette entry in {base}: {}",
                        layer.glyph
                    ));
                }
            }
        }
        if let Some(graph) = &self.graph {
            graph.validate(glyph_order, entries)?;
        }
        Ok(())
    }

    /// Rename a base glyph and every monochrome outline reference.
    pub fn rename_glyph(&mut self, from: &str, to: &GlyphName) {
        if let Some(graph) = &mut self.graph {
            graph.rename_glyph(from, to);
        }
        if let Some(layers) = self.glyphs.remove(from) {
            self.glyphs.insert(to.clone(), layers);
        }
        for layer in self.glyphs.values_mut().flatten() {
            if layer.glyph.as_ref() == from {
                layer.glyph = to.clone();
            }
        }
    }

    /// Remove excluded bases and layers; empty stacks fall back to monochrome.
    pub fn retain_glyphs(&mut self, mut keep: impl FnMut(&GlyphName) -> bool) -> usize {
        let mut removed = 0;
        self.glyphs.retain(|base, layers| {
            if !keep(base) {
                removed += 1;
                return false;
            }
            layers.retain(|layer| {
                let present = keep(&layer.glyph);
                removed += usize::from(!present);
                present
            });
            !layers.is_empty()
        });
        if let Some(graph) = &mut self.graph {
            removed += graph.retain_glyphs(keep);
        }
        removed
    }
}
