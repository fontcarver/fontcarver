//! The desktop shell's command line and its behaviour without a display.
//!
//! These run the shipped binary, because the property being checked is what a
//! packaging smoke test sees: the exit code and what lands on stdout or stderr.
//! None of them opens a window.

use std::process::{Command, Output};

/// Runs the desktop binary with a deliberately unusable window-system
/// environment, so no case here can open a window on the developer's desktop.
fn run(args: &[&str], display: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_fontcarver-desktop"));
    command.args(args);
    command.env_remove("WAYLAND_DISPLAY");
    command.env("GDK_BACKEND", "x11");
    match display {
        Some(value) => command.env("DISPLAY", value),
        None => command.env_remove("DISPLAY"),
    };
    command.output().expect("the desktop binary should run")
}

#[test]
fn version_prints_the_binary_name_and_version_and_succeeds() {
    let output = run(&["--version"], None);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        stdout.trim(),
        format!("fontcarver-desktop {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn help_prints_usage_and_succeeds() {
    for argument in ["--help", "-h"] {
        let output = run(&[argument], None);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(output.status.success(), "{argument}: {output:?}");
        assert!(stdout.contains("Usage: fontcarver-desktop"), "{stdout}");
        assert!(stdout.contains("--version"), "{stdout}");
    }
}

#[test]
fn an_unrecognized_argument_is_refused_with_code_two() {
    let output = run(&["--porcelain"], None);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        stderr.contains("unrecognized argument `--porcelain`"),
        "{stderr}"
    );
    assert!(stderr.contains("Usage: fontcarver-desktop"), "{stderr}");
}

/// The case the continuous-integration smoke test hits: a runner with no
/// graphical session at all. It must be told why, and must not look like a
/// crash.
#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn without_any_display_variable_it_explains_itself_and_does_not_panic() {
    let output = run(&[], None);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_ne!(
        output.status.code(),
        Some(101),
        "an unwinding panic: {stderr}"
    );
    assert!(
        stderr.contains("no graphical session is available"),
        "{stderr}"
    );
    assert!(
        stderr.contains("neither DISPLAY nor WAYLAND_DISPLAY"),
        "{stderr}"
    );
}

/// The other half: a display variable that is set but points at no server. GTK
/// panics inside Tauri there, and the shell has to turn that into the same kind
/// of message rather than a backtrace and exit code 101.
#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn an_unreachable_display_is_reported_rather_than_panicking() {
    let output = run(&[], Some(":987"));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_ne!(
        output.status.code(),
        Some(101),
        "an unwinding panic reached the process exit: {stderr}"
    );
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(
        stderr.contains("no graphical session is available") || stderr.contains("could not start"),
        "{stderr}"
    );
    assert!(
        !stderr.contains("note: run with `RUST_BACKTRACE"),
        "the default panic hook still printed: {stderr}"
    );
}
