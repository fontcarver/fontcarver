use std::collections::{BTreeSet, HashSet};

use crate::{BdfValue, Bitmap, BitmapGlyph, BitmapStrike, Error, Font, GlyphName};

/// Maximum bitmap dimension and strike size accepted by the editor.
pub const MAX_BITMAP_SIZE: u16 = 4096;
/// Maximum decoded bytes in one bitmap.
pub const MAX_BITMAP_BYTES: usize = 16 * 1024 * 1024;
/// Maximum decoded bitmap bytes in one font.
pub const MAX_FONT_BITMAP_BYTES: usize = 256 * 1024 * 1024;
/// Maximum number of format properties stored on one strike.
pub const MAX_STRIKE_PROPERTIES: usize = 65_535;
/// Maximum length of one property name, in characters.
pub const MAX_PROPERTY_NAME_CHARS: usize = 255;
/// Maximum length of one string property value, in bytes.
pub const MAX_PROPERTY_VALUE_BYTES: usize = 65_535;

pub(crate) fn invalid(message: impl Into<String>) -> Error {
    Error::InvalidBitmap(message.into())
}

/// Validate one strike property so that it survives every bitmap format.
///
/// A BDF, PCF or FNT property name is a single whitespace-delimited token and a
/// string value is written between quotes on one line, so whitespace in a name
/// and a line break or NUL in a value cannot be represented at all. Rejecting
/// them here keeps an unrepresentable property out of the document instead of
/// failing much later, at export.
pub fn validate_strike_property(name: &str, value: &BdfValue) -> Result<(), Error> {
    if name.is_empty() || name.chars().count() > MAX_PROPERTY_NAME_CHARS {
        return Err(invalid(
            "a strike property name must have 1 to 255 characters",
        ));
    }
    if name
        .chars()
        .any(|character| character.is_whitespace() || character.is_control())
    {
        return Err(invalid(format!(
            "strike property name `{name}` contains whitespace or a control character"
        )));
    }
    if let BdfValue::Str(text) = value {
        if text.len() > MAX_PROPERTY_VALUE_BYTES {
            return Err(invalid(format!(
                "strike property `{name}` exceeds 65535 bytes"
            )));
        }
        if text.contains(['\n', '\r', '\0']) {
            return Err(invalid(format!(
                "strike property `{name}` contains a line break or NUL"
            )));
        }
    }
    Ok(())
}

impl Bitmap {
    /// Checked bytes per row, including trailing byte padding.
    pub fn stride(&self) -> Result<usize, Error> {
        if !matches!(self.depth, 1 | 2 | 4 | 8 | 32) {
            return Err(invalid("depth must be 1, 2, 4, 8 or 32"));
        }
        if self.width > MAX_BITMAP_SIZE || self.height > MAX_BITMAP_SIZE {
            return Err(invalid("bitmap dimension exceeds 4096 pixels"));
        }
        Ok((usize::from(self.width) * usize::from(self.depth)).div_ceil(8))
    }

    /// Reject malformed pixel buffers and nonzero unused row bits.
    pub fn validate(&self) -> Result<(), Error> {
        let stride = self.stride()?;
        let length = stride * usize::from(self.height);
        if length > MAX_BITMAP_BYTES || self.rows.len() != length {
            return Err(invalid("invalid bitmap buffer length"));
        }
        let bits = (usize::from(self.width) * usize::from(self.depth)) % 8;
        if bits != 0
            && self
                .rows
                .chunks_exact(stride)
                .any(|row| row[stride - 1] & ((1 << (8 - bits)) - 1) != 0)
        {
            return Err(invalid("nonzero bitmap row padding"));
        }
        Ok(())
    }

    /// Allocate an empty, bounded bitmap; origins default to zero.
    pub fn new(width: u16, height: u16, depth: u8) -> Result<Self, Error> {
        let mut bitmap = Self {
            width,
            height,
            depth,
            ..Self::default()
        };
        let size = bitmap.stride()? * usize::from(height);
        if size > MAX_BITMAP_BYTES {
            return Err(invalid("bitmap allocation exceeds limit"));
        }
        bitmap.rows = vec![0; size];
        Ok(bitmap)
    }

    /// Read a sample in top-left buffer coordinates. Outside pixels are zero.
    pub fn pixel(&self, x: u16, y: u16) -> u32 {
        if x >= self.width || y >= self.height {
            return 0;
        }
        let Ok(stride) = self.stride() else {
            return 0;
        };
        let bit = usize::from(x) * usize::from(self.depth);
        let offset = usize::from(y) * stride + bit / 8;
        if self.depth == 32 {
            return self
                .rows
                .get(offset..offset + 4)
                .map_or(0, |s| u32::from_be_bytes([s[0], s[1], s[2], s[3]]));
        }
        self.rows.get(offset).map_or(0, |byte| {
            u32::from(
                (byte >> (8 - self.depth - (bit % 8) as u8)) & ((1u16 << self.depth) - 1) as u8,
            )
        })
    }

