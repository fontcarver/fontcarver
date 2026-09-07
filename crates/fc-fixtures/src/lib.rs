//! Paths to Font Cutter's committed and optional external test fonts.

#![warn(missing_docs)]

use std::{
    env,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

pub mod error;
pub mod ufo;

pub use error::Error;

/// Returns the path to a committed font fixture in `fixtures/fonts`.
#[must_use]
pub fn path(name: &str) -> PathBuf {
    workspace_root().join("fixtures").join("fonts").join(name)
}

/// Returns the path to a committed UFO fixture in `fixtures/ufo`.
#[must_use]
pub fn ufo_path(name: &str) -> PathBuf {
    workspace_root().join("fixtures").join("ufo").join(name)
}

/// Returns the path to an available external fixture.
///
/// Known keys are `source-han-sans-sc-otf` and `fontforge-tests`. Unknown keys
/// and paths that are absent on the current machine return `None`.
#[must_use]
pub fn external(key: &str) -> Option<PathBuf> {
    let candidate = match key {
        "source-han-sans-sc-otf" => source_han_sans_path(),
        "fontforge-tests" => workspace_root()
            .join("fontforge")
            .join("tests")
            .join("fonts"),
        _ => return None,
    };

    candidate.exists().then_some(candidate)
}

fn source_han_sans_path() -> PathBuf {
    source_han_candidate(
        env::var_os("FC_SOURCE_HAN_SANS").filter(|value| !value.is_empty()),
        &primary_repository_root(),
    )
}

fn source_han_candidate(override_path: Option<impl AsRef<OsStr>>, primary_root: &Path) -> PathBuf {
    override_path.map_or_else(
        || {
            primary_root
                .parent()
                .unwrap_or(primary_root)
                .join("SourceHanSanOTF")
                .join("SimplifiedChinese")
                .join("SourceHanSansSC-Regular.otf")
        },
        |path| PathBuf::from(path.as_ref()),
    )
}

fn primary_repository_root() -> PathBuf {
    primary_repository_root_from(&workspace_root())
}

fn primary_repository_root_from(root: &Path) -> PathBuf {
    let dot_git = root.join(".git");
    if dot_git.is_dir() {
        return root.to_path_buf();
    }
    let Some(git_dir) = fs::read_to_string(dot_git)
        .ok()
        .and_then(|contents| {
            contents
                .strip_prefix("gitdir:")
                .map(str::trim)
                .map(PathBuf::from)
        })
        .map(|path| {
            if path.is_absolute() {
                path
            } else {
                root.join(path)
            }
        })
    else {
        return root.to_path_buf();
    };
    let Ok(git_dir) = fs::canonicalize(git_dir) else {
        return root.to_path_buf();
    };
    git_dir
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .filter(|candidate| candidate.join(".git").is_dir())
        .map_or_else(|| root.to_path_buf(), Path::to_path_buf)
}

/// Returns an external fixture path or skips the current unit-returning test.
#[macro_export]
macro_rules! skip_unless_external {
    ($key:expr) => {{
        let key = $key;
        match $crate::external(key) {
            Some(path) => path,
            None => {
                eprintln!("skipping test: external fixture {key:?} is unavailable");
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
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use super::{external, path, primary_repository_root_from, source_han_candidate};

    #[test]
    fn committed_dejavu_fixture_exists() {
        assert!(path("DejaVuSans.ttf").is_file());
    }

    #[test]
    fn unknown_external_fixture_is_absent() {
        assert!(external("not-a-fixture").is_none());
    }

    #[test]
    fn source_han_override_wins_over_the_sibling_default() {
        assert_eq!(
            source_han_candidate(Some("/fonts/custom.otf"), Path::new("/repos/fontcutter")),
            PathBuf::from("/fonts/custom.otf")
        );
    }

    #[test]
    fn linked_worktree_resolves_the_primary_repository() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let primary = temp.path().join("fontcutter");
        let worktree = primary.join(".worktrees/task");
        fs::create_dir_all(primary.join(".git/worktrees/task")).expect("create git metadata");
        fs::create_dir_all(&worktree).expect("create worktree");
        fs::write(worktree.join(".git"), "gitdir: ../../.git/worktrees/task\n")
            .expect("write git pointer");

        assert_eq!(
            primary_repository_root_from(&worktree),
            fs::canonicalize(primary).expect("canonical primary")
        );
    }

    #[test]
    fn source_han_default_is_beside_the_primary_repository() {
        assert_eq!(
            source_han_candidate(None::<&str>, Path::new("/repos/fontcutter")),
            PathBuf::from("/repos/SourceHanSanOTF/SimplifiedChinese/SourceHanSansSC-Regular.otf")
        );
    }

    #[test]
    fn missing_external_fixture_skips_without_reevaluating_key() {
        let mut evaluations = 0;
        crate::skip_unless_external!({
            evaluations += 1;
            assert_eq!(evaluations, 1);
            "missing-test-fixture"
        });
        panic!("the macro did not return early");
    }
}
