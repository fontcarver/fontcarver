//! Local script discovery, script transactions and explicit Python execution.

use fc_doc::DocId;
use fc_model::{GlyphId, MasterId};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Interpreter selected from a local script's extension.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum AutomationLanguage {
    /// JavaScript executed by the frontend macro worker.
    Javascript,
    /// Python executed by an installed interpreter.
    Python,
}

/// Which of the three searched script roots a folder or script belongs to.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum AutomationScriptFolderKind {
    /// The folder chosen in the console's scripting preferences.
    Configured,
    /// The `scripts` folder inside FontCarver's configuration directory.
    Preferences,
    /// The `scripts` folder beside the captured document's project.
    Project,
}

/// One searched script root and whether it currently exists.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct AutomationScriptFolder {
    /// Root this path came from.
    pub kind: AutomationScriptFolderKind,
    /// Absolute directory path.
    pub path: String,
    /// Whether the directory exists and was searched.
    pub exists: bool,
}

/// One directly contained, regular script file in a searched root.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct AutomationScriptEntry {
    /// Filename relative to its own root.
    pub name: String,
    /// Root that contains the file.
    pub folder: AutomationScriptFolderKind,
    /// Runtime inferred from the extension.
    pub language: AutomationLanguage,
    /// Source size in bytes.
    pub bytes: u32,
}

/// Parameters for listing or explicitly reloading local scripts.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct AutomationScriptListParams {
    /// Open document whose project folder is searched as well.
    #[serde(default)]
    pub doc: Option<DocId>,
}

/// Snapshot returned by listing or explicitly reloading local scripts.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct AutomationScriptList {
    /// Searched roots, in the order names resolve.
    pub folders: Vec<AutomationScriptFolder>,
    /// Sorted supported files. Listing never executes a script.
    pub scripts: Vec<AutomationScriptEntry>,
    /// Bounded diagnostics for ignored entries.
    pub warnings: Vec<String>,
    /// Digest of the searched roots' contents, for reload-on-change polling.
    pub fingerprint: String,
}

/// Parameters for explicitly loading one local script.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct AutomationScriptReadParams {
    /// Direct filename; traversal and symlinks are rejected.
    pub name: String,
    /// Root that contains the file; defaults to the preferences folder.
    #[serde(default = "preferences_folder")]
    pub folder: AutomationScriptFolderKind,
    /// Open document used to resolve the project folder.
    #[serde(default)]
    pub doc: Option<DocId>,
}

fn preferences_folder() -> AutomationScriptFolderKind {
    AutomationScriptFolderKind::Preferences
}

/// UTF-8 source loaded on explicit request, without executing it.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct AutomationScriptSource {
    /// Relative filename.
    pub name: String,
    /// Root the file was read from.
    pub folder: AutomationScriptFolderKind,
    /// Selected language.
    pub language: AutomationLanguage,
    /// Source, at most 64 KiB.
    pub source: String,
}

/// Persisted scripting preferences owned by the automation area.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct AutomationSettings {
    /// Absolute extra script folder, or null when only the defaults are searched.
    #[serde(default)]
    pub scripts_directory: Option<String>,
}

/// Replace the persisted scripting preferences.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct AutomationSettingsParams {
    /// Absolute existing directory, or null to search only the defaults.
    #[serde(default)]
    pub scripts_directory: Option<String>,
}

/// Open a script transaction so one run becomes one undo entry.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct AutomationTransactionParams {
    /// Document the script may edit.
    pub doc: DocId,
    /// History label shown for the whole run, at most 120 characters.
    pub label: String,
    /// Lifetime in milliseconds, 100–120000; defaults to 60000.
    ///
    /// The transaction closes itself when this elapses, so an abandoned run
    /// cannot hold history open.
    #[serde(default)]
    pub timeout_ms: Option<u32>,
}

/// A live script transaction. Every later change joins its single undo entry.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct AutomationTransaction {
    /// Document the transaction belongs to.
    pub doc: DocId,
    /// Token required to close it.
    #[ts(type = "number")]
    pub token: u64,
}

/// Close a script transaction, whether the run succeeded or failed.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct AutomationTransactionEndParams {
    /// Document the transaction belongs to.
    pub doc: DocId,
    /// Token returned when the transaction was opened.
    #[ts(type = "number")]
    pub token: u64,
}

/// Outcome of closing a script transaction.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct AutomationTransactionEnd {
    /// Whether this token was the live transaction that has now been closed.
    pub ended: bool,
    /// Undo label of the resulting single history entry, when one was recorded.
    pub undo_label: Option<String>,
}

/// Explicit trusted-code invocation in a separate Python process.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct AutomationPythonParams {
    /// Python source, at most 64 KiB. This is trusted local code, not sandboxed.
    pub source: String,
    /// Captured active document, exposed as `fontcarver.context['doc']`.
    #[serde(default)]
    pub doc: Option<DocId>,
    /// Captured active interpolation master.
    #[serde(default)]
    pub master: Option<MasterId>,
    /// Captured active glyph.
    #[serde(default)]
    pub glyph: Option<GlyphId>,
    /// Deadline in milliseconds, 100–30000; defaults to 10000.
    #[serde(default)]
    pub timeout_ms: Option<u32>,
    /// History label for the run's single undo entry, when a document is captured.
    #[serde(default)]
    pub label: Option<String>,
}

/// Collected bounded output from one explicit Python invocation.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct AutomationPythonResult {
    /// Printed output lines, at most 64 KiB combined.
    pub output: Vec<String>,
    /// Runtime, protocol, limit or deadline failure; null indicates success.
    pub error: Option<String>,
    /// Elapsed execution time in milliseconds.
    pub elapsed_ms: u32,
    /// Number of RPC requests received, including rejected requests.
    pub rpc_calls: u32,
    /// Undo label of the run's single history entry, when it changed the document.
    pub undo_label: Option<String>,
}
