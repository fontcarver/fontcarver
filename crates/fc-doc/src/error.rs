//! Error types for document sessions and persistence.

use std::path::PathBuf;

use fc_model::{GlyphName, LayerId, MasterId};

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
    /// A command contains an invalid index, scope, or parameter.
    ///
    /// Malformed contours surface here too: an outline command that would leave
    /// a contour `fc_model::Contour::try_to_bezpath` rejects — `AddContour` with
    /// a malformed contour, for instance — is reported as an invalid command
    /// rather than as [`Error::Model`], because one shared validator classifies
    /// every such result.
    #[error("invalid command: {0}")]
    InvalidCommand(String),
    /// Source ownership does not permit horizontal kerning edits.
    #[error(transparent)]
    KerningUnavailable(#[from] crate::kerning::HorizontalKerningEditBlock),
    /// An outline operation failed in the geometry engine.
    #[error("geometry operation failed: {0}")]
    Geometry(String),
    /// Metrics keys contain a reference cycle.
    #[error("metrics key cycle")]
    MetricsKeyCycle,
    /// The path is not a supported document format.
    #[error("unsupported document format: `{0}`")]
    UnsupportedFormat(PathBuf),
    /// The input is not a FontCarver document.
    #[error("invalid FontCarver document format `{0}`")]
    InvalidFcfFormat(String),
    /// The input requires a newer version of FontCarver.
    #[error("unsupported FontCarver document version {0}")]
    UnsupportedFcfVersion(u32),
    /// A master contains kerning values that JSON cannot represent losslessly.
    #[error("cannot save a native document with non-finite kerning in master `{0}`; source records remain in memory")]
    NonFiniteKerning(MasterId),
    /// The sfnt reader is not available in this checkout.
    #[error("sfnt opening is unavailable until fc-sfnt lands")]
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
    Model(#[from] fc_model::Error),
    /// A filesystem operation failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// JSON serialization or deserialization failed.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
