//! Thread-safe ownership of all open documents.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU32, AtomicU64, Ordering},
        Arc, RwLock,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use sha1::{Digest, Sha1};

#[cfg(debug_assertions)]
use std::cell::Cell;

use fc_model::Font;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use ts_rs::TS;

use crate::{fcf, Document, DocumentEvent, Error};

const EVENT_CAPACITY: usize = 1024;

/// Separates sessions created in the same process within the same nanosecond.
static NEXT_INSTANCE: AtomicU64 = AtomicU64::new(0);

/// A short identity for one session: process, start time and a counter.
fn new_instance_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    let mut digest = Sha1::new();
    digest.update(std::process::id().to_le_bytes());
    digest.update(nanos.to_le_bytes());
    digest.update(NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed).to_le_bytes());
    format!("{:x}", digest.finalize())[..16].to_owned()
}

/// Stable session identifier for one open document.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
pub struct DocId(pub u32);

/// Boundary used by a session to import binary font containers.
pub trait FontOpener: Send + Sync {
    /// Reads one or more faces from a binary font path.
    fn open_path(&self, path: &Path, face_index: Option<u32>) -> Result<Vec<Font>, Error>;
}

/// Production binary-font opener backed by `fc-sfnt` when that crate is enabled.
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultFontOpener;

#[cfg(feature = "sfnt")]
impl FontOpener for DefaultFontOpener {
    fn open_path(&self, path: &Path, face_index: Option<u32>) -> Result<Vec<Font>, Error> {
        let options = fc_sfnt::ReadOptions {
            font_index: face_index,
            keep_original_bytes: true,
            decode_bitmaps: true,
        };
        let outcome =
            fc_sfnt::read_path(path, &options).map_err(|error| Error::Sfnt(error.to_string()))?;
        if face_index.is_none() {
            let bytes = outcome
                .fonts
                .first()
                .and_then(|font| font.extra.source.as_ref())
                .and_then(|source| source.original_bytes.as_deref());
            if let Some(warning) =
                bytes.and_then(|bytes| fc_sfnt::collection_warning(bytes.as_slice()))
            {
                tracing::warn!(code = warning.code, message = %warning.message, path = %path.display());
            }
        }
        if !outcome.warnings.is_empty() {
            tracing::warn!(warning_count = outcome.warnings.len(), path = %path.display(), "warnings while opening binary font");
        }
        Ok(outcome.fonts)
    }
}

#[cfg(not(feature = "sfnt"))]
impl FontOpener for DefaultFontOpener {
    fn open_path(&self, _path: &Path, _face_index: Option<u32>) -> Result<Vec<Font>, Error> {
        Err(Error::SfntUnavailable)
    }
}

#[cfg(debug_assertions)]
thread_local! {
    static CALLBACK_DEPTH: Cell<u32> = const { Cell::new(0) };
}

#[cfg(debug_assertions)]
struct CallbackGuard;

#[cfg(debug_assertions)]
impl CallbackGuard {
    fn enter() -> Self {
        CALLBACK_DEPTH.with(|depth| depth.set(depth.get().saturating_add(1)));
        Self
    }
}

#[cfg(debug_assertions)]
impl Drop for CallbackGuard {
    fn drop(&mut self) {
        CALLBACK_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
    }
}

/// Process-wide, thread-safe collection of open font documents.
///
/// Callbacks passed to this session must not call back into the same session;
/// doing so deadlocks in release builds and returns [`Error::Reentrant`] in
/// debug builds.
pub struct Session {
    documents: RwLock<HashMap<DocId, Document>>,
    events: broadcast::Sender<DocumentEvent>,
    next_doc_id: AtomicU32,
    opener: Arc<dyn FontOpener>,
    instance_id: String,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    /// Creates an empty session using the production binary-font opener.
    pub fn new() -> Self {
        Self::with_opener(Arc::new(DefaultFontOpener))
    }

    /// Creates an empty session with an injected binary-font opener.
    pub fn with_opener(opener: Arc<dyn FontOpener>) -> Self {
        let (events, _) = broadcast::channel(EVENT_CAPACITY);
        Self {
            documents: RwLock::new(HashMap::new()),
            events,
            next_doc_id: AtomicU32::new(1),
            opener,
            instance_id: new_instance_id(),
        }
    }

