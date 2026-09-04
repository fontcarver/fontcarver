//! Single-entry API dispatch.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock,
    },
    thread,
    time::Duration,
};

use fw_doc::{
    autosave::{spawn as spawn_autosave, AutosaveConfig, AutosaveHandle},
    DocId, Session,
};
use fw_layout::Shaper;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use tokio::sync::broadcast::{self, error::RecvError};

use crate::{error::ApiError, handlers, DocWarning, Event, ProgressEvent, SidebarCounts};

const EVENT_CAPACITY: usize = 1024;

/// Stage name of the terminal event emitted when an operation fails.
pub const FAILED_STAGE: &str = "failed";

/// Method name, parameter TypeScript type, and result TypeScript type.
pub const METHODS: &[(&str, &str, &str)] = &[
    ("session.open", "OpenParams", "DocSummary"),
    ("session.faces", "FacesParams", "Array<string>"),
    ("session.new", "NewParams", "DocSummary"),
    ("session.close", "CloseParams", "null"),
    ("session.list", "EmptyParams", "Array<DocSummary>"),
    ("session.save", "SaveParams", "DocSummary"),
    ("session.recent", "EmptyParams", "Array<string>"),
    ("doc.summary", "DocParams", "DocSummary"),
    ("doc.font_info", "DocParams", "FontInfoView"),
    ("doc.sidebar", "DocParams", "SidebarCounts"),
    ("glyphs.list", "GlyphListParams", "GlyphListResult"),
    ("glyphs.index_of", "GlyphIndexParams", "number | null"),
    ("glyphs.cell", "GlyphCellParams", "GlyphCell"),
    ("glyphs.detail", "GlyphDetailParams", "GlyphDetail"),
    ("glyphs.search", "GlyphSearchParams", "Array<GlyphCell>"),
    ("doc.apply", "ApplyParams", "Change"),
    ("doc.edit_begin", "EditBeginParams", "EditTokenId"),
    ("doc.edit_apply", "EditApplyParams", "Change"),
    ("doc.edit_end", "EditEndParams", "null"),
    ("doc.undo", "DocParams", "Change | null"),
    ("doc.redo", "DocParams", "Change | null"),
    ("prefs.get", "EmptyParams", "Prefs"),
    ("prefs.set", "SetPrefsParams", "Prefs"),
    ("glyphs.outline", "LayerOutlineParams", "LayerOutline"),
    (
        "glyphs.flattened_components",
        "LayerOutlineParams",
        "FlattenedComponents",
    ),
    ("glyphs.add", "AddGlyphsParams", "AddGlyphsResult"),
    ("glyphs.ids", "GlyphIdsParams", "Array<GlyphId>"),
    (
        "doc.metrics_resolve",
        "MetricsResolveParams",
        "ResolvedMetrics",
    ),
    ("text.shape", "ShapeParams", "ShapedRun"),
    ("session.export", "ExportParams", "ExportReport"),
    (
        "session.recover_list",
        "EmptyParams",
        "Array<RecoveryEntry>",
    ),
    ("session.recover", "RecoveryParams", "DocSummary"),
    ("session.discard_recovery", "RecoveryParams", "null"),
];

/// Thread-safe typed facade over an editable document session.
pub struct Api {
    pub(crate) session: Arc<Session>,
    pub(crate) sidebar_cache: std::sync::Mutex<HashMap<DocId, (u64, SidebarCounts)>>,
    pub(crate) warnings: std::sync::Mutex<HashMap<DocId, Vec<DocWarning>>>,
    pub(crate) recovered: std::sync::Mutex<HashMap<DocId, String>>,
    pub(crate) config_lock: std::sync::Mutex<()>,
    pub(crate) shaper: Shaper,
    config_dir: OnceLock<PathBuf>,
    events: broadcast::Sender<Event>,
    event_bridge: Mutex<bool>,
    shutdown: Arc<AtomicBool>,
    autosave: OnceLock<AutosaveHandle>,
}

impl Drop for Api {
    fn drop(&mut self) {
        // The forwarder holds only the session and the sender, so it would
        // outlive this facade and republish into a channel nobody reads.
        self.shutdown.store(true, Ordering::Release);
    }
}

impl Api {
    /// Creates an API facade for `session`.
    pub fn new(session: Arc<Session>) -> Self {
        let (events, _) = broadcast::channel(EVENT_CAPACITY);
        Self {
            session,
            sidebar_cache: std::sync::Mutex::new(HashMap::new()),
            warnings: std::sync::Mutex::new(HashMap::new()),
            recovered: std::sync::Mutex::new(HashMap::new()),
            config_lock: std::sync::Mutex::new(()),
            shaper: Shaper::new(),
            config_dir: OnceLock::new(),
            events,
            event_bridge: Mutex::new(false),
            shutdown: Arc::new(AtomicBool::new(false)),
            autosave: OnceLock::new(),
        }
    }

