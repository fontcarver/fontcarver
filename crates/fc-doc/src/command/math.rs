//! Font-wide MATH authoring; every result is validated before mutation.

use crate::Error;
use fc_model::{Font, GlyphName, MathConstants, MathGlyphInfo, MathTable};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Editable MATH operations. Each one takes ownership of the output table.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub enum MathCommand {
    /// Replace the whole table, or delete it explicitly with null.
    Replace {
        /// Complete table; null removes MATH from exported fonts.
        #[ts(type = "MathTable | null")]
        math: Option<MathTable>,
    },
    /// Replace global constants and the minimum connector overlap.
    SetGlobals {
        /// Global constants; null writes a table without constants.
        #[ts(type = "MathConstants | null")]
        constants: Option<MathConstants>,
        /// Minimum overlap between adjacent assembly connectors.
        min_connector_overlap: f64,
    },
    /// Replace or remove one glyph's placement and construction record.
    SetGlyph {
        /// Persistent glyph name of the record.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Complete record; null removes it.
        #[ts(type = "MathGlyphInfo | null")]
        info: Option<MathGlyphInfo>,
    },
}

pub(super) fn apply(font: &mut Font, command: &MathCommand) -> Result<(), Error> {
    let mut math = match command {
        MathCommand::Replace { math } => math.clone(),
        MathCommand::SetGlobals {
            constants,
            min_connector_overlap,
        } => {
            let mut math = font.math.clone().unwrap_or_default();
            math.constants.clone_from(constants);
            math.min_connector_overlap = *min_connector_overlap;
            Some(math)
        }
        MathCommand::SetGlyph { glyph, info } => {
            let mut math = font.math.clone().unwrap_or_default();
            match info {
                Some(info) => {
                    math.glyphs.insert(glyph.clone(), info.clone());
                }
                None => {
                    math.glyphs.remove(glyph);
                }
            }
            Some(math)
        }
    };
    if let Some(math) = &mut math {
        let order: Vec<GlyphName> = font.glyph_order().cloned().collect();
        math.validate(&order)
            .map_err(|error| Error::InvalidCommand(error.to_string()))?;
    }
    font.math = math;
    font.math_table_owned = true;
    Ok(())
}
