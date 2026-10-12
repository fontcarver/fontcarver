use fc_model::{BitmapGlyph, BitmapStrike, Font, GlyphName};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::Error;

/// Atomic bitmap mutations. Strike keys are stable across insertion/removal.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub enum BitmapCommand {
    /// Create or replace an entire strike, including its properties.
    SetStrike {
        /// Validated replacement strike.
        #[ts(type = "BitmapStrike")]
        strike: BitmapStrike,
    },
    /// Remove an existing strike.
    RemoveStrike {
        /// Pixels per em.
        ppem: u16,
        /// Bits per pixel.
        depth: u8,
    },
    /// Replace or remove one bitmap, without changing the owning outline.
    SetGlyph {
        /// Pixels per em.
        ppem: u16,
        /// Bits per pixel.
        depth: u8,
        /// Existing owning glyph.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Replacement, or null to remove it from this strike.
        #[ts(type = "BitmapGlyph | null")]
        value: Option<BitmapGlyph>,
    },
    /// Replace a bounded collection of glyph images as one undo transaction.
    SetGlyphs {
        /// Pixels per em.
        ppem: u16,
        /// Bits per pixel.
        depth: u8,
        /// Existing glyph names and their replacement images; other images remain unchanged.
        #[ts(type = "Record<string, BitmapGlyph>")]
        glyphs: std::collections::BTreeMap<GlyphName, BitmapGlyph>,
    },
}

pub(super) fn apply(font: &mut Font, command: &BitmapCommand) -> Result<(), Error> {
    match command {
        BitmapCommand::SetGlyphs {
            ppem,
            depth,
            glyphs,
        } => {
            if glyphs.is_empty() || glyphs.len() > 4096 {
                return Err(Error::InvalidCommand(
                    "bitmap batch must contain 1–4096 glyphs".into(),
                ));
            }
            let mut strike = font
                .bitmap_strikes
                .iter()
                .find(|s| (s.ppem, s.depth) == (*ppem, *depth))
                .cloned()
                .ok_or_else(|| Error::InvalidCommand("bitmap strike does not exist".into()))?;
            strike.glyphs.extend(
                glyphs
                    .iter()
                    .map(|(name, value)| (name.clone(), value.clone())),
            );
            apply(font, &BitmapCommand::SetStrike { strike })?;
        }
        BitmapCommand::SetStrike { strike } => {
            strike.validate()?;
            if font.bitmap_strikes.len() >= 256
                && !font
                    .bitmap_strikes
                    .iter()
                    .any(|s| (s.ppem, s.depth) == (strike.ppem, strike.depth))
            {
                return Err(Error::InvalidCommand(
                    "font exceeds 256 bitmap strikes".into(),
                ));
            }
            for name in strike.glyphs.keys() {
                if !font.glyphs.contains_key(name) {
                    return Err(Error::MissingGlyph(name.clone()));
                }
            }
            let bytes: usize = font
                .bitmap_strikes
                .iter()
                .filter(|s| (s.ppem, s.depth) != (strike.ppem, strike.depth))
                .chain(std::iter::once(strike))
                .flat_map(|s| s.glyphs.values())
                .map(|g| g.bitmap.rows.len())
                .sum();
            if bytes > fc_model::MAX_FONT_BITMAP_BYTES {
                return Err(Error::InvalidCommand("font bitmap memory limit".into()));
            }
            if let Some(current) = font
                .bitmap_strikes
                .iter_mut()
                .find(|s| (s.ppem, s.depth) == (strike.ppem, strike.depth))
            {
                *current = strike.clone();
            } else {
                font.bitmap_strikes.push(strike.clone());
            }
            font.bitmap_strikes.sort_by_key(|s| (s.ppem, s.depth));
            font.bitmap_tables_owned = true;
        }
        BitmapCommand::RemoveStrike { ppem, depth } => {
            let index = font
                .bitmap_strikes
                .iter()
                .position(|s| (s.ppem, s.depth) == (*ppem, *depth))
                .ok_or_else(|| Error::InvalidCommand("bitmap strike does not exist".into()))?;
            font.bitmap_strikes.remove(index);
            font.bitmap_tables_owned = true;
        }
        BitmapCommand::SetGlyph {
            ppem,
            depth,
            glyph,
            value,
        } => {
            if !font.glyphs.contains_key(glyph) {
                return Err(Error::MissingGlyph(glyph.clone()));
            }
            let index = font
                .bitmap_strikes
                .iter()
                .position(|s| (s.ppem, s.depth) == (*ppem, *depth))
                .ok_or_else(|| Error::InvalidCommand("bitmap strike does not exist".into()))?;
            if let Some(value) = value {
                value.bitmap.validate()?;
                if value.bitmap.depth != *depth {
                    return Err(Error::InvalidCommand(
                        "bitmap depth differs from strike".into(),
                    ));
                }
            }
            let old = match value {
                Some(value) => font.bitmap_strikes[index]
                    .glyphs
                    .insert(glyph.clone(), value.clone()),
                None => font.bitmap_strikes[index].glyphs.remove(glyph),
            };
            if let Err(error) = font.validate_bitmaps() {
                match old {
                    Some(value) => {
                        font.bitmap_strikes[index]
                            .glyphs
                            .insert(glyph.clone(), value);
                    }
                    None => {
                        font.bitmap_strikes[index].glyphs.remove(glyph);
                    }
                }
                return Err(error.into());
            }
        }
    }
    Ok(())
}
