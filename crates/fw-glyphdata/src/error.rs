//! Errors produced by glyph-data operations.

/// An error produced by glyph-data operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An internal glyph-data error.
    #[error("internal error: {0}")]
    Internal(String),
}
