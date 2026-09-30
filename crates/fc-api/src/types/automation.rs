//! Local script discovery and explicit Python execution contracts.

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

/// One directly contained, regular script file in the scripts directory.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct AutomationScriptEntry {
    /// Filename relative to the scripts directory.
    pub name: String,
    /// Runtime inferred from the extension.
    pub language: AutomationLanguage,
    /// Source size in bytes.
    pub bytes: u32,
}

/// Snapshot returned by listing or explicitly reloading local scripts.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct AutomationScriptList {
    /// Local directory to which users can add trusted scripts.
    pub directory: String,
    /// Alphabetically sorted supported files. Listing never executes a script.
    pub scripts: Vec<AutomationScriptEntry>,
    /// Bounded diagnostics for ignored entries.
    pub warnings: Vec<String>,
}

/// Parameters for explicitly loading one local script.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct AutomationScriptReadParams {
    /// Direct filename; traversal and symlinks are rejected.
    pub name: String,
}

/// UTF-8 source loaded on explicit request, without executing it.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct AutomationScriptSource {
    /// Relative filename.
    pub name: String,
    /// Selected language.
    pub language: AutomationLanguage,
    /// Source, at most 64 KiB.
    pub source: String,
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
}
