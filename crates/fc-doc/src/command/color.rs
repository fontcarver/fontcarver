use fc_model::{ColorTable, Font};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::Error;

/// Atomic edits to font-wide COLR v0 compositions and CPAL palettes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum ColorCommand {
    /// Replace all editable data, or explicitly delete both tables.
    Replace {
        /// New validated palette and composition data.
        #[ts(type = "ColorTable | null")]
        color: Option<ColorTable>,
    },
}

pub(super) fn apply(font: &mut Font, command: &ColorCommand) -> Result<(), Error> {
    let ColorCommand::Replace { color } = command;
    if let Some(color) = color {
        let order = font.glyph_order().cloned().collect::<Vec<_>>();
        color.validate(&order).map_err(Error::InvalidCommand)?;
    }
    font.color.clone_from(color);
    font.color_tables_owned = true;
    Ok(())
}
