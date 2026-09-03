//! Errors produced while locating fixtures.

/// An error produced while locating a fixture.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An internal fixture error.
    #[error("internal error: {0}")]
    Internal(String),
}