    /// Set one sample, rejecting out-of-bounds coordinates and values.
    pub fn set_pixel(&mut self, x: u16, y: u16, value: u32) -> Result<(), Error> {
        let stride = self.stride()?;
        if x >= self.width || y >= self.height || (self.depth < 32 && value >= (1 << self.depth)) {
            return Err(invalid("pixel coordinate or value is out of range"));
        }
        let bit = usize::from(x) * usize::from(self.depth);
        let offset = usize::from(y) * stride + bit / 8;
        if self.depth == 32 {
            let dst = self
                .rows
                .get_mut(offset..offset + 4)
                .ok_or_else(|| invalid("truncated pixels"))?;
            dst.copy_from_slice(&value.to_be_bytes());
        } else {
            let shift = 8 - self.depth - (bit % 8) as u8;
            let mask = (((1u16 << self.depth) - 1) as u8) << shift;
            let dst = self
                .rows
                .get_mut(offset)
                .ok_or_else(|| invalid("truncated pixels"))?;
            *dst = (*dst & !mask) | ((value as u8) << shift);
        }
        Ok(())
    }

    /// Alpha/coverage mapped to 0–255, including RGBA samples.
    pub fn coverage(&self, x: u16, y: u16) -> u8 {
        let value = self.pixel(x, y);
        if self.depth == 32 {
            value as u8
        } else if matches!(self.depth, 1 | 2 | 4 | 8) {
            (value * 255 / ((1 << self.depth) - 1)) as u8
        } else {
            0
        }
    }
}

impl BitmapStrike {
    /// Validate metrics, bitmap storage and a bounded acyclic reference graph.
    pub fn validate(&self) -> Result<(), Error> {
        if self.ppem == 0
            || self.ppem > MAX_BITMAP_SIZE
            || !matches!(self.depth, 1 | 2 | 4 | 8 | 32)
        {
            return Err(invalid("invalid strike size or depth"));
        }
        if self.ascent < 0 || self.descent < 0 || self.glyphs.len() > 65535 {
            return Err(invalid("invalid strike metrics or glyph count"));
        }
        if self.properties.len() > MAX_STRIKE_PROPERTIES {
            return Err(invalid("strike exceeds 65535 properties"));
        }
        for (name, value) in &self.properties {
            validate_strike_property(name, value)?;
        }
        let mut bytes = 0usize;
        for glyph in self.glyphs.values() {
            glyph.bitmap.validate()?;
            if glyph.bitmap.depth != self.depth {
                return Err(invalid("glyph depth differs from strike"));
            }
            bytes += glyph.bitmap.rows.len();
            if bytes > MAX_FONT_BITMAP_BYTES || glyph.refs.len() > 1024 {
                return Err(invalid("bitmap resource limit exceeded"));
            }
        }
        let mut done = HashSet::new();
        for name in self.glyphs.keys() {
            self.check_refs(name, &mut HashSet::new(), &mut done)?;
        }
        Ok(())
    }

    fn check_refs<'a>(
        &'a self,
        name: &'a GlyphName,
        active: &mut HashSet<&'a GlyphName>,
        done: &mut HashSet<&'a GlyphName>,
    ) -> Result<(), Error> {
        if done.contains(name) {
            return Ok(());
        }
        if active.len() >= 64 || !active.insert(name) {
            return Err(invalid("bitmap reference cycle or depth limit"));
        }
        let glyph = self
            .glyphs
            .get(name)
            .ok_or_else(|| invalid(format!("missing bitmap reference {name}")))?;
        for reference in &glyph.refs {
            self.check_refs(&reference.base, active, done)?;
        }
        active.remove(name);
        done.insert(name);
        Ok(())
    }

    /// Resolve references into one bitmap, preserving the referencing glyph's metrics.
    pub fn flattened(&self, name: &GlyphName) -> Result<BitmapGlyph, Error> {
        let mut work = 0usize;
        self.flatten_inner(name, &mut HashSet::new(), &mut work)
    }

    fn flatten_inner(
        &self,
        name: &GlyphName,
        active: &mut HashSet<GlyphName>,
        work: &mut usize,
    ) -> Result<BitmapGlyph, Error> {
        *work += 1;
        if *work > 100_000 || active.len() >= 64 || !active.insert(name.clone()) {
            return Err(invalid("bitmap reference work limit or cycle"));
        }
        let mut result = self
            .glyphs
            .get(name)
            .ok_or_else(|| invalid(format!("missing bitmap glyph {name}")))?
            .clone();
        result.bitmap.validate()?;
        for reference in &result.refs {
            *work = work.saturating_add(
                usize::from(result.bitmap.width) * usize::from(result.bitmap.height) / 64,
            );
            if *work > 100_000 {
                return Err(invalid("bitmap compositing pixel work limit"));
            }
            let other = self.flatten_inner(&reference.base, active, work)?;
            result.bitmap = composite(&result.bitmap, &other.bitmap, reference.dx, reference.dy)?;
        }
        active.remove(name);
        result.refs.clear();
        Ok(result)
    }
}

