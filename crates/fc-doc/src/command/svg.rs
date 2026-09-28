use fc_model::{Font, SvgTable};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::Error;

/// Atomic edits to font-wide SVG documents.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum SvgCommand {
    /// Replace all editable data, or explicitly delete the table.
    Replace {
        /// New validated SVG documents.
        #[ts(type = "SvgTable | null")]
        svg: Option<SvgTable>,
    },
}

pub(super) fn apply(font: &mut Font, command: &SvgCommand) -> Result<(), Error> {
    let SvgCommand::Replace { svg } = command;
    if let Some(svg) = svg {
        let order = font.glyph_order().cloned().collect::<Vec<_>>();
        svg.validate(&order).map_err(Error::InvalidCommand)?;
    }
    font.svg.clone_from(svg);
    font.svg_table_owned = true;
    Ok(())
}
