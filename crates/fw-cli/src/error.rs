//! Errors produced by command-line operations.

/// An error produced by a command-line operation.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An internal command-line error.
    #[error("internal error: {0}")]
    Internal(String),
}
