use super::error;
use crate::Error;

pub(super) struct Reader<'a> {
    pub data: &'a [u8],
    pub pos: usize,
    pub big: bool,
}
impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            big: false,
        }
    }
    pub fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or_else(|| error("binary offset overflow"))?;
        let data = self
            .data
            .get(self.pos..end)
            .ok_or_else(|| error("truncated bitmap data"))?;
        self.pos = end;
        Ok(data)
    }
    pub fn u8(&mut self) -> Result<u8, Error> {
        Ok(self.take(1)?[0])
    }
    pub fn u16(&mut self) -> Result<u16, Error> {
        let big = self.big;
        let b = self.take(2)?;
        Ok(if big {
            u16::from_be_bytes([b[0], b[1]])
        } else {
            u16::from_le_bytes([b[0], b[1]])
        })
    }
    pub fn i16(&mut self) -> Result<i16, Error> {
        Ok(self.u16()? as i16)
    }
    pub fn u32(&mut self) -> Result<u32, Error> {
        let big = self.big;
        let b = self.take(4)?;
        Ok(if big {
            u32::from_be_bytes([b[0], b[1], b[2], b[3]])
        } else {
            u32::from_le_bytes([b[0], b[1], b[2], b[3]])
        })
    }
    pub fn count(&mut self, max: usize) -> Result<usize, Error> {
        let n = self.u32()? as usize;
        if n > max {
            Err(error("bitmap count exceeds resource limit"))
        } else {
            Ok(n)
        }
    }
}
pub(super) fn u16le(out: &mut Vec<u8>, v: u16) {
    out.extend(v.to_le_bytes());
}
pub(super) fn u32le(out: &mut Vec<u8>, v: u32) {
    out.extend(v.to_le_bytes());
}
pub(super) fn cstr(bytes: &[u8], offset: usize) -> Result<String, Error> {
    let tail = bytes
        .get(offset..)
        .ok_or_else(|| error("string offset out of bounds"))?;
    let end = tail
        .iter()
        .position(|b| *b == 0)
        .ok_or_else(|| error("unterminated bitmap string"))?;
    String::from_utf8(tail[..end].to_vec()).map_err(|_| error("invalid bitmap string encoding"))
}
