//! Coalesced edit-session conveniences.

use crate::{Change, Command, Document, EditToken, Error};

impl Document {
    /// Applies a preview command inside an active edit token.
    ///
    /// This is an alias for [`Document::apply_in_edit`]. The existing undo
    /// coalescing keeps the earliest inverse and latest forward snapshot, so
    /// repeated previews do not create additional undo entries.
    pub fn preview_edit(&mut self, token: &EditToken, command: Command) -> Result<Change, Error> {
        self.apply_in_edit(token, command)
    }
}
