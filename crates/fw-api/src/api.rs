//! Single-entry API dispatch.

use std::{collections::HashMap, sync::Arc};

use fw_doc::{DocId, DocumentEvent, Session};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use tokio::sync::broadcast;

use crate::{error::ApiError, handlers, DocWarning, SidebarCounts};

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
];

/// Thread-safe typed facade over an editable document session.
pub struct Api {
    pub(crate) session: Arc<Session>,
    pub(crate) sidebar_cache: std::sync::Mutex<HashMap<DocId, (u64, SidebarCounts)>>,
    pub(crate) warnings: std::sync::Mutex<HashMap<DocId, Vec<DocWarning>>>,
    pub(crate) config_lock: std::sync::Mutex<()>,
}

impl Api {
    /// Creates an API facade for `session`.
    pub fn new(session: Arc<Session>) -> Self {
        Self {
            session,
            sidebar_cache: std::sync::Mutex::new(HashMap::new()),
            warnings: std::sync::Mutex::new(HashMap::new()),
            config_lock: std::sync::Mutex::new(()),
        }
    }

    /// Subscribes to document change events from the underlying session.
    pub fn subscribe(&self) -> broadcast::Receiver<DocumentEvent> {
        self.session.subscribe()
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
    fn api_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<super::Api>();
    }
}