fn composite(a: &Bitmap, b: &Bitmap, dx: i16, dy: i16) -> Result<Bitmap, Error> {
    if a.depth != b.depth {
        return Err(invalid("reference depth mismatch"));
    }
    if b.width == 0 || b.height == 0 {
        return Ok(a.clone());
    }
    if a.width == 0 || a.height == 0 {
        let mut out = b.clone();
        out.origin_x = out
            .origin_x
            .checked_add(dx)
            .ok_or_else(|| invalid("reference x overflow"))?;
        out.origin_y = out
            .origin_y
            .checked_add(dy)
            .ok_or_else(|| invalid("reference y overflow"))?;
        return Ok(out);
    }
    let ax = i32::from(a.origin_x);
    let ay = i32::from(a.origin_y);
    let bx = i32::from(b.origin_x) + i32::from(dx);
    let by = i32::from(b.origin_y) + i32::from(dy);
    let x = ax.min(bx);
    let y = ay.min(by);
    let right = (ax + i32::from(a.width)).max(bx + i32::from(b.width));
    let top = (ay + i32::from(a.height)).max(by + i32::from(b.height));
    let mut out = Bitmap::new(
        u16::try_from(right - x).map_err(|_| invalid("reference bounds"))?,
        u16::try_from(top - y).map_err(|_| invalid("reference bounds"))?,
        a.depth,
    )?;
    out.origin_x = i16::try_from(x).map_err(|_| invalid("reference origin"))?;
    out.origin_y = i16::try_from(y).map_err(|_| invalid("reference origin"))?;
    for (bitmap, ox, oy) in [(a, ax, ay), (b, bx, by)] {
        for row in 0..bitmap.height {
            for col in 0..bitmap.width {
                let tx = (ox - x + i32::from(col)) as u16;
                let ty = (top - oy - i32::from(bitmap.height) + i32::from(row)) as u16;
                let value = bitmap.pixel(col, row);
                let old = out.pixel(tx, ty);
                // Coverage union; RGBA uses straight-alpha source-over.
                let merged = if out.depth == 32 {
                    over(value, old)
                } else {
                    value.max(old)
                };
                out.set_pixel(tx, ty, merged)?;
            }
        }
    }
    Ok(out)
}

fn over(src: u32, dst: u32) -> u32 {
    let s = src.to_be_bytes();
    let d = dst.to_be_bytes();
    let sa = u32::from(s[3]);
    let da = u32::from(d[3]);
    let alpha = sa * 255 + da * (255 - sa);
    if alpha == 0 {
        return 0;
    }
    let mut out = [0; 4];
    for i in 0..3 {
        out[i] = ((u32::from(s[i]) * sa * 255 + u32::from(d[i]) * da * (255 - sa) + alpha / 2)
            / alpha) as u8;
    }
    out[3] = ((alpha + 127) / 255) as u8;
    u32::from_be_bytes(out)
}

impl Font {
    /// Check strike identities, storage, references and owning glyph names.
    pub fn validate_bitmaps(&self) -> Result<(), Error> {
        if self.bitmap_strikes.len() > 256 {
            return Err(invalid("font exceeds 256 bitmap strikes"));
        }
        let mut keys = BTreeSet::new();
        let mut bytes = 0usize;
        for strike in &self.bitmap_strikes {
            if !keys.insert((strike.ppem, strike.depth)) {
                return Err(invalid("duplicate bitmap strike"));
            }
            strike.validate()?;
            for (name, glyph) in &strike.glyphs {
                if !self.glyphs.contains_key(name) {
                    return Err(invalid(format!("bitmap has no owning glyph {name}")));
                }
                bytes += glyph.bitmap.rows.len();
                if bytes > MAX_FONT_BITMAP_BYTES {
                    return Err(invalid("font bitmap memory limit"));
                }
            }
        }
        Ok(())
    }
}
