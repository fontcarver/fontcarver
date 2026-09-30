//! Errors produced by the Tauri application shell.

/// An error produced while running the desktop application.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The command line asked for something the shell does not accept.
    #[error("{0}")]
    Usage(String),
    /// No graphical session is available, so no window can be opened.
    #[error(
        "no graphical session is available ({0}).\n\
         {NAME} is a desktop application and needs an X11 or Wayland session. \
         Use the `fontcarver` command-line interface for headless work."
    )]
    NoDisplay(String),
    /// The window system failed while the application was starting.
    #[error("the desktop shell could not start: {0}")]
    Startup(String),
    /// Tauri could not initialize or run the application.
    #[error("Tauri application error: {0}")]
    Tauri(#[from] tauri::Error),
}

use crate::cli::NAME;

impl Error {
    /// The process exit code this error should produce.
    ///
    /// `2` for a command line the shell does not accept, following the usual
    /// convention, and `1` for a failure while running. Never the `101` of an
    /// unwinding panic: a packaging smoke test has to be able to tell "this
    /// binary refused to start" from "this binary crashed".
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Usage(_) => 2,
            Self::NoDisplay(_) | Self::Startup(_) | Self::Tauri(_) => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_errors_are_distinguishable_from_startup_failures() {
        assert_eq!(Error::Usage("bad".into()).exit_code(), 2);
        assert_eq!(Error::NoDisplay("no DISPLAY".into()).exit_code(), 1);
        assert_eq!(Error::Startup("gtk".into()).exit_code(), 1);
    }

    #[test]
    fn a_missing_display_explains_itself_and_names_the_alternative() {
        let message =
            Error::NoDisplay("neither DISPLAY nor WAYLAND_DISPLAY is set".into()).to_string();
        assert!(
            message.contains("no graphical session is available"),
            "{message}"
        );
        assert!(message.contains("neither DISPLAY"), "{message}");
        assert!(message.contains("fontcarver"), "{message}");
    }
}
