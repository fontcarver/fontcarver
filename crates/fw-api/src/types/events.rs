//! Event-stream API types.

use fw_doc::DocId;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub use fw_doc::DocumentEvent;

/// Progress of one long-running operation on a document.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ProgressEvent {
    /// Document the operation belongs to.
    pub doc: DocId,
    /// Stage identifier, such as `compile`, `preserve`, or `write`.
    pub stage: String,
    /// Completion of the whole operation from 0 to 100.
    pub percent: u8,
}

/// Everything the kernel pushes to subscribed frontends.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind")]
#[ts(export)]
pub enum Event {
    /// A committed document state change.
    Document(DocumentEvent),
    /// Progress of a long-running operation.
    Progress(ProgressEvent),
}

impl From<DocumentEvent> for Event {
    fn from(event: DocumentEvent) -> Self {
        Self::Document(event)
    }
}

impl From<ProgressEvent> for Event {
    fn from(event: ProgressEvent) -> Self {
        Self::Progress(event)
    }
}
