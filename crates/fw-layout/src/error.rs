//! Errors returned by the layout and shaping service.

use fw_model::MasterId;

/// An error that prevents a shaping request from being answered.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The requested master is not present in the font.
    #[error("master {0} is not present in the font")]
    MissingMaster(MasterId),
    /// The document could not be compiled into a shaping font.
    #[error("could not compile a shaping font: {0}")]
    Compile(#[from] fw_compile::Error),
    /// The compiled font could not be parsed back.
    #[error("could not parse the compiled shaping font: {0}")]
    Parse(String),
    /// A shared cache lock was poisoned by a panicking thread.
    #[error("shaping cache lock is poisoned")]
    LockPoisoned,
}
