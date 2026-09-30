//! Narrow filesystem operations for native directory projects.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::Error;

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

pub(super) fn require_directory(path: &Path) -> Result<(), Error> {
    if !fs::symlink_metadata(path)?.file_type().is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "project storage must be a directory, not a symlink",
        )
        .into());
    }
    Ok(())
}

pub(super) fn require_file(path: &Path) -> Result<(), Error> {
    if !fs::symlink_metadata(path)?.file_type().is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "project entry must be a regular file, not a symlink",
        )
        .into());
    }
    Ok(())
}

pub(super) fn lock_project(path: &Path, writing: bool) -> Result<Option<File>, Error> {
    let lock_path = path.join(".fontcarver.lock");
    let file = match OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&lock_path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            require_file(&lock_path)?;
            OpenOptions::new()
                .read(true)
                .write(writing)
                .open(&lock_path)?
        }
        // A copied read-only project can be opened without creating its
        // advisory lock; no successful writer can operate in that directory.
        Err(error) if !writing && error.kind() == io::ErrorKind::PermissionDenied => {
            return Ok(None)
        }
        Err(error) => return Err(error.into()),
    };
    if writing {
        file.lock()?;
    } else {
        file.lock_shared()?;
    }
    Ok(Some(file))
}

pub(super) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "project file has no parent"))?;
    let (temporary, mut file) = loop {
        let sequence = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let temporary = parent.join(format!(".fontcarver-{}-{sequence}.tmp", std::process::id()));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(file) => break (temporary, file),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    };
    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

#[cfg(unix)]
pub(super) fn sync_dir(path: &Path) -> Result<(), Error> {
    File::open(path)?.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
pub(super) fn sync_dir(_path: &Path) -> Result<(), Error> {
    Ok(())
}
