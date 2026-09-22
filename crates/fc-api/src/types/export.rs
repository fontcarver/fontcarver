//! Export and crash-recovery API types.

use fc_doc::DocId;
use fc_model::MasterId;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A container the export pipeline can write.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub enum ExportFormat {
    /// Adobe Bitmap Distribution Format, one selected strike.
    Bdf,
    /// X11 Portable Compiled Format.
    Pcf,
    /// Linux console PSF2.
    Psf,
    /// Legacy Linux console PSF1 (256 or 512 fixed 8-pixel cells).
    Psf1,
    /// Windows raster font.
    Fnt,
    /// Windows raster font resource collection.
    Fon,
    /// OpenType bitmap font (EBDT or CBDT strikes).
    Otb,
    /// TrueType container with PNG color bitmap tables.
    Cbdt,
    /// TrueType container with Apple PNG bitmap strikes.
    Sbix,
    /// TrueType-flavored OpenType sfnt.
    #[default]
    Ttf,
    /// Static OpenType with name-keyed CFF1 outlines.
    Otf,
    /// WOFF 1 wrapping a TrueType-flavored sfnt.
    Woff,
    /// WOFF 2 wrapping a TrueType-flavored sfnt.
    Woff2,
    /// UFO 3 package for the selected master.
    Ufo,
    /// Designspace document plus one UFO per master.
    Designspace,
    /// Glyphs 3 source with all masters.
    Glyphs,
    /// Static Spline Font Database source for the selected master.
    Sfd,
    /// ASCII PostScript Type 1 for the selected master.
    Pfa,
    /// Binary PostScript Type 1 for the selected master.
    Pfb,
    /// Native FontCarver document.
    Fcf,
}

/// Options offered by the export dialog.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ExportOptions {
    /// Index of a named static instance to interpolate for compiled output.
    #[serde(default)]
    #[ts(optional)]
    pub instance: Option<usize>,
    /// Compile all selected masters as a variable font.
    #[serde(default)]
    pub variable: bool,
    /// Optional variable source subset, including the default master.
    #[serde(default)]
    #[ts(optional)]
    pub master_subset: Option<Vec<MasterId>>,
    /// Bitmap strike identity for single-strike outputs.
    #[serde(default)]
    #[ts(optional)]
    pub bitmap_strike: Option<(u16, u8)>,
    /// Output container format.
    pub format: ExportFormat,
    /// Master to export, or the default master when absent.
    #[serde(default)]
    pub master: Option<MasterId>,
    /// Request overlap removal.
    #[serde(default)]
    pub remove_overlap: bool,
    /// Preserve imported TrueType programs for untouched glyphs.
    #[serde(default = "default_true")]
    pub keep_tt_instructions: bool,
    /// Copy eligible imported raw tables into the generated font.
    #[serde(default = "default_true")]
    pub preserve_untouched_tables: bool,
    /// Use production glyph names where the model supplies them.
    #[serde(default)]
    pub production_names: bool,
    /// Request automatic hinting.
    #[serde(default)]
    pub autohint: bool,
    /// Optional allowlist of non-core table tags to keep.
    #[serde(default)]
    pub table_subset: Option<Vec<String>>,
}

const fn default_true() -> bool {
    true
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            instance: None,
            variable: false,
            master_subset: None,
            bitmap_strike: None,
            format: ExportFormat::Ttf,
            master: None,
            remove_overlap: false,
            keep_tt_instructions: true,
            preserve_untouched_tables: true,
            production_names: false,
            autohint: false,
            table_subset: None,
        }
    }
}

/// Parameters for writing one export to disk.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ExportParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Destination path; a directory for UFO and designspace exports.
    pub path: String,
    /// Export options.
    #[serde(default)]
    pub options: ExportOptions,
}

/// A non-fatal diagnostic produced while exporting.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ExportWarning {
    /// Stable machine-readable warning identifier.
    pub code: String,
    /// Human-readable warning detail.
    pub message: String,
}

/// What one finished export produced, without the encoded bytes.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ExportReport {
    /// Path that was written.
    pub path: String,
    /// Container format that was written.
    pub format: ExportFormat,
    /// Total number of bytes written.
    #[ts(type = "number")]
    pub bytes_written: u64,
    /// Non-fatal export diagnostics.
    pub warnings: Vec<ExportWarning>,
    /// Logical sfnt table tags and uncompressed byte lengths.
    pub tables: Vec<(String, u32)>,
    /// Number of glyphs treated as edited since import.
    pub touched_glyphs: u32,
    /// Total wall-clock time spent exporting.
    #[ts(type = "number")]
    pub timing_ms: u64,
}

/// One crash-recovery snapshot found on disk.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct RecoveryEntry {
    /// Path of the document the snapshot belongs to, when it had one.
    pub doc_path: Option<String>,
    /// Path of the recovery snapshot itself.
    pub autosave_path: String,
    /// Snapshot modification time in milliseconds since the Unix epoch.
    #[ts(type = "number")]
    pub saved_at_ms: u64,
}

/// Parameters naming one crash-recovery snapshot.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct RecoveryParams {
    /// Path of the recovery snapshot, as listed by `session.recover_list`.
    pub autosave_path: String,
}
