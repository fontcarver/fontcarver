//! Portable mode: state beside the executable instead of in the user's profile.
//!
//! The portable zip ships `PORTABLE.txt` next to the executable. Its presence is
//! the switch; an installed build never has it.

use std::path::{Path, PathBuf};

pub(crate) const MARKER: &str = "PORTABLE.txt";

/// The data root for a portable run, or `None` for an installed one.
///
/// Callers pass a resolved path: a shortcut or link must find the marker beside
/// the real executable, not beside the link.
pub(crate) fn root(executable: &Path) -> Option<PathBuf> {
    let dir = executable.parent()?;
    dir.join(MARKER).is_file().then(|| dir.join("data"))
}

pub(crate) fn config_dir(root: &Path) -> PathBuf {
    root.join("config")
}

pub(crate) fn webview_dir(root: &Path) -> PathBuf {
    root.join("webview")
}

/// The text shown when the system WebView runtime cannot be found.
///
/// The portable zip has no installer to bootstrap WebView2, so on a Windows host
/// without it this is the only feedback the user gets.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn webview_missing_message(detail: &str) -> String {
    format!(
        "FontCarver needs the Microsoft Edge WebView2 Runtime, which was not found.\n\n\
         Windows 11 includes it. On Windows 10, install it from \
         https://developer.microsoft.com/microsoft-edge/webview2/ and start FontCarver again.\n\n\
         Detail: {detail}"
    )
}

/// This run's portable data root, managed so every window can reach it.
pub(crate) struct Root(pub(crate) Option<PathBuf>);

/// The WebView data directory to apply, or `None` to leave Tauri's default.
///
/// Every window in a run must agree, or windows stop sharing localStorage.
pub(crate) fn webview_directory(root: &Root) -> Option<PathBuf> {
    root.0.as_deref().map(webview_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_file_beside_the_executable_selects_a_data_root() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("fontcarver-desktop.exe");
        std::fs::write(&exe, b"MZ").unwrap();
        std::fs::write(dir.path().join(MARKER), b"portable").unwrap();
        assert_eq!(root(&exe), Some(dir.path().join("data")));
    }

    #[test]
    fn no_marker_means_an_installed_build() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("fontcarver-desktop.exe");
        std::fs::write(&exe, b"MZ").unwrap();
        assert_eq!(root(&exe), None);
    }

    #[test]
    fn a_directory_named_like_the_marker_does_not_count() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("fontcarver-desktop.exe");
        std::fs::write(&exe, b"MZ").unwrap();
        std::fs::create_dir(dir.path().join(MARKER)).unwrap();
        assert_eq!(root(&exe), None);
    }

    #[cfg(unix)]
    #[test]
    fn a_link_to_the_executable_finds_the_marker_beside_the_real_file() {
        let real = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        let exe = real.path().join("fontcarver-desktop");
        std::fs::write(&exe, b"\x7fELF").unwrap();
        std::fs::write(real.path().join(MARKER), b"portable").unwrap();
        let link = elsewhere.path().join("fontcarver-desktop");
        std::os::unix::fs::symlink(&exe, &link).unwrap();
        let resolved = std::fs::canonicalize(&link).unwrap();
        assert_eq!(root(&resolved), Some(real.path().canonicalize().unwrap().join("data")));
    }

    #[test]
    fn the_main_window_is_created_in_code_so_its_data_directory_can_be_set() {
        // Tauri resolves a config-file data_directory under %LOCALAPPDATA% and
        // rejects absolute paths, so the portable directory can only be applied
        // to a window built in code. The config must therefore not auto-create it.
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let main = config["app"]["windows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|window| window["label"] == "main")
            .unwrap();
        assert_eq!(main["create"], serde_json::Value::Bool(false));
    }

    #[test]
    fn webview_directory_is_only_set_in_portable_mode() {
        assert_eq!(webview_directory(&Root(None)), None);
        let root = PathBuf::from("/p/data");
        assert_eq!(
            webview_directory(&Root(Some(root.clone()))),
            Some(root.join("webview"))
        );
    }

    #[test]
    fn a_missing_webview_runtime_is_explained_in_plain_words() {
        let message = webview_missing_message("class not registered");
        assert!(message.contains("WebView2"));
        assert!(message.contains("https://developer.microsoft.com/microsoft-edge/webview2/"));
        assert!(message.contains("class not registered"));
    }

    #[test]
    fn stores_live_in_separate_subdirectories() {
        let root = Path::new("/portable/data");
        assert_eq!(config_dir(root), root.join("config"));
        assert_eq!(webview_dir(root), root.join("webview"));
    }
}
