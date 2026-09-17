//! Bounded, checked binary access shared by the static layout readers.
use fc_model::GlyphName;

pub(crate) type Result<T> = std::result::Result<T, String>;
pub(crate) const MAX_RULES: usize = 100_000;
pub(crate) const MAX_GLYPHS: usize = 65_535;
pub(crate) const MAX_TEXT_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy)]
pub(crate) struct Reader<'a> {
    pub data: &'a [u8],
    pub base: usize,
}
impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, base: 0 }
    }
    pub fn bytes(self, offset: usize, len: usize) -> Result<&'a [u8]> {
        let start = self.base.checked_add(offset).ok_or("offset overflow")?;
        let end = start.checked_add(len).ok_or("offset overflow")?;
        self.data
            .get(start..end)
            .ok_or_else(|| "truncated or invalid table offset".into())
    }
    pub fn u16(self, offset: usize) -> Result<u16> {
        let b = self.bytes(offset, 2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }
    pub fn i16(self, offset: usize) -> Result<i16> {
        Ok(self.u16(offset)? as i16)
    }
    pub fn u32(self, offset: usize) -> Result<u32> {
        let b = self.bytes(offset, 4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
    pub fn at(self, offset: usize) -> Result<Self> {
        self.bytes(offset, 1)?;
        Ok(Self {
            data: self.data,
            base: self.base + offset,
        })
    }
    pub fn offset16(self, field: usize) -> Result<Self> {
        let offset = usize::from(self.u16(field)?);
        if offset == 0 {
            return Err("required offset is null".into());
        }
        self.at(offset)
    }
    pub fn tag(self, offset: usize) -> Result<String> {
        let bytes = self.bytes(offset, 4)?;
        if !bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || *b == b' ')
        {
            return Err("nonrepresentable layout tag".into());
        }
        Ok(String::from_utf8_lossy(bytes).trim_end().into())
    }
    pub fn array(self, offset: usize, count: usize) -> Result<Vec<u16>> {
        self.bytes(offset, count.checked_mul(2).ok_or("array overflow")?)?;
        (0..count).map(|i| self.u16(offset + i * 2)).collect()
    }
    pub fn coverage(self, names: &[GlyphName]) -> Result<Vec<u16>> {
        let mut result = Vec::new();
        match self.u16(0)? {
            1 => {
                result = self.array(4, self.u16(2)? as usize)?;
            }
            2 => {
                for i in 0..self.u16(2)? as usize {
                    let start = self.u16(4 + i * 6)?;
                    let end = self.u16(6 + i * 6)?;
                    if start > end || self.u16(8 + i * 6)? as usize != result.len() {
                        return Err("invalid coverage range/index".into());
                    }
                    result.extend(start..=end);
                    if result.len() > MAX_GLYPHS {
                        return Err("coverage resource limit".into());
                    }
                }
            }
            _ => return Err("unsupported coverage format".into()),
        }
        if result.windows(2).any(|p| p[0] >= p[1]) {
            return Err("coverage is not strictly ordered".into());
        }
        for &gid in &result {
            glyph(names, gid)?;
        }
        Ok(result)
    }
    pub fn classes(self, names: &[GlyphName]) -> Result<Vec<u16>> {
        let mut result = vec![0; names.len()];
        match self.u16(0)? {
            1 => {
                let first = self.u16(2)? as usize;
                for (i, class) in self
                    .array(6, self.u16(4)? as usize)?
                    .into_iter()
                    .enumerate()
                {
                    *result
                        .get_mut(first + i)
                        .ok_or("class glyph is out of range")? = class;
                }
            }
            2 => {
                let mut previous = None;
                for i in 0..self.u16(2)? as usize {
                    let start = self.u16(4 + i * 6)?;
                    let end = self.u16(6 + i * 6)?;
                    let class = self.u16(8 + i * 6)?;
                    if start > end || previous.is_some_and(|p| p >= start) {
                        return Err("invalid class ranges".into());
                    }
                    for gid in start..=end {
                        *result
                            .get_mut(gid as usize)
                            .ok_or("class glyph is out of range")? = class;
                    }
                    previous = Some(end);
                }
            }
            _ => return Err("unsupported class definition format".into()),
        }
        Ok(result)
    }
}
pub(crate) fn glyph(names: &[GlyphName], gid: u16) -> Result<String> {
    let name: &str = names
        .get(gid as usize)
        .ok_or("glyph ID outside imported glyph order")?
        .as_ref();
    if name.is_empty()
        || name.len() > 127
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._+-*:^|~".contains(&b))
    {
        return Err("glyph name cannot be represented safely in FEA".into());
    }
    Ok(format!("\\{name}"))
}
pub(crate) fn class(names: &[GlyphName], gids: &[u16]) -> Result<String> {
    if gids.is_empty() {
        return Err("empty glyph set cannot be represented in FEA".into());
    }
    Ok(format!(
        "[{}]",
        gids.iter()
            .map(|gid| glyph(names, *gid))
            .collect::<Result<Vec<_>>>()?
            .join(" ")
    ))
}
pub(crate) fn members(classes: &[u16], id: u16) -> Vec<u16> {
    classes
        .iter()
        .enumerate()
        .filter_map(|(gid, value)| (*value == id).then_some(gid as u16))
        .collect()
}
#[derive(Clone, Debug, Default)]
pub(crate) struct Rules {
    pub preamble: Vec<String>,
    pub statements: Vec<String>,
    pub glyphs: Vec<Vec<u16>>,
    pub references: Vec<u16>,
    byte_length: usize,
    glyph_count: usize,
}
impl Rules {
    pub fn push(&mut self, text: String, gids: Vec<u16>) -> Result<()> {
        if self.statements.len() >= MAX_RULES {
            return Err("layout rule resource limit".into());
        }
        self.byte_length = self
            .byte_length
            .checked_add(text.len())
            .ok_or("source length overflow")?;
        self.glyph_count = self
            .glyph_count
            .checked_add(gids.len())
            .ok_or("glyph reference count overflow")?;
        if self.byte_length > MAX_TEXT_BYTES || self.glyph_count > 1_600_000 {
            return Err("layout source expansion resource limit".into());
        }
        self.statements.push(text);
        self.glyphs.push(gids);
        Ok(())
    }
}
