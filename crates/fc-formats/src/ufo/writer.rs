//! Non-destructive incremental UFO package writer.

use std::{
    collections::BTreeSet,
    ffi::OsStr,
    fs,
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use walkdir::WalkDir;

use super::UfoWriteOptions;
use crate::Error;

static UNIQUE: AtomicU64 = AtomicU64::new(0);
const OPTIONAL_ROOT_FILES: &[&str] = &[
    "fontinfo.plist",
    "lib.plist",
    "groups.plist",
    "kerning.plist",
    "features.fea",
];

pub(crate) fn write(
    font: &norad::Font,
    destination: &Path,
    options: &UfoWriteOptions,
) -> Result<(), Error> {
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|source| Error::io(parent, source))?;
    if destination.exists() && !destination.is_dir() {
        return Err(Error::io(
            destination,
            std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "UFO target is not a directory",
            ),
        ));
    }

    let staging = unique_sibling(destination, "ufo-stage");
    let guard = StagingGuard(staging.clone());
    font.save(&staging)?;
    // Norad normalizes CRLF; restore the assembled source spelling before synchronization.
    if !font.features.is_empty() {
        let source_path = staging.join("features.fea");
        fs::write(&source_path, &font.features)
            .map_err(|source| Error::io(&source_path, source))?;
    }
    if font.meta.creator.as_deref() != Some("org.linebender.norad") {
        plist::to_file_xml(staging.join("metainfo.plist"), &font.meta)?;
    }

    let old_glifs = contents_glifs(destination)?;
    let new_glifs = contents_glifs(&staging)?;
    fs::create_dir_all(destination).map_err(|source| Error::io(destination, source))?;
    synchronize_files(&staging, destination, options.incremental)?;
    remove_absent_optional_files(&staging, destination)?;
    for stale in old_glifs.difference(&new_glifs) {
        let path = safe_join(destination, stale)?;
        if path.is_file() {
            fs::remove_file(&path).map_err(|source| Error::io(&path, source))?;
        }
    }
    drop(guard);
    Ok(())
}

pub(crate) fn write_file_incrementally(source: &Path, destination: &Path) -> Result<(), Error> {
    let bytes = fs::read(source).map_err(|error| Error::io(source, error))?;
    if fs::read(destination).ok().as_deref() == Some(bytes.as_slice()) {
        return Ok(());
    }
    atomic_write(destination, &bytes)
}

fn synchronize_files(staging: &Path, destination: &Path, incremental: bool) -> Result<(), Error> {
    for entry in WalkDir::new(staging).min_depth(1) {
        let entry = entry.map_err(walkdir_error)?;
        let relative = entry
            .path()
            .strip_prefix(staging)
            .map_err(|error| Error::UnsafePath(error.to_string().into()))?;
        let target = safe_join(destination, relative)?;
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).map_err(|source| Error::io(&target, source))?;
        } else if entry.file_type().is_file() {
            let bytes = fs::read(entry.path()).map_err(|source| Error::io(entry.path(), source))?;
            let identical =
                incremental && fs::read(&target).ok().as_deref() == Some(bytes.as_slice());
            if !identical {
                atomic_write(&target, &bytes)?;
            }
        }
    }
    Ok(())
}

fn remove_absent_optional_files(staging: &Path, destination: &Path) -> Result<(), Error> {
    for name in OPTIONAL_ROOT_FILES {
        let generated = staging.join(name);
        let existing = destination.join(name);
        if !generated.exists() && existing.is_file() {
            fs::remove_file(&existing).map_err(|source| Error::io(&existing, source))?;
        }
    }
    Ok(())
}

fn contents_glifs(root: &Path) -> Result<BTreeSet<PathBuf>, Error> {
    let mut glifs = BTreeSet::new();
    if !root.is_dir() {
        return Ok(glifs);
    }
    for entry in WalkDir::new(root).follow_links(false) {
        let entry = entry.map_err(walkdir_error)?;
        if !entry.file_type().is_file() || entry.file_name() != OsStr::new("contents.plist") {
            continue;
        }
        let value = plist::Value::from_file(entry.path())?;
        let Some(contents) = value.as_dictionary() else {
            continue;
        };
        let layer_dir = entry.path().parent().unwrap_or(root);
        let relative_layer = layer_dir
            .strip_prefix(root)
            .map_err(|error| Error::UnsafePath(error.to_string().into()))?;
        for path in contents.values().filter_map(plist::Value::as_string) {
            let relative = relative_layer.join(path);
            validate_relative(&relative)?;
            glifs.insert(relative);
        }
    }
    Ok(glifs)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|source| Error::io(parent, source))?;
    let temporary = unique_sibling(path, "write");
    fs::write(&temporary, bytes).map_err(|source| Error::io(&temporary, source))?;
    fs::rename(&temporary, path).map_err(|source| Error::io(path, source))?;
    Ok(())
}

fn safe_join(root: &Path, relative: &Path) -> Result<PathBuf, Error> {
    validate_relative(relative)?;
    Ok(root.join(relative))
}

fn validate_relative(path: &Path) -> Result<(), Error> {
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(Error::UnsafePath(path.to_path_buf()));
    }
    Ok(())
}

fn unique_sibling(path: &Path, purpose: &str) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path.file_name().and_then(OsStr::to_str).unwrap_or("font");
    let serial = UNIQUE.fetch_add(1, Ordering::Relaxed);
    parent.join(format!(".{name}.{purpose}-{}-{serial}", std::process::id()))
}

fn walkdir_error(error: walkdir::Error) -> Error {
    let path = error.path().map(Path::to_path_buf).unwrap_or_default();
    let source = error
        .into_io_error()
        .unwrap_or_else(|| std::io::Error::other("directory traversal failed"));
    Error::io(path, source)
}

struct StagingGuard(PathBuf);

impl Drop for StagingGuard {
    fn drop(&mut self) {
        if self.0.is_dir() {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}
