//! Native static Spline Font Database import and export.
//!
//! Implemented from <https://fontforge.org/docs/techref/sfdformat.html>.
//! No FontForge implementation is linked, executed or adapted. Unsupported
//! output-affecting structures fail explicitly rather than becoming opaque data.
//!
//! Supported native layout covers single/multiple/alternate substitutions,
//! ligatures, single/pair positioning, one editable horizontal kerning subtable
//! with classes and explicit exceptions, and simple mark/ligature/cursive anchor
//! classes. Feature scripts/languages and basic lookup flags are retained.
//! Contextual rules, device adjustments, multiple native subtables, additive
//! editable kerning lookups and mark-to-mark anchor roles are rejected explicitly.
//! Selected-master export produces a static font even from a variable source.

use std::{
    io::{Read, Write},
    path::Path,
};

use fc_model::{FeaturesSource, Font, MasterId};

use crate::Error;

mod guards;
mod info;
mod kerning;
mod layout;
mod outlines;
mod reader;
mod strings;
mod writer;

const MAX_BYTES: usize = 64 * 1024 * 1024;
const MAX_GLYPHS: usize = 65_535;
const MAX_ITEMS: usize = 1_000_000;
const SHADOW: &str = "org.fontcarver.sfd.shadow";

/// Read a static SFD file, with bounded input and native geometry/layout parsing.
pub fn read_sfd(path: &Path) -> Result<Font, Error> {
    let file = std::fs::File::open(path).map_err(|error| Error::io(path, error))?;
    let mut bytes = Vec::new();
    file.take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| Error::io(path, error))?;
    if bytes.len() > MAX_BYTES {
        return Err(error("file exceeds 64 MiB input limit"));
    }
    let source = std::str::from_utf8(&bytes)
        .map_err(|_| error("expected ASCII/UTF-8 text with UTF-7 quoted strings"))?;
    let mut font = reader::read(source)?;
    font.extra.source = Some(fc_model::SourceProvenance {
        path: Some(path.to_string_lossy().into_owned()),
        format: "SFD".into(),
        glyph_order: font.glyph_order().cloned().collect(),
        ..Default::default()
    });
    Ok(font)
}

/// Write one master's native SFD data atomically after all validation succeeds.
///
/// Contextual/state-machine layout, device adjustments, color, bitmap and CID
/// structures are rejected. Mixed curve flavors are elevated exactly to cubic.
pub fn write_sfd(font: &Font, master: &MasterId, path: &Path) -> Result<(), Error> {
    if font.has_unapplied_feature_draft() {
        return Err(Error::UnappliedFeatureDraft);
    }
    if font.features.source_of_truth == FeaturesSource::ImportedTables {
        return Err(Error::ImportedLayoutRequiresConversion);
    }
    let master = font
        .master(master)
        .ok_or_else(|| Error::MissingMaster(master.to_string()))?;
    let text = writer::write(font, master)?;
    if text.len() > MAX_BYTES {
        return Err(error("serialized SFD exceeds 64 MiB limit"));
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temp =
        tempfile::NamedTempFile::new_in(parent).map_err(|error| Error::io(path, error))?;
    temp.write_all(text.as_bytes())
        .map_err(|error| Error::io(path, error))?;
    temp.as_file()
        .sync_all()
        .map_err(|error| Error::io(path, error))?;
    temp.persist(path)
        .map_err(|error| Error::io(path, error.error))?;
    Ok(())
}

fn error(reason: impl Into<String>) -> Error {
    Error::Source {
        format: "SFD",
        reason: reason.into(),
    }
}

fn number(text: &str) -> Result<f64, Error> {
    let value: f64 = text
        .parse()
        .map_err(|_| error(format!("invalid number {text:?}")))?;
    if !value.is_finite() || value.abs() > 1e12 {
        return Err(error(format!("nonfinite or out-of-range number {text:?}")));
    }
    Ok(value)
}

fn integer(text: &str) -> Result<i64, Error> {
    text.parse()
        .map_err(|_| error(format!("invalid integer {text:?}")))
}

fn finite(value: f64) -> Result<f64, Error> {
    if !value.is_finite() || value.abs() > 1e12 {
        Err(error("nonfinite or out-of-range geometry/metrics"))
    } else {
        Ok(value)
    }
}
