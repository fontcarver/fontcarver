//! Paths to FontWorkshop's committed and optional external test fonts.

use std::path::{Path, PathBuf};

pub mod error;

pub use error::Error;

/// Returns the path to a committed font fixture in `fixtures/fonts`.
#[must_use]
pub fn path(name: &str) -> PathBuf {
    workspace_root().join("fixtures").join("fonts").join(name)
}

/// Returns the path to an available external fixture.
///
/// Known keys are `source-han-sans-sc-otf` and `fontforge-tests`. Unknown keys
/// and paths that are absent on the current machine return `None`.
#[must_use]
pub fn external(key: &str) -> Option<PathBuf> {
    let candidate = match key {
        "source-han-sans-sc-otf" => PathBuf::from(
            "/home/chen/githubprojects/pixelfonts/SourceHanSanOTF/SimplifiedChinese/SourceHanSansSC-Regular.otf",
        ),
        "fontforge-tests" => workspace_root().join("fontforge").join("tests").join("fonts"),
        _ => return None,
    };

    candidate.exists().then_some(candidate)
}

/// Returns an external fixture path or skips the current unit-returning test.
#[macro_export]
macro_rules! skip_unless_external {
    ($key:expr) => {{
        match $crate::external($key) {
            Some(path) => path,
            None => {
                eprintln!("skipping test: external fixture {:?} is unavailable", $key);
                return;
            }
        }
    }};
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map_or_else(
            || PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            Path::to_path_buf,
        )
}

#[cfg(test)]
mod tests {
    use super::path;

    #[test]
    fn committed_dejavu_fixture_exists() {
        assert!(path("DejaVuSans.ttf").is_file());
    }
}
