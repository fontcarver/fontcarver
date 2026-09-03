//! Errors produced while reading or writing fonts.

/// An error produced while reading or writing a font.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An internal font I/O error.
    #[error("internal error: {0}")]
    Internal(String),
}
