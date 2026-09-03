//! Errors produced by the Tauri application shell.

/// An error produced while running the desktop application.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Tauri could not initialize or run the application.
    #[error("Tauri application error: {0}")]
    Tauri(#[from] tauri::Error),
}
