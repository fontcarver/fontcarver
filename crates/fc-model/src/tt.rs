use serde::{Deserialize, Serialize};

/// Persistent association between a glyph program and its compiled point basis.
/// Geometry edits retain this record and the source program so stale instructions
/// can be reviewed; exporters must compare it with the current compiled basis.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TtInstructionBinding {
    /// Versioned digest of stored outline order, component dependencies and metrics.
    pub fingerprint: String,
    /// Explicitly authored programs export independently of imported-program retention.
    #[serde(default)]
    pub authored: bool,
}
