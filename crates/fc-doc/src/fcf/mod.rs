//! Native FontCarver JSON persistence.

mod read;
mod write;

use std::path::{Path, PathBuf};

use fc_model::Font;
use serde::{de::IgnoredAny, Deserialize, Serialize};

use crate::Error;

const FORMAT: &str = "fontcarver";
/// Format tags written under earlier product names.
///
/// Documents carrying it are read unchanged; they are rewritten with [`FORMAT`]
/// on the next save.
const LEGACY_FORMATS: &[&str] = &["fontcutter", "fontworkshop"];
const VERSION: u32 = 1;

#[derive(Serialize)]
struct WriteEnvelope<'a> {
    format: &'static str,
    version: u32,
    font: &'a Font,
    #[serde(skip_serializing_if = "Option::is_none")]
    document_path: Option<&'a Path>,
}

#[derive(Deserialize)]
struct ReadEnvelope {
    format: String,
    version: u32,
    font: Font,
}

#[derive(Deserialize)]
struct MetadataEnvelope {
    #[serde(default)]
    format: String,
    #[serde(default)]
    version: u32,
    #[serde(default)]
    document_path: Option<PathBuf>,
    #[serde(rename = "font")]
    _font: IgnoredAny,
}

/// Writes a font to a `.fcf` or gzip-wrapped `.fcf.gz` path.
///
/// Non-finite imported kerning is rejected before touching the destination;
/// JSON cannot preserve those values, and silently writing null would make
/// the native document unreadable. Finite unresolved references are retained.
pub fn write(path: &Path, font: &Font) -> Result<(), Error> {
    write::write(path, font, None)
}

/// Reads a native document, sniffing gzip by its magic bytes.
pub fn read(path: &Path) -> Result<Font, Error> {
    Ok(validate(read::read(path)?)?.font)
}

pub(crate) fn write_autosave(
    path: &Path,
    font: &Font,
    document_path: Option<&Path>,
) -> Result<(), Error> {
    write::write(path, font, document_path)
}

pub(crate) fn read_document_path(path: &Path) -> Result<Option<PathBuf>, Error> {
    let envelope = read::read_metadata(path)?;
    validate_metadata(&envelope)?;
    Ok(envelope.document_path)
}

fn envelope<'a>(font: &'a Font, document_path: Option<&'a Path>) -> WriteEnvelope<'a> {
    WriteEnvelope {
        format: FORMAT,
        version: VERSION,
        font,
        document_path,
    }
}

fn is_known_format(format: &str) -> bool {
    format == FORMAT || LEGACY_FORMATS.contains(&format)
}

fn validate(envelope: ReadEnvelope) -> Result<ReadEnvelope, Error> {
    if !is_known_format(&envelope.format) {
        return Err(Error::InvalidFcfFormat(envelope.format));
    }
    if envelope.version > VERSION {
        return Err(Error::UnsupportedFcfVersion(envelope.version));
    }
    Ok(envelope)
}

fn validate_metadata(envelope: &MetadataEnvelope) -> Result<(), Error> {
    if !is_known_format(&envelope.format) {
        return Err(Error::InvalidFcfFormat(envelope.format.clone()));
    }
    if envelope.version > VERSION {
        return Err(Error::UnsupportedFcfVersion(envelope.version));
    }
    Ok(())
}
