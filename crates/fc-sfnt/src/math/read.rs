use super::{error, MAX_TABLE_BYTES};
use crate::Error;
use fc_model::{
    GlyphName, MathAssembly, MathConstants, MathConstruction, MathDevice, MathGlyphInfo, MathKern,
    MathKerns, MathPart, MathTable, MathValue, MathVariant, MAX_MATH_ITEMS,
};

#[derive(Clone, Copy)]
struct Data<'a>(&'a [u8]);

impl<'a> Data<'a> {
    fn check(self, length: usize) -> Result<(), Error> {
        if self.0.len() < length {
            return Err(error("truncated subtable"));
        }
        Ok(())
    }
    fn u16(self, at: usize) -> Result<u16, Error> {
        let bytes = self
            .0
            .get(at..at + 2)
            .ok_or_else(|| error("truncated field"))?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }
    fn i16(self, at: usize) -> Result<i16, Error> {
        Ok(self.u16(at)? as i16)
    }
    fn sub(self, at: usize, header: usize) -> Result<Option<Self>, Error> {
        let offset = usize::from(self.u16(at)?);
        if offset == 0 {
            return Ok(None);
        }
        if offset < header {
            return Err(error("subtable offset points into its parent header"));
        }
        let bytes = self
            .0
            .get(offset..)
            .ok_or_else(|| error("subtable offset exceeds table"))?;
        Ok(Some(Self(bytes)))
    }
    fn required(self, at: usize, header: usize) -> Result<Self, Error> {
        self.sub(at, header)?
            .ok_or_else(|| error("required offset is null"))
    }
}

struct Reader<'a> {
    order: &'a [GlyphName],
    remaining: usize,
}

