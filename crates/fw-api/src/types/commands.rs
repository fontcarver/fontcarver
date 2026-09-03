//! Document-mutation API types.

use fw_doc::{Command, DocId};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Parameters for applying one labelled command.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ApplyParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Mutation to apply.
    pub command: Command,
    /// Undo label.
    pub label: String,
}

/// Parameters for starting a coalesced edit.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct EditBeginParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Undo label for the edit gesture.
    pub label: String,
}

/// Transport-safe identifier for a coalesced edit gesture.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[serde(transparent)]
#[ts(export)]
pub struct EditTokenId(#[ts(type = "number")] pub u64);

/// Parameters for applying a command within a coalesced edit.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct EditApplyParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Active edit token.
    pub token: EditTokenId,
    /// Mutation to apply.
    pub command: Command,
}

/// Parameters for ending a coalesced edit.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct EditEndParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Active edit token.
    pub token: EditTokenId,
}
