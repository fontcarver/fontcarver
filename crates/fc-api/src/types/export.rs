//! Export and crash-recovery API types.

use fc_doc::DocId;
use fc_model::MasterId;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A container the export pipeline can write.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub enum ExportFormat {
    /// TrueType-flavored OpenType sfnt.
    #[default]
    Ttf,
    /// WOFF 1 wrapping a TrueType-flavored sfnt.
    Woff,
    /// WOFF 2 wrapping a TrueType-flavored sfnt.
    Woff2,
    /// UFO 3 package for the selected master.
    Ufo,
    /// Designspace document plus one UFO per master.
    Designspace,
    /// Native FontCarver document.
    Fcf,
}

/// Options offered by the export dialog.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ExportOptions {
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
