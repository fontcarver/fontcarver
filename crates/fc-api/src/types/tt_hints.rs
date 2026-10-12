//! Compiled-point TrueType editing, independent of PostScript hint state.

use fc_doc::DocId;
use fc_model::{MasterId, TtMasterData};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Select a glyph's static master for instruction authoring.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct TtHintStateParams {
    /// Open document.
    pub doc: DocId,
    /// Source master.
    pub master: MasterId,
    /// Model glyph name, never a positional glyph ID.
    pub glyph: String,
}

/// Stored bytes remain available even when disassembly fails.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct TtProgramSource {
    /// Exact source bytecode.
    pub bytes: Vec<u8>,
    /// Canonical lossless assembly, if decoding succeeded.
    pub assembly: Option<String>,
    /// A decoding diagnostic, if the byte stream is malformed.
    pub error: Option<String>,
}

/// One actual compiled point; its array index is its TT instruction index.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct TtPointView {
    /// Position in unscaled font units.
    #[ts(type = "{ x: number; y: number }")]
    pub position: kurbo::Point,
    /// Whether the point is on the outline.
    pub on_curve: bool,
    /// Expanded contour index.
    pub contour: u32,
}

/// Captured source and actual compiled point identity from one document revision.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct TtHintState {
    /// Revision required for applying or previewing a draft.
    #[ts(type = "number")]
    pub generation: u64,
    /// Source master-layer identity.
    pub layer: String,
    /// Current compiled point binding.
    pub fingerprint: String,
    /// Stored program belongs to the current compiled point identity.
    pub binding_matches: bool,
    /// The stored program was explicitly authored in FontCarver.
    pub authored: bool,
    /// Source program, available independently of current binding validity.
    pub program: TtProgramSource,
    /// Source global values; missing maxima are not silently filled in.
    pub globals: TtMasterData,
    /// Global function definitions.
    pub fpgm: TtProgramSource,
    /// Global size initialization program.
    pub prep: TtProgramSource,
    /// Actual compiled unhinted outline.
    pub path: String,
    /// Actual stored point order, including off-curve controls and components.
    pub points: Vec<TtPointView>,
    /// Index of the first of four phantom points.
    pub phantom_start: u32,
    /// Nominal horizontal phantom coordinates from parent metrics.
    pub horizontal_phantoms: [f64; 2],
    /// Explicit vertical phantom coordinates, if vertical metrics exist.
    pub vertical_phantoms: Option<[f64; 2]>,
    /// A component can replace nominal parent metrics during interpretation.
    pub component_metrics: bool,
    /// Current source scale.
    pub units_per_em: u16,
}

/// Editable assembly and global parameters bound to a reviewed point identity.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct TtHintDraft {
    /// Binding returned by the compiled-point state, after reviewing the program.
    pub fingerprint: String,
    /// Glyph assembly.
    pub program: String,
    /// Global function assembly; replaces `globals.fpgm`.
    pub fpgm: String,
    /// Global preparation assembly; replaces `globals.prep`.
    pub prep: String,
    /// CVT and complete interpreter maxima; ownership is set by the command.
    pub globals: TtMasterData,
}

/// Atomically store glyph and global programs after compiling and checking points.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct TtHintApplyParams {
    /// Glyph/master/document selection.
    pub target: TtHintStateParams,
    /// Captured document generation.
    #[ts(type = "number")]
    pub expected_generation: u64,
    /// Candidate instruction source.
    pub draft: TtHintDraft,
    /// Undo label.
    pub label: String,
}

/// Execute a candidate without mutating document, history or persisted programs.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct TtHintPreviewParams {
    /// Glyph/master/document selection.
    pub target: TtHintStateParams,
    /// Captured document generation.
    #[ts(type = "number")]
    pub expected_generation: u64,
    /// Candidate instruction source.
    pub draft: TtHintDraft,
    /// One to eight pixel sizes, each in 1..=128.
    pub ppems: Vec<u16>,
    /// Enable full monochrome grid fitting instead of grayscale compatibility.
    pub mono: bool,
}

/// Interpreter output for one size, with coordinates and metrics in pixels.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct TtHintPreviewSize {
    /// Pixels per em.
    pub ppem: u16,
    /// Grid-fitted outline.
    pub hinted: String,
    /// Original scaled outline.
    pub unhinted: String,
    /// Interpreter-adjusted horizontal advance, including phantom point movement.
    pub hinted_advance: f64,
    /// Original scaled horizontal advance.
    pub unhinted_advance: f64,
}

/// Read-only preview at a captured document revision.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct TtHintPreview {
    /// Revision used for this preview.
    #[ts(type = "number")]
    pub generation: u64,
    /// Successfully interpreted samples; any interpreter error fails the request.
    pub sizes: Vec<TtHintPreviewSize>,
    /// Export diagnostics, including omitted imported programs.
    pub warnings: Vec<crate::DocWarning>,
}
