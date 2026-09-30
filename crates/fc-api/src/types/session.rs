//! Session and preference API types.

use fc_doc::DocId;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Parameters for opening a font or FontCarver document.
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
    pub id: fc_model::MasterId,
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
    /// Path of the document this one was recovered from, when it was recovered.
    ///
    /// A recovered document keeps the original path and stays dirty, so saving
    /// it overwrites the file the snapshot belongs to; this marker lets the UI
    /// say so before the user does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub recovered_from: Option<String>,
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

/// Which editor objects the pointer snaps to, and how far each one reaches.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct SnapPrefs {
    /// Snap to on-curve outline nodes.
    #[serde(default = "yes")]
    pub nodes: bool,
    /// Snap to off-curve control handles.
    #[serde(default = "yes")]
    pub handles: bool,
    /// Snap to glyph anchors.
    #[serde(default = "yes")]
    pub anchors: bool,
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
    /// Snap to active-master alignment zones.
    #[serde(default = "yes")]
    pub zones: bool,
    /// Snap to outline points in the background layer.
    #[serde(default = "yes")]
    pub background: bool,
    /// Per-target snap reach; zero on a target inherits `snap_distance`.
    #[serde(default)]
    pub distances: SnapDistances,
}

/// Per-target snap reach in CSS pixels, with zero meaning "use the default".
///
/// Professional snapping is not one radius: a designer who wants nodes to grab
/// from far away rarely wants the grid to do the same. Each target therefore
/// carries its own reach, and zero keeps it on `Prefs::snap_distance` so an
/// untouched preference file behaves exactly as one radius always did.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct SnapDistances {
    /// Reach of on-curve outline nodes.
    #[serde(default)]
    pub nodes: f64,
    /// Reach of off-curve control handles.
    #[serde(default)]
    pub handles: f64,
    /// Reach of glyph anchors.
    #[serde(default)]
    pub anchors: f64,
    /// Reach of guides.
    #[serde(default)]
    pub guides: f64,
    /// Reach of metrics lines and sidebearings.
    #[serde(default)]
    pub metrics: f64,
    /// Reach of the editing grid.
    #[serde(default)]
    pub grid: f64,
    /// Reach of component ghost nodes.
    #[serde(default)]
    pub components: f64,
    /// Reach of active-master alignment zones.
    #[serde(default)]
    pub zones: f64,
    /// Reach of background-layer outline points.
    #[serde(default)]
    pub background: f64,
}

const fn yes() -> bool {
    true
}

impl Default for SnapPrefs {
    fn default() -> Self {
        Self {
            nodes: true,
            handles: true,
            anchors: true,
            guides: true,
            metrics: true,
            grid: true,
            components: true,
            zones: true,
            background: true,
            distances: SnapDistances::default(),
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
    /// Number of subdivisions per editing grid interval, from 1 through 100.
    #[serde(default = "default_grid_subdivision")]
    pub grid_subdivision: u32,
    /// Maximum pointer snapping distance in CSS pixels, from 0 through 64.
    #[serde(default = "default_snap_distance")]
    pub snap_distance: f64,
    /// Pointer hit-test distance in CSS pixels, from 1 through 64.
    #[serde(default = "default_hit_distance")]
    pub hit_distance: f64,
    /// Control-handle marker diameter in CSS pixels, from 2 through 20.
    #[serde(default = "default_handle_size")]
    pub handle_size: f64,
    /// On-curve node marker diameter in CSS pixels, from 2 through 20.
    #[serde(default = "default_node_size")]
    pub node_size: f64,
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

const fn default_grid_subdivision() -> u32 {
    1
}

const fn default_snap_distance() -> f64 {
    6.0
}

const fn default_hit_distance() -> f64 {
    5.0
}

const fn default_handle_size() -> f64 {
    3.5
}

const fn default_node_size() -> f64 {
    5.0
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
            grid_subdivision: default_grid_subdivision(),
            snap_distance: default_snap_distance(),
            hit_distance: default_hit_distance(),
            handle_size: default_handle_size(),
            node_size: default_node_size(),
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

/// One layer a history entry changes.
///
/// A layer the font names carries that name. An unnamed one carries only its
/// role, so the interface can spell it in the reader's language instead of the
/// kernel shipping English prose. Exactly one of the two is present.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct HistoryLayer {
    /// The layer's own display name, when it has one.
    pub name: Option<String>,
    /// The layer's role: `master`, `background`, `intermediate`, `alternate`,
    /// `smart`, `backup` or `other`.
    pub role: Option<String>,
}

/// One entry of a document's history timeline.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct HistoryEntry {
    /// Position in the timeline, oldest first.
    pub index: u32,
    /// User-facing operation label.
    pub label: String,
    /// Glyphs the entry restores.
    pub glyphs: Vec<String>,
    /// The layers the entry changes.
    pub layers: Vec<HistoryLayer>,
    /// Bitmap strikes the entry restores, as `<ppem>px/<depth>bpp`.
    pub strikes: Vec<String>,
    /// Whether the entry restores font-wide data.
    pub font_level: bool,
    /// Whether undo has already passed this entry, so redo would reapply it.
    pub undone: bool,
}

/// A document's undo timeline as the History surface shows it.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct DocumentHistory {
    /// Retained entries, oldest first.
    pub entries: Vec<HistoryEntry>,
    /// How many entries undo can still restore.
    pub cursor: u32,
    /// Total retained entries, which may exceed the listed ones.
    pub len: u32,
    /// Whether older entries were dropped from this listing.
    pub truncated: bool,
}

/// Parameters for moving a document's history cursor to an exact position.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct HistoryGoParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Requested cursor: the number of entries that must remain undoable.
    pub cursor: u32,
    /// Timeline length the caller saw, so a stale request is rejected.
    pub expected_len: u32,
}

/// One named shell arrangement stored with a document.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct NamedWorkspace {
    /// User-chosen workspace name.
    pub name: String,
    /// Opaque layout record; the interface owns and validates its schema.
    #[ts(type = "unknown")]
    pub layout: serde_json::Value,
}

/// Named workspaces stored beside one document.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct DocumentWorkspaces {
    /// Stored workspaces, in their stored order.
    pub workspaces: Vec<NamedWorkspace>,
    /// Where the records live, or null when this document has no project storage.
    pub path: Option<String>,
}

/// Parameters for replacing the workspaces stored with one document.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct SetWorkspacesParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Complete replacement list.
    pub workspaces: Vec<NamedWorkspace>,
}
