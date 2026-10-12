//! Named, editable OpenType SVG documents. XML is data, never executable UI markup.

use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::GlyphName;

mod xml;
pub use xml::{rewrite_ids, validate_xml};

/// Maximum UTF-8 bytes in one SVG document.
pub const MAX_SVG_DOCUMENT_BYTES: usize = 2 * 1024 * 1024;
/// Maximum uncompressed source bytes in an SVG table.
pub const MAX_SVG_TABLE_BYTES: usize = 32 * 1024 * 1024;

/// One shared SVG document with explicit glyph-to-element bindings.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SvgDocument {
    /// UTF-8 SVG XML, retaining shared definitions and presentation attributes.
    pub source: String,
    /// Glyph names mapped to element IDs, independent of export glyph ordering.
    pub glyphs: BTreeMap<GlyphName, String>,
}

/// Editable SVG table; no glyph may occur in more than one document.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SvgTable {
    /// Documents in editor order. Binary records are sorted at export.
    pub documents: Vec<SvgDocument>,
}

impl SvgTable {
    /// Validate XML, bindings, unique coverage and bounded source size.
    pub fn validate(&self, order: &[GlyphName]) -> Result<(), String> {
        if self.documents.len() > 65535 || order.len() > 65535 {
            return Err("SVG document or glyph count exceeds 65535".into());
        }
        let names: HashSet<_> = order.iter().collect();
        if names.len() != order.len() {
            return Err("SVG needs unique glyph names".into());
        }
        let mut covered = HashSet::new();
        let mut bytes = 0usize;
        for document in &self.documents {
            bytes = bytes.saturating_add(document.source.len());
            if bytes > MAX_SVG_TABLE_BYTES || document.glyphs.is_empty() {
                return Err("SVG source budget exceeded or document has no glyphs".into());
            }
            let ids = validate_xml(&document.source)?;
            for (name, id) in &document.glyphs {
                if !names.contains(name) || !covered.insert(name) || !ids.contains(id) {
                    return Err(format!(
                        "SVG missing/duplicate glyph or element: {name} → {id}"
                    ));
                }
            }
        }
        Ok(())
    }

    /// Update name bindings without changing XML identifiers.
    pub fn rename_glyph(&mut self, from: &str, to: &GlyphName) {
        for document in &mut self.documents {
            if let Some(id) = document.glyphs.remove(from) {
                document.glyphs.insert(to.clone(), id);
            }
        }
    }

    /// Duplicate a binding, sharing the existing source document.
    pub fn duplicate_glyph(&mut self, from: &GlyphName, to: &GlyphName) {
        for document in &mut self.documents {
            if let Some(id) = document.glyphs.get(from).cloned() {
                document.glyphs.insert(to.clone(), id);
            }
        }
    }

    /// Remove bindings, retaining shared definitions for surviving glyphs.
    pub fn retain_glyphs(&mut self, mut keep: impl FnMut(&GlyphName) -> bool) -> usize {
        let mut removed = 0;
        for document in &mut self.documents {
            document.glyphs.retain(|name, _| {
                let present = keep(name);
                removed += usize::from(!present);
                present
            });
        }
        self.documents
            .retain(|document| !document.glyphs.is_empty());
        removed
    }

    /// Estimate retained undo memory without serializing the XML.
    pub fn estimated_bytes(&self) -> usize {
        self.documents
            .iter()
            .map(|d| {
                d.source.len()
                    + d.glyphs
                        .iter()
                        .map(|(name, id)| name.len() + id.len() + 80)
                        .sum::<usize>()
                    + 64
            })
            .sum()
    }
}
