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
    /// Number of contours in the selected layer, which bounds contour indices.
    pub contour_count: u32,
    /// One read-only outline per flex hint, in `hints.flex_hints` order, so the
    /// editor can show which curve pair each hint binds to. A hint whose binding
    /// no longer resolves contributes an empty string.
    pub flex_paths: Vec<String>,
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
    /// Proposed values for each additional CID private dictionary the selection
    /// uses, by one-based index. Dictionaries no selected glyph uses are absent.
    pub cid_private: Vec<(u8, PsPrivate)>,
}

/// Query the external hinter's state, and optionally check a set of options.
#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
pub struct TtAutohinterParams {
    /// Options to validate; the default record is always valid.
    #[serde(default)]
    pub options: TtAutohintOptions,
}

/// Reported state of the optional external TrueType hinter.
///
/// `installed` is false when no `ttfautohint` executable is on `PATH`; the UI
/// uses this to disclose before an export that automatic TrueType hinting
/// cannot run, instead of reporting it only afterwards.
#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
pub struct TtAutohinterStatus {
    /// Whether an executable was found on `PATH`.
    pub installed: bool,
    /// Absolute path of the executable, empty when none was found.
    pub path: String,
    /// First line of the tool's `--version` output, empty when unavailable.
    pub version: String,
    /// Why the supplied options would be refused, empty when they are valid.
    ///
    /// The kernel owns the option rules, so the editor shows this instead of
    /// repeating them, and an invalid value is named before an export runs.
    pub problem: String,
}

/// Validated subset of `ttfautohint` options carried by an export request.
///
/// Every field mirrors one documented command-line option. Values are validated
/// in the kernel before any process starts; an invalid value fails the export
/// request instead of being silently dropped.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct TtAutohintOptions {
    /// Smallest hinted ppem (`--hinting-range-min`).
    #[serde(default)]
    #[ts(optional)]
    pub hinting_range_min: Option<u16>,
    /// Largest hinted ppem (`--hinting-range-max`).
    #[serde(default)]
    #[ts(optional)]
    pub hinting_range_max: Option<u16>,
    /// Largest ppem hinted at all; zero removes the limit (`--hinting-limit`).
    #[serde(default)]
    #[ts(optional)]
    pub hinting_limit: Option<u16>,
    /// Stem width for scripts without blue zones (`--fallback-stem-width`).
    #[serde(default)]
    #[ts(optional)]
    pub fallback_stem_width: Option<u16>,
    /// Ppem limit for x-height rounding up; zero disables (`--increase-x-height`).
    #[serde(default)]
    #[ts(optional)]
    pub increase_x_height: Option<u16>,
    /// Ppem values and ranges, or `-` for all (`--x-height-snapping-exceptions`).
    #[serde(default)]
    pub x_height_snapping_exceptions: String,
    /// Four-letter script tag for glyphs without a script (`--default-script`).
    #[serde(default)]
    pub default_script: String,
    /// Four-letter script tag for unassigned glyphs (`--fallback-script`).
    #[serde(default)]
    pub fallback_script: String,
    /// Any of `g`, `G` and `D` (`--strong-stem-width`).
    #[serde(default)]
    pub strong_stem_width: String,
    /// Run the subglyph adjustment pass (`--adjust-subglyphs`).
    #[serde(default)]
    pub adjust_subglyphs: bool,
    /// Hint composite glyphs as a whole (`--composites`).
    #[serde(default)]
    pub composites: bool,
    /// Remove all hints instead of generating new ones (`--dehint`).
    #[serde(default)]
    pub dehint: bool,
    /// Accept a font without standard characters (`--symbol`).
    #[serde(default)]
    pub symbol: bool,
    /// Record the tool's parameters in a `TTFA` table (`--ttfa-table`).
    #[serde(default)]
    pub ttfa_table: bool,
    /// Add blue zones for Windows vertical clipping (`--windows-compatibility`).
    #[serde(default)]
    pub windows_compatibility: bool,
    /// How much provenance the tool records in the `name` table.
    #[serde(default)]
    pub info: TtAutohintInfo,
}

/// How much provenance the external hinter records in the output `name` table.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum TtAutohintInfo {
    /// Add nothing (`--no-info`).
    #[default]
    None,
    /// Let the tool add its short info string.
    Short,
    /// Record the full option list (`--detailed-info`).
    Detailed,
}

impl TtAutohintOptions {
    /// Converts to the kernel options after validating every value.
    pub fn resolve(&self) -> Result<fc_hint::external::AutohintOptions, crate::ApiError> {
        let options = fc_hint::external::AutohintOptions {
            hinting_range_min: self.hinting_range_min,
            hinting_range_max: self.hinting_range_max,
            hinting_limit: self.hinting_limit,
            fallback_stem_width: self.fallback_stem_width,
            increase_x_height: self.increase_x_height,
            x_height_snapping_exceptions: self.x_height_snapping_exceptions.clone(),
            default_script: self.default_script.clone(),
            fallback_script: self.fallback_script.clone(),
            strong_stem_width: self.strong_stem_width.clone(),
            adjust_subglyphs: self.adjust_subglyphs,
            composites: self.composites,
            dehint: self.dehint,
            symbol: self.symbol,
            ttfa_table: self.ttfa_table,
            windows_compatibility: self.windows_compatibility,
            info: match self.info {
                TtAutohintInfo::None => fc_hint::external::AutohintInfo::None,
                TtAutohintInfo::Short => fc_hint::external::AutohintInfo::Short,
                TtAutohintInfo::Detailed => fc_hint::external::AutohintInfo::Detailed,
            },
        };
        options
            .validate()
            .map_err(|error| crate::ApiError::invalid_params(error.to_string()))?;
        Ok(options)
    }
}
