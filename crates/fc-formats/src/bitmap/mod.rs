//! Native bitmap font formats, with bounded decoding and atomic output.

pub mod bdf;
mod binary;
pub mod fnt;
pub mod pcf;
pub mod psf;

use crate::Error;
use fc_model::{BitmapGlyph, BitmapStrike, Font, Glyph};
use std::{
    io::{Read, Write},
    path::Path,
};

const MAX_INPUT: u64 = 128 * 1024 * 1024;

/// Recognize supported standalone bitmap extensions, including gzip wrappers.
pub fn is_bitmap_path(path: &Path) -> bool {
    let name = path.to_string_lossy().to_ascii_lowercase();
    let name = name.strip_suffix(".gz").unwrap_or(&name);
    [".bdf", ".psf", ".psfu", ".pcf", ".fnt", ".fon"]
        .iter()
        .any(|ext| name.ends_with(ext))
}

/// Sniff and decode a standalone bitmap font.
pub fn read(path: &Path) -> Result<Font, Error> {
    let bytes = read_bytes(path)?;
    if bytes.starts_with(b"STARTFONT") {
        bdf::decode(&bytes)
    } else if bytes.starts_with(&[0x36, 0x04]) || bytes.starts_with(&[0x72, 0xb5, 0x4a, 0x86]) {
        psf::decode(&bytes)
    } else if bytes.starts_with(b"\x01fcp") {
        pcf::decode(&bytes)
    } else if bytes.starts_with(b"MZ") {
        fnt::decode_fon(&bytes)
    } else if bytes.starts_with(&[0, 2]) || bytes.starts_with(&[0, 3]) {
        fnt::decode(&bytes)
    } else {
        Err(error("unrecognized bitmap font signature"))
    }
}

pub(super) fn error(message: impl Into<String>) -> Error {
    Error::Source {
        format: "bitmap",
        reason: message.into(),
    }
}

pub(super) fn checked<T>(result: Result<T, fc_model::Error>) -> Result<T, Error> {
    result.map_err(|e| error(e.to_string()))
}

/// Read a bounded file, decompressing gzip by magic bytes.
pub fn read_bytes(path: &Path) -> Result<Vec<u8>, Error> {
    let file = std::fs::File::open(path).map_err(|e| Error::io(path, e))?;
    let mut bytes = Vec::new();
    file.take(MAX_INPUT + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| Error::io(path, e))?;
    if bytes.len() as u64 > MAX_INPUT {
        return Err(error("bitmap file exceeds input limit"));
    }
    if bytes.starts_with(&[0x1f, 0x8b]) {
        let mut decoded = Vec::new();
        flate2::read::GzDecoder::new(bytes.as_slice())
            .take(MAX_INPUT + 1)
            .read_to_end(&mut decoded)
            .map_err(|e| Error::io(path, e))?;
        if decoded.len() as u64 > MAX_INPUT {
            return Err(error("decoded bitmap file exceeds input limit"));
        }
        bytes = decoded;
    }
    Ok(bytes)
}

/// Replace a destination only after serialization succeeds.
pub fn write_bytes(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary =
        tempfile::NamedTempFile::new_in(parent).map_err(|e| Error::io(parent, e))?;
    if path.extension().is_some_and(|ext| ext == "gz") {
        let mut gzip =
            flate2::write::GzEncoder::new(&mut temporary, flate2::Compression::default());
        gzip.write_all(bytes).map_err(|e| Error::io(path, e))?;
        gzip.finish().map_err(|e| Error::io(path, e))?;
    } else {
        temporary.write_all(bytes).map_err(|e| Error::io(path, e))?;
    }
    temporary
        .as_file()
        .sync_all()
        .map_err(|e| Error::io(path, e))?;
    temporary
        .persist(path)
        .map_err(|e| Error::io(path, e.error))?;
    Ok(())
}

pub(super) fn add_glyph(
    font: &mut Font,
    strike: &mut BitmapStrike,
    name: &str,
    codes: Vec<u32>,
    bitmap: BitmapGlyph,
) -> Result<(), Error> {
    if font.glyphs.len() >= 65535 || name.is_empty() || font.glyphs.contains_key(name) {
        return Err(error("duplicate/empty glyph name or glyph limit"));
    }
    let mut glyph = Glyph::new(name);
    glyph.unicodes = codes;
    checked(font.insert_glyph(glyph))?;
    if let Some(layer) = font.glyph_mut(name).and_then(|g| g.layers.first_mut()) {
        layer.width = f64::from(bitmap.advance) * 1000.0 / f64::from(strike.ppem);
    }
    strike.glyphs.insert(name.into(), bitmap);
    Ok(())
}

/// Choose a strike explicitly; ambiguous exports require a size/depth choice.
pub fn select_strike(font: &Font, key: Option<(u16, u8)>) -> Result<&BitmapStrike, Error> {
    checked(font.validate_bitmaps())?;
    if font.has_unapplied_feature_draft() {
        return Err(Error::UnappliedFeatureDraft);
    }
    match key {
        Some(key) => font
            .bitmap_strikes
            .iter()
            .find(|s| (s.ppem, s.depth) == key)
            .ok_or_else(|| error("requested bitmap strike does not exist")),
        None if font.bitmap_strikes.len() == 1 => Ok(&font.bitmap_strikes[0]),
        None => Err(error("select a bitmap strike for this export")),
    }
}
