//! Errors exposed by the kernel API.

/// An error exposed by the kernel API.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An internal API error.
    #[error("internal error: {0}")]
    Internal(String),
}
