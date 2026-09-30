//! Revision-checked hint editors and non-mutating hint analysis.

use fc_doc::{command::HintCommand, DocId};
use fc_model::{Master, MasterId, PsHints, PsPrivate};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Read master parameters and optionally one glyph's layer hints atomically.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct HintStateParams {
    /// Open document.
    pub doc: DocId,
    /// Master to inspect.
    pub master: MasterId,
    /// Optional glyph selection.
    pub glyph: Option<String>,
    /// Optional layer identity; absent selects the master layer.
    pub layer: Option<String>,
}

/// Hint editing state from one consistent document revision.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct HintState {
    /// Revision to use when applying edits.
    #[ts(type = "number")]
    pub generation: u64,
    /// Full master metadata, including zones, stems and private values.
    pub master: Master,
    /// Layer hint set, if a glyph was requested.
    pub hints: Option<PsHints>,
    /// Selected layer identity.
    pub layer: Option<String>,
    /// Whether this font supports additional CID private dictionaries.
    pub is_cid: bool,
    /// Dictionary assigned to the selected layer; absent when no layer is selected.
    pub cff_private_dict_index: Option<u8>,
    /// Read-only outline for stem overlay preview.
    pub path: String,
    /// Whether automatic analysis targets this exact selected master layer.
    pub can_autohint_ps: bool,
    /// Scale of the editable font.
    pub units_per_em: u16,
}

/// Apply one or more hint edits as one undoable operation.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct HintApplyParams {
    /// Open document.
    pub doc: DocId,
    /// Revision from the hint state or preview.
    #[ts(type = "number")]
    pub expected_generation: u64,
    /// Operations to apply atomically.
    pub commands: Vec<HintCommand>,
    /// Undo history label.
    pub label: String,
}

/// Preview automatic PostScript hinting of a captured selection.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct HintAnalyzeParams {
    /// Open document.
    pub doc: DocId,
    /// Master containing the target layers.
    pub master: MasterId,
    /// Captured glyph names.
    pub glyphs: Vec<String>,
}

/// Inferred layer hints and missing private values; no mutation has occurred.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct HintAnalysis {
    /// Document revision used for analysis.
    #[ts(type = "number")]
    pub generation: u64,
    /// Glyph names and their generated hint sets.
    pub hints: Vec<(String, PsHints)>,
    /// Master private values, with existing explicit values retained.
    pub private: PsPrivate,
}
