//! Error types for document sessions and persistence.

use std::path::PathBuf;

use fw_model::{GlyphName, LayerId, MasterId};

/// An error produced by a document session.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A trusted internal state could not be restored.
    #[error("internal document error: {0}")]
    Internal(String),
    /// A requested document is not open in the session.
    #[error("document {0} is not open")]
    DocumentNotFound(u32),
    /// A requested glyph does not exist.
    #[error("glyph `{0}` does not exist")]
    MissingGlyph(GlyphName),
    /// A requested layer does not exist in its glyph.
    #[error("layer `{layer}` does not exist in glyph `{glyph}`")]
    MissingLayer {
        /// Glyph expected to contain the layer.
        glyph: GlyphName,
        /// Missing layer identifier.
        layer: LayerId,
    },
    /// A requested master does not exist.
    #[error("master `{0}` does not exist")]
    MissingMaster(MasterId),
    /// An edit token does not identify the active coalesced edit.
    #[error("edit token is not active")]
    InvalidEditToken,
    /// The path is not a supported document format.
    #[error("unsupported document format: `{0}`")]
    UnsupportedFormat(PathBuf),
    /// The input is not a FontWorkshop document.
    #[error("invalid FontWorkshop document format `{0}`")]
    InvalidFwfFormat(String),
    /// The input requires a newer version of FontWorkshop.
    #[error("unsupported FontWorkshop document version {0}")]
    UnsupportedFwfVersion(u32),
    /// The sfnt reader is not available in this checkout.
    #[error("sfnt opening is unavailable until fw-sfnt lands")]
    SfntUnavailable,
    /// An sfnt reader error.
    #[error("could not open binary font: {0}")]
    Sfnt(String),
    /// A shared session lock was poisoned.
    #[error("document session lock is poisoned")]
    LockPoisoned,
    /// A document callback tried to access a session recursively in a debug build.
    #[error("session callbacks must not call back into the session")]
    Reentrant,
    /// An underlying model invariant was rejected.
    #[error(transparent)]
    Model(#[from] fw_model::Error),
    /// A filesystem operation failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// JSON serialization or deserialization failed.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
