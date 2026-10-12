//! Revision-checked editing of font-wide OpenType SVG data.

use fc_doc::DocId;
use fc_model::SvgTable;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Where a document's SVG data comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum SvgStatus {
    /// The font has no SVG data.
    Absent,
    /// Editable SVG data owns the exported table.
    Editable,
    /// SVG was explicitly deleted; exports omit it.
    Deleted,
    /// Only imported table bytes exist; they are exported under preservation rules.
    Imported,
}

/// SVG editor state from one consistent document revision.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct SvgState {
    /// Revision to use when applying edits.
    #[ts(type = "number")]
    pub generation: u64,
    /// Data ownership and availability.
    pub status: SvgStatus,
    /// Editable data when status is `Editable`.
    pub svg: Option<SvgTable>,
    /// Validated imported data for read-only rendering; ownership remains imported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub preview_svg: Option<SvgTable>,
    /// Why imported bytes cannot become editable, if they cannot.
    pub import_error: Option<String>,
    /// Scale of the editable font.
    pub units_per_em: u16,
}

/// Apply one or more SVG edits as one undoable operation.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct SvgApplyParams {
    /// Open document.
    pub doc: DocId,
    /// Revision from the SVG state.
    #[ts(type = "number")]
    pub expected_generation: u64,
    /// Replacement data, or null to delete the table.
    pub svg: Option<SvgTable>,
    /// Undo history label.
    pub label: String,
}

/// Convert imported SVG bytes into editable data as one undoable operation.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct SvgDecodeParams {
    /// Open document.
    pub doc: DocId,
    /// Revision from the SVG state.
    #[ts(type = "number")]
    pub expected_generation: u64,
    /// Undo history label.
    pub label: String,
}
