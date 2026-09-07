//! Errors produced while reading sfnt containers.

use std::{io, path::PathBuf};

/// An error produced while reading an sfnt container.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The input does not begin with a supported sfnt-container magic.
    #[error("unsupported font container")]
    UnsupportedContainer,
    /// A file could not be read.
    #[error("failed to read `{path}`: {source}")]
    Io {
        /// Path that could not be read.
        path: PathBuf,
        /// Underlying file-system error.
        source: io::Error,
    },
    /// An sfnt structure or table was malformed.
    #[error("failed to parse {table}: {reason}")]
    Parse {
        /// Table tag or container-level structure name.
        table: &'static str,
        /// Parser diagnostic.
        reason: String,
    },
}

impl Error {
    pub(crate) fn parse(table: &'static str, error: impl std::fmt::Display) -> Self {
        Self::Parse {
            table,
            reason: error.to_string(),
        }
    }
}
