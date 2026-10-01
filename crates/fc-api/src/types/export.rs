//! Export and crash-recovery API types.

use fc_doc::DocId;
use fc_model::{GlyphName, MasterId};
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
    /// Bare static Compact Font Format data without an OpenType container.
    Cff,
    /// PostScript Type 42 wrapping a static TrueType font.
    Type42,
    /// SVG font containing the selected master's outlines.
    Svg,
    /// Encapsulated PostScript outline of one explicitly selected glyph.
    Eps,
    /// Adobe font metrics for the selected master.
    Afm,
    /// Windows printer font metrics for the selected master.
    Pfm,
    /// Lossless XML containing compiled sfnt tables.
    Ttx,
    /// TrueType collection containing one or more independently compiled documents.
    Ttc,
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

impl ExportFormat {
    /// File-name extension written for this container, including the dot.
    ///
    /// The export dialog, the stored profiles and the command line all derive
    /// destination names from this one table.
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Bdf => ".bdf",
            Self::Pcf => ".pcf",
            Self::Psf | Self::Psf1 => ".psf",
            Self::Fnt => ".fnt",
            Self::Fon => ".fon",
            Self::Otb => ".otb",
            Self::Ttf | Self::Cbdt | Self::Sbix => ".ttf",
            Self::Otf => ".otf",
            Self::Woff => ".woff",
            Self::Woff2 => ".woff2",
            Self::Cff => ".cff",
            Self::Type42 => ".t42",
            Self::Svg => ".svg",
            Self::Eps => ".eps",
            Self::Afm => ".afm",
            Self::Pfm => ".pfm",
            Self::Ttx => ".ttx",
            Self::Ttc => ".ttc",
            Self::Ufo => ".ufo",
            Self::Designspace => ".designspace",
            Self::Glyphs => ".glyphs",
            Self::Sfd => ".sfd",
            Self::Pfa => ".pfa",
            Self::Pfb => ".pfb",
            Self::Fcf => ".fcf",
        }
    }
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
    /// Optional variable axis subset by four-character tag; other axes are dropped.
    #[serde(default)]
    #[ts(optional)]
    pub axis_subset: Option<Vec<String>>,
    /// Optional named-instance subset written to the variable font.
    #[serde(default)]
    #[ts(optional)]
    pub instance_subset: Option<Vec<String>>,
    /// Bitmap strike identity for single-strike outputs.
    #[serde(default)]
    #[ts(optional)]
    pub bitmap_strike: Option<(u16, u8)>,
    /// Output container format.
    pub format: ExportFormat,
    /// Master to export, or the default master when absent.
    #[serde(default)]
    pub master: Option<MasterId>,
    /// Glyph to write for a single-glyph outline output such as EPS.
    #[serde(default)]
    #[ts(optional)]
    pub glyph: Option<GlyphName>,
    /// Insert missing curve extrema in a static export copy.
    #[serde(default)]
    #[ts(as = "Option<bool>", optional)]
    pub add_extrema: bool,
    /// Round points, component offsets and metrics in a static export copy.
    #[serde(default)]
    #[ts(as = "Option<bool>", optional)]
    pub round_coordinates: bool,
    /// Expand components into contours in a static export copy.
    #[serde(default)]
    #[ts(as = "Option<bool>", optional)]
    pub decompose_components: bool,
    /// Share repeated static CFF charstrings using global subroutines.
    #[serde(default)]
    #[ts(as = "Option<bool>", optional)]
    pub subroutinize: bool,
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
    /// Options for the external TrueType hinter when automatic hinting runs.
    #[serde(default)]
    #[ts(as = "Option<crate::TtAutohintOptions>", optional)]
    pub tt_autohint: crate::TtAutohintOptions,
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
            axis_subset: None,
            instance_subset: None,
            bitmap_strike: None,
            format: ExportFormat::Ttf,
            master: None,
            glyph: None,
            add_extrema: false,
            round_coordinates: false,
            decompose_components: false,
            subroutinize: false,
            remove_overlap: false,
            keep_tt_instructions: true,
            preserve_untouched_tables: true,
            production_names: false,
            autohint: false,
            tt_autohint: crate::TtAutohintOptions::default(),
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
    /// Additional open documents to append after `doc` in a TTC collection.
    #[serde(default)]
    #[ts(as = "Option<Vec<DocId>>", optional)]
    pub collection_docs: Vec<DocId>,
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

/// A named export configuration stored in the project file.
///
/// Profiles are project data, not application state: the export dialog, the
/// instance/profile matrix and the `fontcarver` command line read the same
/// records, so a continuous-integration build reproduces what the editor does.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ExportProfile {
    /// Unique display name, compared case-insensitively after NFC normalization.
    pub name: String,
    /// Output subdirectory relative to the destination folder; may be empty.
    #[serde(default)]
    pub subfolder: String,
    /// Output base name without extension; empty derives it from the source.
    #[serde(default)]
    pub file_name: String,
    /// Complete option set applied before the destination is validated.
    #[serde(default)]
    pub options: ExportOptions,
}

/// A named variable-font export configuration: a design-space subset plus a name.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct VariableExportConfig {
    /// Unique display name, compared case-insensitively after NFC normalization.
    pub name: String,
    /// Output base name without extension; empty derives it from the source.
    #[serde(default)]
    pub file_name: String,
    /// Source masters to keep; empty keeps every master.
    #[serde(default)]
    pub masters: Vec<MasterId>,
    /// Axis tags to keep; empty keeps every axis that still varies.
    #[serde(default)]
    pub axes: Vec<String>,
    /// Named instances to write; empty writes every active named instance.
    #[serde(default)]
    pub instances: Vec<String>,
}

/// Every export configuration a document stores.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ExportProfilesResult {
    /// Stored profiles in their saved order.
    pub profiles: Vec<ExportProfile>,
    /// Stored variable-font configurations in their saved order.
    pub variable_configs: Vec<VariableExportConfig>,
}

/// Replace a document's stored export configurations as one undo entry.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct SetExportProfilesParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Complete replacement profile list.
    #[serde(default)]
    pub profiles: Vec<ExportProfile>,
    /// Complete replacement variable-font configuration list.
    #[serde(default)]
    pub variable_configs: Vec<VariableExportConfig>,
}

/// Resolve a stored configuration into the options one export would use.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ResolveExportProfileParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Stored profile name.
    pub profile: String,
    /// Stored variable-font configuration name, for a variable output.
    #[serde(default)]
    #[ts(optional)]
    pub variable_config: Option<String>,
    /// Named static instance, for a static output.
    #[serde(default)]
    #[ts(optional)]
    pub instance: Option<String>,
}

/// The options and suggested file name one stored configuration produces.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ResolvedExportProfile {
    /// Options ready for `session.export`.
    pub options: ExportOptions,
    /// Destination path relative to the chosen output folder.
    pub relative_path: String,
}