impl Reader<'_> {
    fn take(&mut self, count: usize) -> Result<(), Error> {
        self.remaining = self
            .remaining
            .checked_sub(count)
            .ok_or_else(|| error("decoded item limit exceeded"))?;
        Ok(())
    }
    fn name(&self, gid: u16) -> Result<GlyphName, Error> {
        self.order
            .get(usize::from(gid))
            .cloned()
            .ok_or_else(|| error(format!("glyph ID {gid} exceeds glyph count")))
    }
    fn coverage(
        &mut self,
        data: Option<Data<'_>>,
        count: Option<usize>,
    ) -> Result<Vec<GlyphName>, Error> {
        let Some(data) = data else {
            if count.is_some_and(|n| n != 0) {
                return Err(error("nonempty coverage has a null offset"));
            }
            return Ok(Vec::new());
        };
        let records = usize::from(data.u16(2)?);
        self.take(records)?;
        let mut ids = Vec::new();
        match data.u16(0)? {
            1 => {
                data.check(4 + records * 2)?;
                for i in 0..records {
                    ids.push(data.u16(4 + i * 2)?);
                }
            }
            2 => {
                data.check(4 + records * 6)?;
                for i in 0..records {
                    let start = data.u16(4 + i * 6)?;
                    let end = data.u16(6 + i * 6)?;
                    let index = usize::from(data.u16(8 + i * 6)?);
                    if end < start || index != ids.len() {
                        return Err(error("invalid coverage range or index"));
                    }
                    let length = usize::from(end) - usize::from(start) + 1;
                    self.take(length)?;
                    if ids.len() + length > self.order.len() {
                        return Err(error("coverage exceeds glyph count"));
                    }
                    ids.extend(start..=end);
                }
            }
            _ => return Err(error("unsupported coverage format")),
        }
        if count.is_some_and(|n| n != ids.len()) || ids.windows(2).any(|p| p[0] >= p[1]) {
            return Err(error("coverage count or glyph ordering is invalid"));
        }
        ids.into_iter().map(|id| self.name(id)).collect()
    }
    fn value(&mut self, data: Data<'_>, at: usize, header: usize) -> Result<MathValue, Error> {
        self.take(1)?;
        Ok(MathValue {
            value: f64::from(data.i16(at)?),
            device: data
                .sub(at + 2, header)?
                .map(|d| self.device(d))
                .transpose()?,
        })
    }
    fn device(&mut self, data: Data<'_>) -> Result<MathDevice, Error> {
        let start_size = data.u16(0)?;
        let end_size = data.u16(2)?;
        if end_size < start_size {
            return Err(error("device end size precedes start size"));
        }
        let bits =
            match data.u16(4)? {
                1 => 2,
                2 => 4,
                3 => 8,
                _ => return Err(error(
                    "unsupported MATH device format; variation indices are not defined for MATH",
                )),
            };
        let count = usize::from(end_size) - usize::from(start_size) + 1;
        self.take(count)?;
        let per_word = 16 / bits;
        data.check(6 + count.div_ceil(per_word) * 2)?;
        let mut deltas = Vec::with_capacity(count);
        for i in 0..count {
            let word = data.u16(6 + i / per_word * 2)?;
            let value = ((word >> (16 - bits - (i % per_word) * bits)) & ((1 << bits) - 1)) as i16;
            let signed = if value & (1 << (bits - 1)) != 0 {
                value - (1 << bits)
            } else {
                value
            };
            deltas.push(signed as i8);
        }
        Ok(MathDevice { start_size, deltas })
    }
    fn constants(&mut self, data: Data<'_>) -> Result<MathConstants, Error> {
        const SIZE: usize = 8 + 51 * 4 + 2;
        data.check(SIZE)?;
        let mut constants = MathConstants {
            script_percent_scale_down: data.i16(0)?,
            script_script_percent_scale_down: data.i16(2)?,
            delimited_sub_formula_min_height: f64::from(data.u16(4)?),
            display_operator_min_height: f64::from(data.u16(6)?),
            radical_degree_bottom_raise_percent: data.i16(SIZE - 2)?,
            ..Default::default()
        };
        for (i, value) in constants.values_mut().into_iter().enumerate() {
            *value = self.value(data, 8 + i * 4, SIZE)?;
        }
        Ok(constants)
    }
    fn glyph_info(&mut self, data: Data<'_>, math: &mut MathTable) -> Result<(), Error> {
        data.check(8)?;
        for (offset, accent) in [(0, false), (2, true)] {
            if let Some(values) = data.sub(offset, 8)? {
                let count = usize::from(values.u16(2)?);
                let size = 4 + count * 4;
                values.check(size)?;
                let names = self.coverage(values.sub(0, size)?, Some(count))?;
                for (i, name) in names.into_iter().enumerate() {
                    let value = Some(self.value(values, 4 + i * 4, size)?);
                    let glyph = math.glyphs.entry(name).or_default();
                    if accent {
                        glyph.top_accent = value;
                    } else {
                        glyph.italic_correction = value;
                    }
                }
            }
        }
        for name in self.coverage(data.sub(4, 8)?, None)? {
            math.glyphs.entry(name).or_default().extended_shape = true;
        }
        if let Some(kerns) = data.sub(6, 8)? {
            let count = usize::from(kerns.u16(2)?);
            let size = 4 + count * 8;
            kerns.check(size)?;
            let names = self.coverage(kerns.sub(0, size)?, Some(count))?;
            for (i, name) in names.into_iter().enumerate() {
                let at = 4 + i * 8;
                let mut corner =
                    |offset| kerns.sub(offset, size)?.map(|d| self.kern(d)).transpose();
                let kern = MathKerns {
                    top_right: corner(at)?,
                    top_left: corner(at + 2)?,
                    bottom_right: corner(at + 4)?,
                    bottom_left: corner(at + 6)?,
                };
                if kern != MathKerns::default() {
                    math.glyphs.entry(name).or_default().kern = kern;
                }
            }
        }
        Ok(())
    }
    fn kern(&mut self, data: Data<'_>) -> Result<MathKern, Error> {
        let count = usize::from(data.u16(0)?);
        let size = 2 + (count * 2 + 1) * 4;
        data.check(size)?;
        // Charge before allocating: a malicious font may reuse one huge subtable.
        self.take(count * 2 + 1)?;
        let heights = (0..count)
            .map(|i| self.value(data, 2 + i * 4, size))
            .collect::<Result<_, _>>()?;
        let values = (0..=count)
            .map(|i| self.value(data, 2 + (count + i) * 4, size))
            .collect::<Result<_, _>>()?;
        Ok(MathKern { heights, values })
    }
    fn variants(&mut self, data: Data<'_>, math: &mut MathTable) -> Result<(), Error> {
        data.check(10)?;
        math.min_connector_overlap = f64::from(data.u16(0)?);
        let vertical = usize::from(data.u16(6)?);
        let horizontal = usize::from(data.u16(8)?);
        let size = 10 + (vertical + horizontal) * 2;
        data.check(size)?;
        for (coverage, count, start, is_vertical) in
            [(2, vertical, 0, true), (4, horizontal, vertical, false)]
        {
            let names = self.coverage(data.sub(coverage, size)?, Some(count))?;
            for (i, name) in names.into_iter().enumerate() {
                let construction = self.construction(data.required(10 + (start + i) * 2, size)?)?;
                let glyph = math.glyphs.entry(name).or_default();
                if is_vertical {
                    glyph.vertical = Some(construction);
                } else {
                    glyph.horizontal = Some(construction);
                }
            }
        }
        Ok(())
    }
    fn construction(&mut self, data: Data<'_>) -> Result<MathConstruction, Error> {
        let count = usize::from(data.u16(2)?);
        self.take(count + 1)?;
        let size = 4 + count * 4;
        data.check(size)?;
        let variants = (0..count)
            .map(|i| {
                Ok(MathVariant {
                    glyph: self.name(data.u16(4 + i * 4)?)?,
                    advance: f64::from(data.u16(6 + i * 4)?),
                })
            })
            .collect::<Result<_, Error>>()?;
        let assembly = data.sub(0, size)?.map(|d| self.assembly(d)).transpose()?;
        Ok(MathConstruction { variants, assembly })
    }
    fn assembly(&mut self, data: Data<'_>) -> Result<MathAssembly, Error> {
        let count = usize::from(data.u16(4)?);
        self.take(count)?;
        let size = 6 + count * 10;
        data.check(size)?;
        let italic_correction = self.value(data, 0, size)?;
        let parts = (0..count)
            .map(|i| {
                let at = 6 + i * 10;
                let flags = data.u16(at + 8)?;
                if flags & !1 != 0 {
                    return Err(error("reserved assembly part flags are set"));
                }
                Ok(MathPart {
                    glyph: self.name(data.u16(at)?)?,
                    start_connector: f64::from(data.u16(at + 2)?),
                    end_connector: f64::from(data.u16(at + 4)?),
                    full_advance: f64::from(data.u16(at + 6)?),
                    extender: flags == 1,
                })
            })
            .collect::<Result<_, _>>()?;
        Ok(MathAssembly {
            italic_correction,
            parts,
        })
    }
}

