//! Editable feature source, revision guards and compiler diagnostics.

use fc_doc::DocId;
use fc_model::{
    FeatureDraft, FeatureGenerationSettings, FeatureGenerator, FeatureSourceId, Features,
    FeaturesSource, GeneratedItemKind, MasterId,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::EditTokenId;

/// One source entry name an automatic recipe can own in this document.
///
/// The editor uses this to offer a per-entry automatic switch without mirroring
/// the kernel's recipe registry.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct AutomaticEntryOption {
    /// Feature tag or glyph class name.
    pub name: String,
    /// Recipe family that would own the entry.
    pub generator: FeatureGenerator,
    /// Whether the entry is a feature block or a glyph class.
    pub kind: GeneratedItemKind,
}

/// Active source and a separately persisted, possibly invalid draft.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct FeatureState {
    /// Open document identifier.
    pub doc: DocId,
    /// Document mutation generation.
    #[ts(type = "number")]
    pub generation: u64,
    /// Revision of effective compile inputs, excluding draft-only edits.
    #[ts(type = "number")]
    pub compile_revision: u64,
    /// Monotonic source/draft revision, also changed by undo and redo.
    #[ts(type = "number")]
    pub draft_revision: u64,
    /// Current layout owner; a draft never changes this field.
    pub authority: FeaturesSource,
    /// Last accepted source, used by shaping and export.
    pub active_source: Features,
    /// Last accepted generation and writer settings.
    pub active_settings: FeatureGenerationSettings,
    /// Persisted editable draft.
    pub draft: Option<FeatureDraft>,
    /// Whether applying or discarding edits is required before export.
    pub has_unapplied_changes: bool,
    /// Whether a native conversion backup exists for checked original-layout revert.
    pub original_layout_available: bool,
    /// Whether active generated source predates the current exported glyph set.
    pub generation_stale: bool,
    /// Every entry name an automatic recipe can own, for the per-entry switch.
    pub automatic_entries: Vec<AutomaticEntryOption>,
}

/// Store draft source without compiling or changing active layout.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct SetFeatureDraftParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Expected document generation.
    #[ts(type = "number")]
    pub expected_generation: u64,
    /// Expected current source/draft revision.
    #[ts(type = "number")]
    pub expected_draft_revision: u64,
    /// Source to retain, including invalid syntax.
    pub source: Features,
    /// Draft generation and writer policy.
    pub settings: FeatureGenerationSettings,
    /// Optional existing document edit gesture used for coalescing.
    #[serde(default)]
    pub token: Option<EditTokenId>,
}

/// Discard the persisted draft, preserving accepted source.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct DiscardFeatureDraftParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Expected document generation.
    #[ts(type = "number")]
    pub expected_generation: u64,
    /// Expected current source/draft revision.
    #[ts(type = "number")]
    pub expected_draft_revision: u64,
}

/// Identity of the source and font inputs to validate or activate.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct FeatureValidationParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Expected document generation.
    #[ts(type = "number")]
    pub expected_generation: u64,
    /// Expected effective compile-input revision.
    #[ts(type = "number")]
    pub expected_compile_revision: u64,
    /// Expected source/draft revision.
    #[ts(type = "number")]
    pub expected_draft_revision: u64,
}

/// Freeze one already materialized generator into ordinary draft source.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct FreezeFeatureParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Expected document generation.
    #[ts(type = "number")]
    pub expected_generation: u64,
    /// Expected effective compile-input revision.
    #[ts(type = "number")]
    pub expected_compile_revision: u64,
    /// Expected source/draft revision.
    #[ts(type = "number")]
    pub expected_draft_revision: u64,
    /// Generator whose visible snapshot becomes ordinary editable source.
    pub generator: FeatureGenerator,
    /// One feature tag or class name to hold manual, leaving the recipe enabled.
    #[serde(default)]
    pub entry: Option<String>,
}

/// Severity of a located feature diagnostic.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub enum FeatureDiagnosticSeverity {
    /// Prevents source activation.
    Error,
    /// Does not prevent activation.
    Warning,
    /// Informational compiler or generator message.
    Info,
}

/// Half-open span in a source item's UTF-16 code units.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct FeatureDiagnosticSpan {
    /// First UTF-16 code-unit offset.
    pub start: u32,
    /// Exclusive final UTF-16 code-unit offset.
    pub end: u32,
}

/// Bounded, located compiler or generation message.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct FeatureDiagnostic {
    /// Whether this message prevents activation.
    pub severity: FeatureDiagnosticSeverity,
    /// Stable machine-readable category.
    pub code: String,
    /// Human-readable explanation.
    pub message: String,
    /// Source item identity, absent for whole-font failures.
    pub source_id: Option<FeatureSourceId>,
    /// UTF-16 span within the named source item.
    pub span: Option<FeatureDiagnosticSpan>,
    /// Whether the location belongs to generated syntax.
    pub generated: bool,
    /// Master whose actual compilation produced this message.
    pub master: Option<MasterId>,
}

/// Validation result tied to every input revision, never a caller-supplied proof.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct FeatureValidationResult {
    /// Open document identifier.
    pub doc: DocId,
    /// Generation inspected; apply may return a newer state separately.
    #[ts(type = "number")]
    pub generation: u64,
    /// Effective compile-input revision inspected.
    #[ts(type = "number")]
    pub compile_revision: u64,
    /// Source/draft revision inspected.
    #[ts(type = "number")]
    pub draft_revision: u64,
    /// Actual compilation succeeded for every affected master.
    pub valid: bool,
    /// Bounded compiler and generation messages.
    pub diagnostics: Vec<FeatureDiagnostic>,
    /// More diagnostics exist than the response limit permits.
    pub truncated: bool,
    /// Masters successfully compiled.
    pub compiled_masters: Vec<MasterId>,
    /// Materialized candidate, unchanged in the document until activation.
    pub generated_source: Features,
    /// Settings and provenance associated with the candidate.
    pub generated_settings: FeatureGenerationSettings,
}

/// Atomic activation outcome; failure keeps both accepted source and draft.
#[derive(Clone, Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct FeatureApplyResult {
    /// True only after a successful checked activation, including a valid no-op.
    pub applied: bool,
    /// Validation of the exact candidate used for activation.
    pub validation: FeatureValidationResult,
    /// Current document feature state after the operation.
    pub state: FeatureState,
}
