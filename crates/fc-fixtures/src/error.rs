//! Errors produced while locating fixtures.

/// An error produced while locating a fixture.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A fixture directory could not be created.
    #[error("failed to create fixture directory: {0}")]
    Io(#[from] std::io::Error),
    /// The generated UFO could not be written.
    #[error("failed to write UFO fixture: {0}")]
    UfoWrite(#[from] norad::error::FontWriteError),
    /// An internal fixture error.
    #[error("internal error: {0}")]
    Internal(String),
}