/// Decode one MATH table, rejecting malformed data before returning any model.
///
/// The input is limited to 16 MiB and one million decoded work items. Nullable
/// top-level subtables are represented by absent constants or empty glyph data.
pub fn read_table(bytes: &[u8], glyph_order: &[GlyphName]) -> Result<MathTable, Error> {
    if bytes.len() > MAX_TABLE_BYTES {
        return Err(error("table exceeds 16 MiB"));
    }
    let data = Data(bytes);
    data.check(10)?;
    if data.u16(0)? != 1 || data.u16(2)? != 0 {
        return Err(error("unsupported MATH version"));
    }
    let mut reader = Reader {
        order: glyph_order,
        remaining: MAX_MATH_ITEMS,
    };
    let mut math = MathTable {
        constants: data.sub(4, 10)?.map(|d| reader.constants(d)).transpose()?,
        ..Default::default()
    };
    if let Some(info) = data.sub(6, 10)? {
        reader.glyph_info(info, &mut math)?;
    }
    if let Some(variants) = data.sub(8, 10)? {
        reader.variants(variants, &mut math)?;
    }
    math.glyphs
        .retain(|_, info| info != &MathGlyphInfo::default());
    math.validate(glyph_order)
        .map_err(|e| error(e.to_string()))?;
    Ok(math)
}
