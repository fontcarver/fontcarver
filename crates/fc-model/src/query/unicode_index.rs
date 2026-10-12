//! Transient reverse character map for session queries.
use crate::{Font, Glyph, GlyphName};
use std::collections::{BTreeSet, HashMap};

/// Reusable index retaining every glyph assigned to a scalar, including duplicates.
/// Call `update` after a glyph's identity or Unicode assignments change.
#[derive(Clone, Debug, Default)]
pub struct UnicodeIndex {
    by_scalar: HashMap<u32, BTreeSet<GlyphName>>,
    by_name: HashMap<GlyphName, Vec<u32>>,
}
impl UnicodeIndex {
    /// Build an index once when a document opens.
    pub fn new(font: &Font) -> Self {
        let mut index = Self::default();
        for (name, glyph) in &font.glyphs {
            index.update(name, Some(glyph));
        }
        index
    }
    /// Refresh one changed or removed glyph without visiting the rest of the font.
    pub fn update(&mut self, name: &GlyphName, glyph: Option<&Glyph>) {
        let next = glyph
            .map(|glyph| glyph.unicodes.as_slice())
            .unwrap_or_default();
        if self
            .by_name
            .get(name)
            .is_some_and(|current| current == next)
        {
            return;
        }
        if let Some(old) = self.by_name.remove(name) {
            for scalar in old {
                if let Some(names) = self.by_scalar.get_mut(&scalar) {
                    names.remove(name);
                    if names.is_empty() {
                        self.by_scalar.remove(&scalar);
                    }
                }
            }
        }
        if !next.is_empty() {
            for scalar in next {
                self.by_scalar
                    .entry(*scalar)
                    .or_default()
                    .insert(name.clone());
            }
            self.by_name.insert(name.clone(), next.to_vec());
        }
    }
    /// Iterate all glyphs assigned to this scalar, without allocating or scanning.
    pub fn glyphs(&self, scalar: u32) -> impl Iterator<Item = &GlyphName> {
        self.by_scalar.get(&scalar).into_iter().flatten()
    }
}
