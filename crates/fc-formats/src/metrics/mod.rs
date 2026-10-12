//! Metrics-only interchange for AFM and Windows PFM files.
//!
//! AFM supports horizontal advances, optional bounding boxes, ligatures and
//! direct horizontal pair kerning. Vertical metrics sets, composites, track
//! kerning and multiple-master extensions are rejected. Reading a metrics file
//! creates an outline-free font; exporting deliberately selects only metrics.
//! AFM numbers are in 1000 units per em. PFM has its own binary data model.

mod afm;
mod bridge;
pub mod pfm;

use std::collections::BTreeMap;

use fc_model::GlyphName;
use serde::{Deserialize, Serialize};

use crate::Error;

pub use afm::{read_afm, write_afm};
pub use bridge::{apply_to_font, from_font, into_font, read_afm_path, write_afm_font};

/// Horizontal AFM metrics; numeric values use 1000 units per em.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MetricsFont {
    /// PostScript font name, without whitespace or PostScript delimiters.
    pub font_name: String,
    /// Supported AFM global keywords and their original textual values.
    ///
    /// Includes names, encoding declarations, font bounds, standard stem widths,
    /// ascender/descender, cap/x height and horizontal direction properties.
    /// Unknown keywords and unsupported non-default metric sets are rejected.
    #[serde(default)]
    pub properties: BTreeMap<String, String>,
    /// Comments, preserved as text without their original file position.
    #[serde(default)]
    pub comments: Vec<String>,
    /// Character records in source order.
    #[serde(default)]
    pub glyphs: Vec<GlyphMetrics>,
    /// Direct horizontal kerning pairs in source order.
    #[serde(default)]
    pub kerning: Vec<KerningPair>,
}

/// One horizontal character metric record.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GlyphMetrics {
    /// Encoding-specific code, with `None` representing AFM code -1.
    /// This is not a Unicode scalar value.
    pub code: Option<u16>,
    /// PostScript glyph name.
    pub name: GlyphName,
    /// Horizontal advance in 1000-em units.
    pub width: f64,
    /// Optional `[xmin, ymin, xmax, ymax]` in 1000-em units.
    pub bbox: Option<[f64; 4]>,
    /// Named ligature substitutions declared by this character.
    #[serde(default)]
    pub ligatures: Vec<Ligature>,
}

/// An AFM two-character ligature declaration.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Ligature {
    /// Glyph following the character that owns this declaration.
    pub successor: GlyphName,
    /// Replacement glyph name.
    pub replacement: GlyphName,
}

/// A direct horizontal kerning adjustment in 1000-em units.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct KerningPair {
    /// Left glyph name.
    pub left: GlyphName,
    /// Right glyph name.
    pub right: GlyphName,
    /// Signed horizontal adjustment.
    pub value: f64,
}

pub(crate) fn err(reason: impl Into<String>) -> Error {
    Error::Source {
        format: "AFM",
        reason: reason.into(),
    }
}

pub(crate) use afm::validate;
