//! Session and preference API types.

use fw_doc::DocId;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Parameters for opening a font or FontWorkshop document.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct OpenParams {
    /// Filesystem path to open.
    pub path: String,
    /// Optional face index in a font collection.
    #[serde(default)]
    pub face_index: Option<u32>,
}

/// Parameters for inspecting the faces in a font container.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct FacesParams {
    /// Filesystem path to inspect.
    pub path: String,
}

/// Parameters for creating an empty document.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct NewParams {
    /// Initial family name.
    pub family: String,
    /// Font units per em.
    pub units_per_em: u16,
}

/// Parameters for closing a document.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct CloseParams {
    /// Open document identifier.
    pub doc: DocId,
}

/// Parameters for saving a document.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct SaveParams {
    /// Open document identifier.
    pub doc: DocId,
    /// New save path, or the document's current path when absent.
    #[serde(default)]
    pub path: Option<String>,
}

/// An empty JSON object used by parameterless API methods.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct EmptyParams {}

#[derive(Deserialize)]
struct EmptyParamsObject {}

impl<'de> Deserialize<'de> for EmptyParams {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let _ = Option::<EmptyParamsObject>::deserialize(deserializer)?;
        Ok(Self {})
    }
}

/// Compact information about one interpolation master.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct MasterSummary {
    /// Persistent master identifier.
    pub id: fw_model::MasterId,
    /// Master display name.
    pub name: String,
}

/// Compact information about one variation axis.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct AxisSummary {
    /// Four-character OpenType axis tag.
    pub tag: String,
    /// Axis display name.
    pub name: String,
    /// Minimum user-space coordinate.
    pub min: f64,
    /// Default user-space coordinate.
    pub default: f64,
    /// Maximum user-space coordinate.
    pub max: f64,
    /// Whether interfaces should hide the axis.
    pub hidden: bool,
}

/// Compact state for one open document.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct DocSummary {
    /// Open document identifier.
    pub doc: DocId,
    /// Current document path.
    pub path: Option<String>,
    /// Whether the document differs from its last save.
    pub dirty: bool,
    /// Default family name.
    pub family_name: String,
    /// Default style name.
    pub style_name: String,
    /// Font units per em.
    pub units_per_em: u16,
    /// Number of glyphs.
    pub glyph_count: u32,
    /// Interpolation masters.
    pub masters: Vec<MasterSummary>,
    /// Variation axes.
    pub axes: Vec<AxisSummary>,
    /// Label of the next undo operation.
    pub undo_label: Option<String>,
    /// Label of the next redo operation.
    pub redo_label: Option<String>,
    /// Current source or native document format.
    pub format: String,
    /// Non-fatal diagnostics associated with opening the document.
    pub warnings: Vec<DocWarning>,
}

/// A non-fatal diagnostic associated with opening a document.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct DocWarning {
    /// Stable machine-readable warning identifier.
    pub code: String,
    /// Human-readable warning detail used when no translation is available.
    pub message: String,
}

/// Which editor objects the pointer snaps to.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct SnapPrefs {
    /// Snap to outline nodes and handles.
    #[serde(default = "yes")]
    pub nodes: bool,
    /// Snap to guides.
    #[serde(default = "yes")]
    pub guides: bool,
    /// Snap to metrics lines and sidebearings.
    #[serde(default = "yes")]
    pub metrics: bool,
    /// Snap to the editing grid.
    #[serde(default = "yes")]
    pub grid: bool,
    /// Snap to component ghost nodes.
    #[serde(default = "yes")]
    pub components: bool,
}

const fn yes() -> bool {
    true
}

impl Default for SnapPrefs {
    fn default() -> Self {
        Self {
            nodes: true,
            guides: true,
            metrics: true,
            grid: true,
            components: true,
        }
    }
}

/// Persisted application preferences.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Prefs {
    /// Color-theme preference.
    pub theme: String,
    /// Interface language code.
    pub language: String,
    /// Maximum number of recent paths shown by the interface.
    pub recent_limit: u32,
    /// Autosave interval in seconds; zero disables autosave.
    pub autosave_seconds: u32,
    /// Editing grid size in font units; zero disables the grid.
    #[serde(default = "default_grid_size")]
    pub grid_size: f64,
    /// Snapping targets.
    #[serde(default)]
    pub snap: SnapPrefs,
    /// Nudge distances for arrow keys, in font units: plain, shift, command.
    #[serde(default = "default_nudge")]
    pub nudge: [f64; 3],
}

const fn default_grid_size() -> f64 {
    1.0
}

const fn default_nudge() -> [f64; 3] {
    [1.0, 10.0, 100.0]
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            theme: "system".to_owned(),
            language: "en".to_owned(),
            recent_limit: 20,
            autosave_seconds: 60,
            grid_size: default_grid_size(),
            snap: SnapPrefs::default(),
            nudge: default_nudge(),
        }
    }
}

/// Parameters for replacing persisted application preferences.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct SetPrefsParams {
    /// Complete preference value to persist.
    pub prefs: Prefs,
}
