//! Cross-document glyph transfer and component repair contracts.

use fc_doc::{Change, DocId};
use fc_model::{GlyphId, MasterId};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// How source glyphs are assigned to destination glyphs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum PasteDestination {
    /// Replace matching names and add names absent from the target.
    SameName,
    /// Replace selected glyphs while keeping their names and handles.
    Selected,
    /// Append references to selected glyphs, retaining their existing content.
    Components,
}

/// Which glyph data is transferred.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum PasteContent {
    /// Complete glyph records, including all compatible layers and metadata.
    AllData,
    /// The selected source master's active layer only.
    Layer,
    /// First- and second-side kerning group membership only; no pair values.
    KerningGroups,
    /// Metrics formula strings only; widths remain unchanged.
    MetricKeys,
}

/// Treatment of component bases absent from the destination.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum MissingBasePolicy {
    /// Reject the entire transfer.
    Reject,
    /// Recursively copy missing source base glyphs.
    Copy,
    /// Create clearly marked, empty, nonexporting base glyphs.
    Placeholder,
    /// Convert source component geometry to contours before transfer.
    Decompose,
}

/// Explicit stable handles for one source/destination pair.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PasteGlyphMapping {
    /// Source-document glyph handle.
    pub source: GlyphId,
    /// Required for selected/component destinations; absent for same-name mode.
    #[serde(default)]
    pub target: Option<GlyphId>,
}

/// Preview or commit an atomic transfer between open documents.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PasteSpecialParams {
    /// Destination document.
    pub doc: DocId,
    /// Source document; the source is read without being modified.
    pub source_doc: DocId,
    /// Explicit mappings, limited to 512 pairs per operation.
    pub mappings: Vec<PasteGlyphMapping>,
    /// Name matching, selected destinations, or appended components.
    pub destination: PasteDestination,
    /// Data subset to transfer.
    pub content: PasteContent,
    /// Selected source master; defaults to the source's default master.
    #[serde(default)]
    #[ts(type = "string | null")]
    pub source_master: Option<MasterId>,
    /// Selected target master; defaults to the destination's default master.
    #[serde(default)]
    #[ts(type = "string | null")]
    pub target_master: Option<MasterId>,
    /// Explicit missing component-base policy.
    pub missing_bases: MissingBasePolicy,
    /// Validate and describe the operation without applying it.
    #[serde(default)]
    pub dry_run: bool,
    /// Destination generation returned by a previous preview.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub expected_generation: Option<u64>,
    /// Source generation returned by a previous preview.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub expected_source_generation: Option<u64>,
}

/// Validated transfer result and revisions for preview/apply workflows.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PasteSpecialResult {
    /// Atomic document change; absent for a preview or an empty operation.
    pub change: Option<Change>,
    /// Destination generation after this request.
    #[ts(type = "number")]
    pub generation: u64,
    /// Generation of the captured source snapshot.
    #[ts(type = "number")]
    pub source_generation: u64,
    /// Destination glyph names affected by requested mappings.
    pub pasted: Vec<String>,
    /// Additional copied or placeholder base glyph names.
    pub added_bases: Vec<String>,
    /// Explanations of placeholders or cross-document resource boundaries.
    pub warnings: Vec<String>,
}

/// Replace matching component bases in selected glyphs, atomically.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RelinkComponentsParams {
    /// Destination document.
    pub doc: DocId,
    /// Glyphs whose references should be repaired, limited to 100000 handles.
    pub gids: Vec<GlyphId>,
    /// Existing or missing base name to replace.
    pub from: String,
    /// Existing target base handle in this document.
    pub to: GlyphId,
    /// Restrict to this master's layers; absent means every layer.
    #[serde(default)]
    #[ts(type = "string | null")]
    pub master: Option<MasterId>,
    /// Optional guard against a changed destination document.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub expected_generation: Option<u64>,
}
