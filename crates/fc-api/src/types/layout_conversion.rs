//! Validated imported-layout conversion and restoration requests.

use fc_doc::DocId;
use fc_model::{Features, MasterId};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Inspect and prove an immutable source candidate without changing the font.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct PrepareLayoutConversionParams {
    /// Open document identity.
    pub doc: DocId,
    /// Current preview master; every master is still validated.
    #[serde(default)]
    pub master: Option<MasterId>,
    /// Document generation displayed by the client.
    #[ts(type = "number")]
    pub expected_generation: u64,
    /// Effective compile-input revision displayed by the client.
    #[ts(type = "number")]
    pub expected_compile_revision: u64,
    /// Saved draft revision displayed by the client.
    #[ts(type = "number")]
    pub expected_draft_revision: u64,
    /// Attempt equivalent model spacing and anchor lifts, defaulting to true.
    #[serde(default = "default_model_lift")]
    pub prefer_model_lift: bool,
}

fn default_model_lift() -> bool {
    true
}

/// Read-only conversion proof and its short-lived, document-scoped capability.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct LayoutConversionPreview {
    /// Open document identity.
    pub doc: DocId,
    /// Preview master from the request.
    pub master: MasterId,
    /// Exact document generation covered by the proof.
    #[ts(type = "number")]
    pub generation: u64,
    /// Exact effective compile-input revision covered by the proof.
    #[ts(type = "number")]
    pub compile_revision: u64,
    /// Exact saved draft revision covered by the proof.
    #[ts(type = "number")]
    pub draft_revision: u64,
    /// Original raw-layout and glyph-mapping fingerprint.
    pub fingerprint: String,
    /// Opaque, single-use token; absent if conversion is blocked.
    pub token: Option<String>,
    /// Token lifetime from the response, in seconds; zero without a token.
    pub expires_in_seconds: u32,
    /// Exact accepted candidate; editing this response cannot alter the token.
    pub source: Option<Features>,
    /// Horizontal kerning moved wholly into model data.
    pub kerning_lifted: bool,
    /// Complete supported mark lookups moved into model anchors.
    pub anchors_lifted: bool,
    /// Nonfatal proof and optional-lift details.
    pub messages: Vec<String>,
    /// Reasons the complete conversion could not be proven.
    pub blockers: Vec<String>,
}

/// Consume one server-held exact conversion candidate.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct ApplyLayoutConversionParams {
    /// The document for which the token was issued.
    pub doc: DocId,
    /// Opaque token returned by `layout.prepare_conversion`.
    pub token: String,
}

/// Restore original layout authority and conversion-owned model data.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct RevertOriginalLayoutParams {
    /// Open document identity.
    pub doc: DocId,
    /// Document generation displayed by the client.
    #[ts(type = "number")]
    pub expected_generation: u64,
    /// Effective compile-input revision displayed by the client.
    #[ts(type = "number")]
    pub expected_compile_revision: u64,
    /// Saved draft revision displayed by the client.
    #[ts(type = "number")]
    pub expected_draft_revision: u64,
}
