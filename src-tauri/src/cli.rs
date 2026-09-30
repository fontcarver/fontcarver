//! Command line and environment handling for the desktop shell.
//!
//! The shell is a window, not a tool, so its command line is deliberately tiny:
//! `--version` and `--help` exist because packaging and continuous integration
//! smoke-test every shipped binary with them, and everything else is refused
//! rather than quietly ignored. The display check exists for the same reason: a
//! headless runner that starts this binary must be told what is wrong instead of
//! being handed a GTK panic and a backtrace.

use std::ffi::OsString;

/// Binary name, as packaging and diagnostics spell it.
pub const NAME: &str = "fontcarver-desktop";

/// Version of the desktop shell, from its package metadata.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// What the command line asks the shell to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request {
    /// Open the application window.
    Launch,
    /// Print the version and exit successfully.
    Version,
    /// Print the usage text and exit successfully.
    Help,
}

/// The shell's usage text, used by `--help` and by every usage error.
pub fn usage() -> String {
    format!(
        "{NAME} {VERSION}
The FontCarver desktop application.

Usage: {NAME} [OPTIONS]

Options:
  -h, --help     Print this help and exit
  -V, --version  Print the version and exit

The shell opens a window and needs a graphical session. Documents are opened
from the File menu; `fontcarver` is the command-line interface."
    )
}

/// Parses the shell's arguments, without the program name.
///
/// # Errors
///
/// Returns the message to print when an argument is not one the shell accepts.
/// Refusing is deliberate: a shell that silently ignored `--porcelain` would let
/// a packaging mistake ship, and a desktop launcher never passes one.
pub fn parse<I>(args: I) -> Result<Request, String>
where
    I: IntoIterator,
    I::Item: Into<OsString>,
{
    let mut request = Request::Launch;
    for argument in args {
        let argument = argument.into();
        match argument.to_str() {
            Some("--version" | "-V") => request = Request::Version,
            Some("--help" | "-h") => return Ok(Request::Help),
            _ => {
                return Err(format!(
                    "unrecognized argument `{}`\n\n{}",
                    argument.to_string_lossy(),
                    usage()
                ))
            }
        }
    }
    Ok(request)
}

/// Whether these environment values describe a reachable graphical session.
///
/// Only the obvious case is decided here — no `DISPLAY` and no `WAYLAND_DISPLAY`
/// at all, which is what a container or an SSH session without forwarding looks
/// like. A variable that is set but points at a server that is not listening can
/// only be discovered by trying, so that case is handled where the window is
/// created.
#[cfg(all(unix, not(target_os = "macos")))]
pub(crate) fn display_configured(display: Option<&str>, wayland: Option<&str>) -> bool {
    [display, wayland]
        .into_iter()
        .flatten()
        .any(|value| !value.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_arguments_launch_the_window() {
        assert_eq!(parse(Vec::<String>::new()).unwrap(), Request::Launch);
    }

    #[test]
    fn version_and_help_are_accepted_in_both_spellings() {
        for argument in ["--version", "-V"] {
            assert_eq!(parse([argument]).unwrap(), Request::Version, "{argument}");
        }
        for argument in ["--help", "-h"] {
            assert_eq!(parse([argument]).unwrap(), Request::Help, "{argument}");
        }
        // Help wins over anything after it, because printing help is never wrong.
        assert_eq!(parse(["--help", "--nonsense"]).unwrap(), Request::Help);
    }

    #[test]
    fn anything_else_is_refused_with_the_usage_text() {
        for argument in ["--porcelain", "-x", "open", "--version=2", ""] {
            let message = parse([argument]).unwrap_err();
            assert!(
                message.starts_with("unrecognized argument"),
                "{argument}: {message}"
            );
            assert!(message.contains("Usage: fontcarver-desktop"), "{argument}");
        }
    }

    #[test]
    fn usage_and_version_name_the_binary_and_its_version() {
        assert!(usage().starts_with(&format!("{NAME} {VERSION}\n")));
        assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn a_session_needs_one_nonempty_display_variable() {
        assert!(display_configured(Some(":0"), None));
        assert!(display_configured(None, Some("wayland-0")));
        assert!(display_configured(Some(""), Some("wayland-1")));
        assert!(!display_configured(None, None));
        assert!(!display_configured(Some(""), Some("   ")));
    }
}