    /// Identity of this session, unique to the process run that created it.
    ///
    /// Document ids restart at 1 in every process, so they cannot name
    /// crash-recovery snapshots on their own: the snapshots of an earlier run,
    /// which nobody has recovered yet, would be overwritten. Combining this
    /// identity with the document id keeps every run's snapshots separate.
    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }

    /// Opens a native file/directory project or imports a supported binary font.
    pub fn open_path(&self, path: &Path, face_index: Option<u32>) -> Result<DocId, Error> {
        if is_fcf_path(path) {
            let font = fcf::read(path)?;
            return Ok(self.open_font(font, Some(path.to_path_buf())));
        }
        if !is_binary_font_path(path) {
            return Err(Error::UnsupportedFormat(path.to_path_buf()));
        }
        let mut fonts = self.opener.open_path(path, face_index)?;
        if fonts.is_empty() {
            return Err(Error::Internal(format!(
                "font reader returned no faces for `{}`",
                path.display()
            )));
        }
        Ok(self.open_font(fonts.remove(0), Some(path.to_path_buf())))
    }

    /// Adds an already decoded font to the session.
    pub fn open_font(&self, font: Font, provenance: Option<PathBuf>) -> DocId {
        let id = DocId(self.next_doc_id.fetch_add(1, Ordering::Relaxed));
        let document = Document::attached(id, font, provenance, self.events.clone());
        match self.documents.write() {
            Ok(mut documents) => {
                documents.insert(id, document);
            }
            Err(poisoned) => {
                poisoned.into_inner().insert(id, document);
            }
        }
        id
    }

    /// Creates and opens a new empty static font.
    pub fn new_document(&self, family: &str, units_per_em: u16) -> DocId {
        self.open_font(Font::new_empty(family, units_per_em), None)
    }

    /// Closes an open document.
    pub fn close(&self, id: DocId) -> Result<(), Error> {
        let removed = self
            .documents
            .write()
            .map_err(|_| Error::LockPoisoned)?
            .remove(&id);
        removed.map(|_| ()).ok_or(Error::DocumentNotFound(id.0))
    }

    /// Returns all open identifiers in ascending order.
    pub fn ids(&self) -> Vec<DocId> {
        let mut ids: Vec<_> = match self.documents.read() {
            Ok(documents) => documents.keys().copied().collect(),
            Err(poisoned) => poisoned.into_inner().keys().copied().collect(),
        };
        ids.sort_by_key(|id| id.0);
        ids
    }

    /// Runs a read-only closure with an open document.
    ///
    /// The callback must not call back into this session; doing so deadlocks in
    /// release builds and returns [`Error::Reentrant`] in debug builds.
    pub fn with_doc<R>(
        &self,
        id: DocId,
        callback: impl FnOnce(&Document) -> R,
    ) -> Result<R, Error> {
        reject_reentrant()?;
        let documents = self.documents.read().map_err(|_| Error::LockPoisoned)?;
        let document = documents.get(&id).ok_or(Error::DocumentNotFound(id.0))?;
        #[cfg(debug_assertions)]
        let _callback_guard = CallbackGuard::enter();
        Ok(callback(document))
    }

    /// Runs a mutable closure with an open document.
    ///
    /// The callback must not call back into this session; doing so deadlocks in
    /// release builds and returns [`Error::Reentrant`] in debug builds.
    pub fn with_doc_mut<R>(
        &self,
        id: DocId,
        callback: impl FnOnce(&mut Document) -> R,
    ) -> Result<R, Error> {
        reject_reentrant()?;
        let mut documents = self.documents.write().map_err(|_| Error::LockPoisoned)?;
        let document = documents
            .get_mut(&id)
            .ok_or(Error::DocumentNotFound(id.0))?;
        #[cfg(debug_assertions)]
        let _callback_guard = CallbackGuard::enter();
        Ok(callback(document))
    }

    /// Subscribes to document change notifications.
    pub fn subscribe(&self) -> broadcast::Receiver<DocumentEvent> {
        self.events.subscribe()
    }
}

#[cfg(debug_assertions)]
fn reject_reentrant() -> Result<(), Error> {
    CALLBACK_DEPTH.with(|depth| {
        if depth.get() == 0 {
            Ok(())
        } else {
            Err(Error::Reentrant)
        }
    })
}

#[cfg(not(debug_assertions))]
fn reject_reentrant() -> Result<(), Error> {
    Ok(())
}

fn is_fcf_path(path: &Path) -> bool {
    fcf::is_native_path(path)
}

fn is_binary_font_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .is_some_and(|extension| {
            matches!(
                extension.as_str(),
                "ttf" | "otf" | "ttc" | "otc" | "woff" | "woff2" | "otb"
            )
        })
}
