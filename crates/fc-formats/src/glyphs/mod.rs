//! Native Glyphs 2/3 import and atomic Glyphs 3 source export.
//!
//! The native plist remains authoritative. Unknown source dictionaries are retained
//! separately from narrowly scoped editor metadata; neither replaces native geometry.

use crate::Error;
use fc_model::{FeaturesSource, Font};
use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

mod conditional;
mod extras;
mod metadata;
mod outlines;
mod plist;
mod read;
mod smart;
mod write;

const MAX_BYTES: u64 = 64 * 1024 * 1024;
const SHADOW: &str = "org.fontcarver.glyphs.source";
const METADATA: &str = "org.fontcarver.glyphs.editor.v1";

fn error(reason: impl Into<String>) -> Error {
    Error::Source {
        format: "Glyphs",
        reason: reason.into(),
    }
}

/// Reads a bounded, native Glyphs 2 or Glyphs 3 single-file source.
pub fn read_glyphs(path: &Path) -> Result<Font, Error> {
    let mut text = String::new();
    File::open(path)
        .map_err(|err| Error::io(path, err))?
        .take(MAX_BYTES + 1)
        .read_to_string(&mut text)
        .map_err(|err| Error::io(path, err))?;
    let raw = plist::parse(&text)?;
    let mut font = read::font(&raw)?;
    font.extra
        .lib
        .insert(SHADOW.into(), serde_json::Value::String(text));
    font.extra.source = Some(fc_model::SourceProvenance {
        path: Some(path.to_string_lossy().into_owned()),
        format: "Glyphs".into(),
        glyph_order: font.glyph_order().cloned().collect(),
        ..Default::default()
    });
    Ok(font)
}

/// Writes all masters as native Glyphs 3, replacing the destination atomically.
///
/// Invalid feature authority, unsupported semantics and nonfinite data are rejected
/// before the destination is touched. Supplemental editing metadata lives in userData.
pub fn write_glyphs(font: &Font, path: &Path) -> Result<(), Error> {
    if font.has_unapplied_feature_draft() {
        return Err(Error::UnappliedFeatureDraft);
    }
    if font.features.source_of_truth == FeaturesSource::ImportedTables {
        return Err(Error::ImportedLayoutRequiresConversion);
    }
    let raw = write::font(font)?;
    let text = plist::serialize(&raw);
    if text.len() as u64 > MAX_BYTES {
        return Err(error("source exceeds the 64 MiB limit"));
    }
    // Verify our own native output before opening a staging file.
    let _ = read::font(&raw)?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary =
        tempfile::NamedTempFile::new_in(parent).map_err(|err| Error::io(parent, err))?;
    temporary
        .write_all(text.as_bytes())
        .map_err(|err| Error::io(path, err))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|err| Error::io(path, err))?;
    temporary
        .persist(path)
        .map_err(|err| Error::io(path, err.error))?;
    Ok(())
}
