//! Errors returned by source-format importers and exporters.

use std::path::PathBuf;

/// An error encountered while reading or writing a source font.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Native source data is malformed or unsupported.
    #[error("{format}: {reason}")]
    Source {
        /// Human-readable format name.
        format: &'static str,
        /// Specific parsing or representation failure.
        reason: String,
    },
    /// A source export would silently omit saved feature edits.
    #[error("apply or discard the feature draft before exporting a source font")]
    UnappliedFeatureDraft,
    /// Binary layout must be converted before a source format can represent it.
    #[error("imported layout requires validated conversion before source export")]
    ImportedLayoutRequiresConversion,
    /// A UFO could not be loaded.
    #[error("failed to load UFO: {0}")]
    UfoLoad(#[from] norad::error::FontLoadError),
    /// A UFO could not be serialized.
    #[error("failed to write UFO: {0}")]
    UfoWrite(#[from] norad::error::FontWriteError),
    /// A designspace could not be loaded.
    #[error("failed to load designspace: {0}")]
    DesignSpaceLoad(#[from] norad::error::DesignSpaceLoadError),
    /// A designspace could not be serialized.
    #[error("failed to write designspace: {0}")]
    DesignSpaceWrite(#[from] norad::error::DesignSpaceSaveError),
    /// A property list could not be read or converted.
    #[error("property-list error: {0}")]
    Plist(#[from] plist::Error),
    /// JSON-backed model metadata could not be converted.
    #[error("metadata conversion failed: {0}")]
    Json(#[from] serde_json::Error),
    /// A filesystem operation failed.
    #[error("filesystem operation failed for {path}: {source}")]
    Io {
        /// Path involved in the failed operation.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// The requested master is not present in the font.
    #[error("master {0} does not exist")]
    MissingMaster(String),
    /// A designspace source could not be associated with a master.
    #[error("designspace source {0} has no corresponding master")]
    MissingSourceMaster(String),
    /// A model or source name is invalid in UFO.
    #[error("invalid UFO name {0:?}")]
    InvalidName(String),
    /// An object identifier is invalid in UFO.
    #[error("invalid UFO identifier {0:?}")]
    InvalidIdentifier(String),
    /// A designspace axis tag is invalid.
    #[error("invalid designspace axis tag {0:?}")]
    InvalidAxisTag(String),
    /// A static font has no axes and must be exported directly as UFO.
    #[error("a designspace requires at least one axis; the font was written as a UFO instead")]
    NoAxes,
    /// A serialized property-list value cannot be represented by JSON.
    #[error("unsupported property-list value: {0}")]
    UnsupportedPlist(String),
    /// A path embedded in a UFO attempts to leave the UFO directory.
    #[error("unsafe path in UFO contents: {0}")]
    UnsafePath(PathBuf),
}

impl Error {
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}