    /// Creates an API facade with a fresh session and an explicit configuration directory.
    pub fn with_config_dir(dir: impl Into<PathBuf>) -> Self {
        let api = Self::new(Arc::new(Session::new()));
        let _ = api.config_dir.set(dir.into());
        api
    }

    /// Configures the directory used for preferences and recent files.
    ///
    /// This is idempotent for the same path and must be called before serving
    /// requests. An API facade cannot be redirected after a path is installed.
    pub fn configure_config_dir(&self, dir: impl Into<PathBuf>) -> Result<(), ApiError> {
        let dir = dir.into();
        if self.config_dir.get().is_some_and(|current| current == &dir) {
            return Ok(());
        }
        self.config_dir
            .set(dir)
            .map_err(|_| ApiError::internal("API configuration directory is already set"))
    }

    pub(crate) fn config_dir(&self) -> Option<&Path> {
        self.config_dir.get().map(PathBuf::as_path)
    }

    /// Subscribes to document change and progress events.
    ///
    /// The first subscription starts a forwarding thread that republishes the
    /// session's document events on this stream; it stops when this facade is
    /// dropped or the session closes. A subscription made after a failed thread
    /// spawn tries again, so a transient failure does not silently leave the
    /// stream carrying progress events only.
    ///
    /// Document and progress events are not ordered against each other; see
    /// [`Event`].
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.start_event_bridge();
        self.events.subscribe()
    }

    /// Starts periodic crash-recovery snapshots for every open document.
    ///
    /// The interval comes from [`crate::Prefs::autosave_seconds`]; a zero
    /// interval leaves autosave off. Calling this more than once is a no-op.
    ///
    /// # Errors
    ///
    /// Returns an error when the recovery directory cannot be determined.
    pub fn start_autosave(&self) -> Result<(), ApiError> {
        if self.autosave.get().is_some() {
            return Ok(());
        }
        let seconds = handlers::prefs::get(self, crate::EmptyParams {})?.autosave_seconds;
        if seconds == 0 {
            return Ok(());
        }
        let config = AutosaveConfig {
            interval: Duration::from_secs(u64::from(seconds)),
            dir: self.autosave_dir()?,
        };
        let handle = spawn_autosave(Arc::clone(&self.session), config);
        let _ = self.autosave.set(handle);
        Ok(())
    }

    /// Returns the directory holding crash-recovery snapshots.
    pub(crate) fn autosave_dir(&self) -> Result<PathBuf, ApiError> {
        Ok(handlers::prefs::config_dir(self)?.join("autosave"))
    }

    pub(crate) fn emit_progress(&self, doc: DocId, stage: &str, percent: u8) {
        self.emit(doc, stage, percent, None);
    }

    /// Publishes the terminal event of an operation that failed.
    ///
    /// Every long-running operation ends in either `write 100` or `failed 100`,
    /// so a progress bar always terminates.
    pub(crate) fn emit_failure(&self, doc: DocId, message: impl Into<String>) {
        self.emit(doc, FAILED_STAGE, 100, Some(message.into()));
    }

    fn emit(&self, doc: DocId, stage: &str, percent: u8, message: Option<String>) {
        // An error here only means nobody is listening.
        let _ = self.events.send(Event::Progress(ProgressEvent {
            doc,
            stage: stage.to_owned(),
            percent,
            message,
        }));
    }

    fn start_event_bridge(&self) {
        let mut started = self
            .event_bridge
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *started = start_once(*started, || self.spawn_event_bridge());
    }

    fn spawn_event_bridge(&self) -> bool {
        let mut source = self.session.subscribe();
        let sink = self.events.clone();
        let shutdown = Arc::clone(&self.shutdown);
        let spawned = thread::Builder::new()
            .name("fontworkshop-events".to_owned())
            .spawn(move || loop {
                if shutdown.load(Ordering::Acquire) {
                    break;
                }
                match source.blocking_recv() {
                    Ok(event) => {
                        if shutdown.load(Ordering::Acquire) {
                            break;
                        }
                        // An error here only means nobody is listening yet.
                        let _ = sink.send(Event::Document(event));
                    }
                    Err(RecvError::Lagged(skipped)) => {
                        tracing::warn!(skipped, "API event forwarder lagged");
                    }
                    Err(RecvError::Closed) => break,
                }
            });
        match spawned {
            Ok(_handle) => true,
            Err(error) => {
                tracing::error!(%error, "could not start the API event forwarder");
                false
            }
        }
    }

    fn invoke<P, R>(
        &self,
        params: Value,
        handler: fn(&Self, P) -> Result<R, ApiError>,
    ) -> Result<Value, ApiError>
    where
        P: DeserializeOwned,
        R: Serialize,
    {
        let params = serde_json::from_value(params)
            .map_err(|error| ApiError::invalid_params(error.to_string()))?;
        let result = handler(self, params)?;
        serde_json::to_value(result).map_err(|error| ApiError::internal(error.to_string()))
    }
}

