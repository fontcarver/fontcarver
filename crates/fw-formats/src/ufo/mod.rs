//! Unified Font Object and designspace import/export.

use std::path::{Path, PathBuf};

use fw_model::{Font, MasterId};

use crate::Error;

mod convert;
mod designspace;
mod extra;
mod info;
mod metadata;
mod value;
mod writer;

/// Options controlling UFO serialization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UfoWriteOptions {
    /// Compare generated files with existing files and leave identical files untouched.
    pub incremental: bool,
}

impl Default for UfoWriteOptions {
    fn default() -> Self {
        Self { incremental: true }
    }
}

/// Reads one UFO as a single-master FontWorkshop document.
pub fn read_ufo(path: &Path) -> Result<Font, Error> {
    let source = norad::Font::load_requested_data(path, norad::DataRequest::all())?;
    convert::from_norad(&source, path)
}

/// Reads a designspace and all of its UFO sources.
pub fn read_designspace(path: &Path) -> Result<Font, Error> {
    designspace::read(path)
}

/// Writes one master as a UFO 3 package without deleting the destination directory.
pub fn write_ufo(
    font: &Font,
    master: &MasterId,
    dir: &Path,
    opts: &UfoWriteOptions,
) -> Result<(), Error> {
    let master = font
        .master(master)
        .ok_or_else(|| Error::MissingMaster(master.to_string()))?;
    let ufo = convert::to_norad(font, master)?;
    writer::write(&ufo, dir, opts)
}

/// Writes one UFO per master and a `<family>.designspace` document.
pub fn write_designspace(font: &Font, dir: &Path) -> Result<PathBuf, Error> {
    designspace::write(font, dir)
}
