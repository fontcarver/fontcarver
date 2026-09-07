//! Event-stream API types.

use fc_doc::DocId;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub use fc_doc::DocumentEvent;

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
    /// Failure detail, present on the terminal `failed` stage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub message: Option<String>,
}

/// Everything the kernel pushes to subscribed frontends.
///
/// Document and progress events are ordered within their own kind but not
/// against each other: progress is published directly by the operation while
/// document events cross a forwarder thread, so a progress event can be
/// delivered before a document event that happened first.
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
