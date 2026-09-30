//! Bounded Type 42 wrappers around standalone TrueType font data.
//!
//! Input is a declarative, hexadecimal subset of Adobe technical note 5012.
//! No PostScript is executed. Binary `sfnts`, executable glyphs, metric overrides,
//! nonidentity transforms and stroked fonts are explicitly unsupported.

use std::io::Read;
use std::path::Path;

use crate::Error;

mod lexer;
mod reader;
mod sfnt;
mod writer;

const MAX_SOURCE_BYTES: usize = 2 * sfnt::MAX_SFNT_BYTES + 16 * 1024 * 1024;

/// Extracts the embedded TrueType SFNT from a literal Type 42 wrapper.
///
/// The embedded SFNT is returned unchanged; wrapper encoding and glyph aliases
/// are not applied to its existing cmap or post table.
pub fn read_bytes(bytes: &[u8]) -> Result<Vec<u8>, Error> {
    reader::read(bytes)
}

/// Reads a bounded Type 42 file and extracts its embedded TrueType SFNT.
pub fn read_path(path: &Path) -> Result<Vec<u8>, Error> {
    let file = std::fs::File::open(path).map_err(|source| Error::io(path, source))?;
    if file
        .metadata()
        .map_err(|source| Error::io(path, source))?
        .len()
        > MAX_SOURCE_BYTES as u64
    {
        return Err(error("Type 42 file size limit exceeded"));
    }
    let mut bytes = Vec::new();
    file.take(MAX_SOURCE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|source| Error::io(path, source))?;
    read_bytes(&bytes)
}

/// Wraps a standalone TrueType SFNT in a Type 42 PostScript font program.
///
/// Uses the font's PostScript and glyph names, Unicode codes 0–255 for Encoding,
/// and compatible padded strings split only at table or glyph boundaries.
/// An indivisible table or glyph exceeding 65,534 bytes is unsupported.
pub fn write_bytes(sfnt: &[u8]) -> Result<Vec<u8>, Error> {
    writer::write(sfnt)
}

fn error(reason: impl Into<String>) -> Error {
    Error::Source {
        format: "Type 42",
        reason: reason.into(),
    }
}
