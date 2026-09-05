//! Errors returned by the compiler pipeline.

use fc_model::MasterId;

/// An error that prevents an export from being produced.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The requested master is not in the font.
    #[error("master {0} is not present in the font")]
    MissingMaster(MasterId),
    /// The font has no exported glyphs.
    #[error("the font has no exported glyphs")]
    NoExportedGlyphs,
    /// The model's units-per-em value is outside the OpenType range.
    #[error("units per em {0} is outside 16..=16384")]
    InvalidUnitsPerEm(u16),
    /// The model cannot be normalized into a valid compiler source.
    #[error("invalid compiler source: {0}")]
    InvalidSource(String),
    /// fontc rejected the in-memory source or failed to build it.
    #[error("fontc compilation failed: {0}")]
    Fontc(#[from] fontc::Error),
    /// An sfnt or table could not be parsed.
    #[error("failed to parse {context}: {reason}")]
    Parse {
        /// The container or table being parsed.
        context: &'static str,
        /// Upstream parser detail.
        reason: String,
    },
    /// A table could not be encoded.
    #[error("failed to write {context}: {reason}")]
    Write {
        /// The table or container being written.
        context: &'static str,
        /// Upstream writer detail.
        reason: String,
    },
    /// An input offset, size, or count is outside the representable format range.
    #[error("{0} exceeds the format limits")]
    Limit(&'static str),
    /// The selected export format is defined by the API but not handled by this crate.
    #[error("{0} export is not supported by fc-compile")]
    UnsupportedFormat(&'static str),
    /// WOFF2's native encoder rejected the sfnt.
    #[error("WOFF2 compression failed")]
    Woff2Compression,
    /// Compression or output assembly failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Error {
    pub(crate) fn parse(context: &'static str, error: impl std::fmt::Display) -> Self {
        Self::Parse {
            context,
            reason: error.to_string(),
        }
    }

    pub(crate) fn write(context: &'static str, error: impl std::fmt::Display) -> Self {
        Self::Write {
            context,
            reason: error.to_string(),
        }
    }
}
