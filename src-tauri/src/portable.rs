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
    fn stores_live_in_separate_subdirectories() {
        let root = Path::new("/portable/data");
        assert_eq!(config_dir(root), root.join("config"));
        assert_eq!(webview_dir(root), root.join("webview"));
    }
}
