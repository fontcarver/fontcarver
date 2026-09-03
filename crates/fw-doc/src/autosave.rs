//! Periodic crash-recovery snapshots for dirty documents.

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Condvar, Mutex},
    thread::{self, JoinHandle},
    time::{Duration, SystemTime},
};

use sha1::{Digest, Sha1};

use crate::{fwf, DocId, Error, Session};

/// Timing and storage configuration for the autosave worker.
#[derive(Clone, Debug)]
pub struct AutosaveConfig {
    /// Delay between autosave passes.
    pub interval: Duration,
    /// Directory containing recovery files.
    pub dir: PathBuf,
}

impl Default for AutosaveConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(60),
            dir: default_autosave_dir(),
        }
    }
}

/// Metadata for one recovery snapshot found on disk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoverableEntry {
    /// Original document path, absent for documents that had never been saved.
    pub doc_path: Option<PathBuf>,
    /// Native recovery document path.
    pub autosave_path: PathBuf,
    /// Filesystem modification time of the recovery snapshot.
    pub saved_at: SystemTime,
}

/// Running autosave worker; dropping it requests a stop and joins the thread.
pub struct AutosaveHandle {
    stop: Arc<(Mutex<bool>, Condvar)>,
    thread: Option<JoinHandle<()>>,
}

impl AutosaveHandle {
    /// Stops the worker promptly and waits for its thread to exit.
    pub fn stop(mut self) {
        self.request_stop();
        self.join();
    }

    fn request_stop(&self) {
        let (stopped, wake) = &*self.stop;
        match stopped.lock() {
            Ok(mut stopped) => {
                *stopped = true;
                wake.notify_one();
            }
            Err(poisoned) => {
                *poisoned.into_inner() = true;
                wake.notify_one();
            }
        }
    }

    fn join(&mut self) {
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for AutosaveHandle {
    fn drop(&mut self) {
        self.request_stop();
        self.join();
    }
}

/// Starts a background autosave worker for a shared session.
pub fn spawn(session: Arc<Session>, config: AutosaveConfig) -> AutosaveHandle {
    let stop = Arc::new((Mutex::new(false), Condvar::new()));
    let thread_stop = Arc::clone(&stop);
    let thread = thread::Builder::new()
        .name("fontworkshop-autosave".to_owned())
        .spawn(move || run_worker(&session, &config, &thread_stop));
    let thread = match thread {
        Ok(thread) => Some(thread),
        Err(error) => {
            tracing::error!(%error, "could not start autosave worker");
            None
        }
    };
    AutosaveHandle { stop, thread }
}

/// Lists autosaves that are newer than their original document, if known.
pub fn list_recoverable(dir: &Path) -> Vec<RecoverableEntry> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) => {
            tracing::debug!(%error, path = %dir.display(), "could not scan autosave directory");
            return Vec::new();
        }
    };
    let mut recoverable = Vec::new();
    for entry in entries.flatten() {
        let autosave_path = entry.path();
        if !is_autosave_path(&autosave_path) {
            continue;
        }
        let Ok(saved_at) = entry.metadata().and_then(|metadata| metadata.modified()) else {
            continue;
        };
        let Ok(doc_path) = fwf::read_document_path(&autosave_path) else {
            continue;
        };
        if original_is_newer(doc_path.as_deref(), saved_at) {
            continue;
        }
        recoverable.push(RecoverableEntry {
            doc_path,
            autosave_path,
            saved_at,
        });
    }
    recoverable.sort_by(|left, right| right.saved_at.cmp(&left.saved_at));
    recoverable
}

fn run_worker(session: &Session, config: &AutosaveConfig, stop: &Arc<(Mutex<bool>, Condvar)>) {
    loop {
        let (stopped, wake) = &**stop;
        let guard = match stopped.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let result = wake.wait_timeout_while(guard, config.interval, |stopped| !*stopped);
        let stopped = match result {
            Ok((guard, _)) => *guard,
            Err(poisoned) => *poisoned.into_inner().0,
        };
        if stopped {
            break;
        }
        if let Err(error) = autosave_pass(session, &config.dir) {
            tracing::warn!(%error, "autosave pass failed");
        }
    }
}

fn autosave_pass(session: &Session, dir: &Path) -> Result<(), Error> {
    std::fs::create_dir_all(dir)?;
    for id in session.ids() {
        let snapshot = session.with_doc(id, |document| {
            if !document.dirty {
                return None;
            }
            Some((document.font.clone(), document.path.clone()))
        });
        let result = match snapshot {
            Ok(Some((font, document_path))) => {
                let path = autosave_path(dir, id, document_path.as_deref());
                fwf::write_autosave(&path, &font, document_path.as_deref())
            }
            Ok(None) => Ok(()),
            Err(error) => Err(error),
        };
        if let Err(error) = result {
            tracing::warn!(%error, doc = id.0, "could not autosave document");
        }
    }
    Ok(())
}

fn autosave_path(dir: &Path, id: DocId, path: Option<&Path>) -> PathBuf {
    let stem = path.map_or_else(
        || id.0.to_string(),
        |path| {
            let digest = Sha1::digest(path.to_string_lossy().as_bytes());
            format!("{digest:x}")
        },
    );
    dir.join(format!("{stem}.autosave.fwf"))
}

fn is_autosave_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".autosave.fwf"))
}

fn original_is_newer(path: Option<&Path>, autosave_time: SystemTime) -> bool {
    path.and_then(|path| std::fs::metadata(path).ok())
        .and_then(|metadata| metadata.modified().ok())
        .is_some_and(|modified| modified > autosave_time)
}

fn default_autosave_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(dir).join("fontworkshop/autosave");
    }
    if let Some(dir) = std::env::var_os("HOME") {
        return PathBuf::from(dir).join(".local/share/fontworkshop/autosave");
    }
    std::env::temp_dir().join("fontworkshop/autosave")
}
