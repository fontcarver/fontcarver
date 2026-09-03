//! Errors produced by command-line operations.

use std::path::PathBuf;

/// An error produced by a command-line operation.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A document could not be opened.
    #[error("failed to open `{}`: {source}", path.display())]
    Open {
        /// Input document path.
        path: PathBuf,
        /// Document-layer failure.
        source: fw_doc::Error,
    },
    /// A font collection could not be read for face metadata.
    #[error("failed to read `{}`: {source}", path.display())]
    Read {
        /// Input collection path.
        path: PathBuf,
        /// Filesystem failure.
        source: std::io::Error,
    },
    /// A font collection's face metadata could not be decoded.
    #[error("failed to inspect faces in `{}`: {source}", path.display())]
    Faces {
        /// Input collection path.
        path: PathBuf,
        /// Font decoding failure.
        source: fw_sfnt::Error,
    },
    /// An open document could not be accessed.
    #[error(transparent)]
    Document(#[from] fw_doc::Error),
    /// An output document could not be saved.
    #[error("failed to save `{}`: {source}", path.display())]
    Save {
        /// Output document path.
        path: PathBuf,
        /// Document-layer failure.
        source: fw_doc::Error,
    },
    /// The shared API facade rejected a query.
    #[error(transparent)]
    Api(#[from] fw_api::ApiError),
    /// CLI JSON could not be encoded or decoded.
    #[error("failed to process JSON output: {0}")]
    Json(#[from] serde_json::Error),
    /// A glyph-filter expression was not recognized.
    #[error("invalid glyph filter '{0}'")]
    InvalidFilter(String),
    /// Conversion to the requested output format is not available yet.
    #[error(
        "unsupported output format '{extension}': only .fwf and .fwf.gz are supported in this version"
    )]
    UnsupportedOutput {
        /// Requested filename extension, without the leading dot.
        extension: String,
    },
}

impl Error {
    /// Returns the process exit code associated with this error.
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::UnsupportedOutput { .. } => 2,
            _ => 1,
        }
    }
}
