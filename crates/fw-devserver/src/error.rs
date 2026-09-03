//! Errors produced by the development server.

/// An error produced by the development server.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An internal development-server error.
    #[error("internal error: {0}")]
    Internal(String),
}
