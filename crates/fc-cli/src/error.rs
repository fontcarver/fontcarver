//! Errors produced by command-line operations.

use std::path::PathBuf;

/// An error produced by a command-line operation.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A read-only check found at least one error in the document.
    #[error("document problems found; see the check report")]
    ProblemsFound,
    /// A document could not be opened.
    #[error("failed to open `{}`: {source}", path.display())]
    Open {
        /// Input document path.
        path: PathBuf,
        /// Document-layer failure.
        source: fc_doc::Error,
    },
    /// A source font could not be imported.
    #[error("failed to import `{}`: {source}", path.display())]
    OpenSource {
        /// Input document path.
        path: PathBuf,
        /// Source-format failure.
        source: fc_formats::Error,
    },
    /// An open document could not be accessed.
    #[error(transparent)]
    Document(#[from] fc_doc::Error),
    /// An output document could not be saved.
    #[error("failed to save `{}`: {source}", path.display())]
    Save {
        /// Output document path.
        path: PathBuf,
        /// Document-layer failure.
        source: fc_doc::Error,
    },
    /// The shared API facade rejected a query.
    #[error(transparent)]
    Api(#[from] fc_api::ApiError),
    /// CLI JSON could not be encoded or decoded.
    #[error("failed to process JSON output: {0}")]
    Json(#[from] serde_json::Error),
    /// A glyph-filter expression was not recognized.
    #[error("invalid glyph filter '{0}'")]
    InvalidFilter(String),
    /// The requested output format is not supported by this command.
    #[error("unsupported output format '{extension}': supported formats are {supported}")]
    UnsupportedOutput {
        /// Requested filename extension, without the leading dot.
        extension: String,
        /// Comma-separated list of formats the command accepts.
        supported: &'static str,
    },
    /// The requested master does not exist in the document.
    #[error("unknown master '{name}'")]
    UnknownMaster {
        /// Master name or identifier that was requested.
        name: String,
    },
}

impl Error {
    /// Returns the process exit code associated with this error.
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::UnsupportedOutput { .. } => 2,
            Self::UnknownMaster { .. } => 2,
            _ => 1,
        }
    }
}
