//! Errors produced by document sessions.

/// An error produced by a document session.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An internal document error.
    #[error("internal error: {0}")]
    Internal(String),
}
