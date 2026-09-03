//! Errors produced by the document model.

use serde::{Deserialize, Serialize};

use crate::{GlyphName, MasterId};

/// An error produced by the document model.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum Error {
    /// A glyph name is already present in the font.
    #[error("glyph `{glyph}` already exists")]
    DuplicateGlyph {
        /// The duplicate name.
        glyph: GlyphName,
    },
    /// A requested glyph or component base does not exist.
    #[error("glyph `{glyph}` does not exist")]
    MissingGlyph {
        /// The missing name.
        glyph: GlyphName,
    },
    /// A glyph has no layer for a requested master.
    #[error("glyph `{glyph}` has no layer for master `{master}`")]
    MissingMasterLayer {
        /// The affected glyph.
        glyph: GlyphName,
        /// The requested master.
        master: MasterId,
    },
    /// Components contain a recursive reference.
    #[error("component cycle at glyph `{glyph}`")]
    ComponentCycle {
        /// A glyph participating in the cycle.
        glyph: GlyphName,
    },
    /// Component nesting exceeds the supported recursion limit.
    #[error("component nesting exceeds the depth limit at glyph `{glyph}`")]
    ComponentDepthLimit {
        /// The glyph at which the limit was reached.
        glyph: GlyphName,
    },
    /// A contour has a malformed point sequence.
    #[error("malformed contour")]
    MalformedContour,
    /// An OpenType tag is not exactly four bytes long.
    #[error("invalid four-byte OpenType tag `{tag}`")]
    InvalidTag {
        /// The invalid textual representation.
        tag: String,
    },
}
