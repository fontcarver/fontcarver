//! Errors produced by the document model.

/// An error produced by the document model.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An internal model error.
    #[error("internal error: {0}")]
    Internal(String),
}
