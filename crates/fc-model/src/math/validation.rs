use super::{MathConstruction, MathTable, MathValue};
use crate::{Error, GlyphName};
use std::collections::HashSet;

/// Maximum decoded MATH records and pixel corrections in a document.
pub const MAX_MATH_ITEMS: usize = 1_000_000;

fn invalid(message: impl Into<String>) -> Error {
    Error::InvalidMath(message.into())
}

fn units(value: f64, unsigned: bool) -> Result<(), Error> {
    let range = if unsigned {
        0.0..=65535.0
    } else {
        -32768.0..=32767.0
    };
    if !value.is_finite() || !range.contains(&value.round()) {
        return Err(invalid("font-unit value exceeds its OpenType range"));
    }
    Ok(())
}

fn count(count: usize) -> Result<(), Error> {
    if count > usize::from(u16::MAX) {
        return Err(invalid("array exceeds 65535 records"));
    }
    Ok(())
}

struct Check<'a> {
    names: HashSet<&'a GlyphName>,
    remaining: usize,
}

impl Check<'_> {
    fn take(&mut self, amount: usize) -> Result<(), Error> {
        self.remaining = self
            .remaining
            .checked_sub(amount)
            .ok_or_else(|| invalid("decoded item limit exceeded"))?;
        Ok(())
    }
    fn name(&mut self, name: &GlyphName) -> Result<(), Error> {
        self.take(1)?;
        if !self.names.contains(name) {
            return Err(invalid(format!("missing glyph `{name}`")));
        }
        Ok(())
    }
    fn value(&mut self, value: &MathValue) -> Result<(), Error> {
        self.take(1)?;
        units(value.value, false)?;
        if let Some(device) = &value.device {
            self.take(device.deltas.len())?;
            if device.deltas.is_empty()
                || usize::from(device.start_size) + device.deltas.len() > 65536
            {
                return Err(invalid(
                    "device correction range is empty or exceeds PPEM 65535",
                ));
            }
        }
        Ok(())
    }
    fn construction(&mut self, construction: &MathConstruction) -> Result<(), Error> {
        count(construction.variants.len())?;
        for variant in &construction.variants {
            self.name(&variant.glyph)?;
            units(variant.advance, true)?;
        }
        if let Some(assembly) = &construction.assembly {
            self.value(&assembly.italic_correction)?;
            count(assembly.parts.len())?;
            for part in &assembly.parts {
                self.name(&part.glyph)?;
                for value in [part.start_connector, part.end_connector, part.full_advance] {
                    units(value, true)?;
                }
                // OpenType specifies these as independent UFWORD values.
                // A connector may extend beyond the part's advance (for
                // example, the radical top in DejaVu Math TeX Gyre).
            }
        }
        Ok(())
    }
}

impl MathTable {
    /// Check references, finite binary ranges, counts, devices and kern intervals.
    ///
    /// Values are checked after rounding to the integer used in the binary.
    /// Assembly quality and optional layout-engine choices are not rejected.
    pub fn validate(&self, glyph_order: &[GlyphName]) -> Result<(), Error> {
        count(glyph_order.len())?;
        count(self.glyphs.len())?;
        let mut check = Check {
            names: glyph_order.iter().collect(),
            remaining: MAX_MATH_ITEMS,
        };
        if check.names.len() != glyph_order.len() {
            return Err(invalid("duplicate glyph names in export order"));
        }
        units(self.min_connector_overlap, true)?;
        if let Some(constants) = &self.constants {
            units(constants.delimited_sub_formula_min_height, true)?;
            units(constants.display_operator_min_height, true)?;
            for value in constants.values() {
                check.value(value)?;
            }
        }
        for (name, glyph) in &self.glyphs {
            check.name(name)?;
            for value in [&glyph.italic_correction, &glyph.top_accent]
                .into_iter()
                .flatten()
            {
                check.value(value)?;
            }
            for kern in glyph.kern.corners().into_iter().flatten() {
                count(kern.heights.len())?;
                if kern.values.len() != kern.heights.len() + 1 {
                    return Err(invalid(format!(
                        "glyph `{name}`: kern values must outnumber heights by one"
                    )));
                }
                for value in kern.heights.iter().chain(&kern.values) {
                    check.value(value)?;
                }
                if kern
                    .heights
                    .windows(2)
                    .any(|pair| pair[0].value.round() >= pair[1].value.round())
                {
                    return Err(invalid(format!(
                        "glyph `{name}`: rounded kern heights must increase"
                    )));
                }
            }
            for construction in [&glyph.vertical, &glyph.horizontal].into_iter().flatten() {
                check.construction(construction)?;
            }
        }
        Ok(())
    }
}