/// Runs `spawn` unless the thread it starts is already running.
///
/// Returns whether a forwarder is running afterwards, so a failed spawn is
/// retried by the next caller instead of being remembered as started.
fn start_once(started: bool, spawn: impl FnOnce() -> bool) -> bool {
    started || spawn()
}

macro_rules! define_dispatch {
    ($( $name:literal => $handler:path ),+ $(,)?) => {
        #[cfg(test)]
        pub(crate) const HANDLED_METHODS: &[&str] = &[$($name),+];

        impl Api {
            /// Calls one typed API method through the shared JSON dispatch boundary.
            pub fn call(&self, method: &str, params: Value) -> Result<Value, ApiError> {
                match method {
                    $($name => self.invoke(params, $handler),)+
                    _ => Err(ApiError::not_found(format!("unknown API method `{method}`"))),
                }
            }
        }
    };
}

define_dispatch! {
    "session.open" => handlers::session::open,
    "session.faces" => handlers::session::faces,
    "session.new" => handlers::session::new_document,
    "session.close" => handlers::session::close,
    "session.list" => handlers::session::list,
    "session.save" => handlers::session::save,
    "session.recent" => handlers::session::recent,
    "doc.summary" => handlers::doc::summary,
    "doc.font_info" => handlers::doc::font_info,
    "doc.sidebar" => handlers::doc::sidebar,
    "glyphs.list" => handlers::glyphs::list,
    "glyphs.index_of" => handlers::glyphs::index_of,
    "glyphs.cell" => handlers::glyphs::cell,
    "glyphs.detail" => handlers::glyphs::detail,
    "glyphs.search" => handlers::glyphs::search,
    "doc.apply" => handlers::doc::apply,
    "doc.edit_begin" => handlers::doc::edit_begin,
    "doc.edit_apply" => handlers::doc::edit_apply,
    "doc.edit_end" => handlers::doc::edit_end,
    "doc.undo" => handlers::doc::undo,
    "doc.redo" => handlers::doc::redo,
    "prefs.get" => handlers::prefs::get,
    "prefs.set" => handlers::prefs::set,
    "glyphs.outline" => handlers::editing::outline,
    "glyphs.flattened_components" => handlers::editing::flattened_components,
    "glyphs.add" => handlers::editing::add,
    "glyphs.ids" => handlers::editing::ids,
    "doc.metrics_resolve" => handlers::editing::metrics_resolve,
    "text.shape" => handlers::shaping::shape,
    "session.export" => handlers::export::export,
    "session.recover_list" => handlers::session::recover_list,
    "session.recover" => handlers::session::recover,
    "session.discard_recovery" => handlers::session::discard_recovery,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{HANDLED_METHODS, METHODS};

    #[test]
    fn dispatch_and_typescript_method_tables_match() {
        let handled: BTreeSet<_> = HANDLED_METHODS.iter().copied().collect();
        let typed: BTreeSet<_> = METHODS.iter().map(|entry| entry.0).collect();

        assert_eq!(
            handled.len(),
            HANDLED_METHODS.len(),
            "duplicate dispatch method"
        );
        assert_eq!(typed.len(), METHODS.len(), "duplicate typed method");
        assert_eq!(handled, typed);
    }

    #[test]
    fn a_failed_forwarder_spawn_is_retried_by_the_next_subscription() {
        let mut attempts = 0;
        let mut spawn = |started: bool| {
            super::start_once(started, || {
                attempts += 1;
                attempts > 1
            })
        };

        assert!(!spawn(false), "the first spawn fails");
        assert!(spawn(false), "the next subscription retries");
        assert!(spawn(true), "a running forwarder is not spawned again");
        assert_eq!(attempts, 2);
    }

    #[test]
    fn dropping_the_api_stops_the_event_forwarder() {
        let api = super::Api::new(std::sync::Arc::new(fw_doc::Session::new()));
        let shutdown = std::sync::Arc::clone(&api.shutdown);
        let _events = api.subscribe();

        drop(api);

        assert!(shutdown.load(std::sync::atomic::Ordering::Acquire));
    }

    #[test]
    fn api_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<super::Api>();
    }
}
