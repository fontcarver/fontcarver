//! Per-glyph text projects committed by an atomic manifest replacement.

use std::{
    collections::HashSet,
    fs::{self, File},
    io,
    path::Path,
};

use fc_model::{Font, Glyph, GlyphName};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

use super::project_io::{atomic_write, lock_project, require_directory, require_file, sync_dir};
use crate::Error;

const FORMAT: &str = "fontcarver-project";
const VERSION: u32 = 1;
const MANIFEST: &str = "project.json";

#[derive(Serialize, Deserialize)]
struct Manifest {
    format: String,
    version: u32,
    font: Font,
    glyph_files: Vec<GlyphFile>,
}

#[derive(Serialize, Deserialize)]
struct GlyphFile {
    name: GlyphName,
    file: String,
}

pub(super) fn read(path: &Path) -> Result<Font, Error> {
    require_directory(path)?;
    let _lock = lock_project(path, false)?;
    let mut manifest = read_manifest(path)?;
    require_directory(&path.join("glyphs"))?;
    for entry in manifest.glyph_files {
        let glyph_path = path.join("glyphs").join(&entry.file);
        require_file(&glyph_path)?;
        let glyph: Glyph = serde_json::from_reader(File::open(glyph_path)?)?;
        if glyph.name != entry.name {
            return Err(invalid("glyph name differs from its manifest entry"));
        }
        manifest.font.glyphs.insert(entry.name, glyph);
    }
    manifest.font.validate_bitmaps()?;
    manifest.font.validate_effects()?;
    Ok(manifest.font)
}

pub(super) fn write(path: &Path, font: &Font) -> Result<(), Error> {
    super::write::validate_kerning(font)?;
    for (name, glyph) in &font.glyphs {
        if name != &glyph.name {
            return Err(invalid("glyph name differs from its document key"));
        }
    }
    if !path.exists() {
        fs::create_dir(path)?;
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            sync_dir(parent)?;
        }
    }
    require_directory(path)?;
    let _lock = lock_project(path, true)?;
    let previous = match fs::symlink_metadata(path.join(MANIFEST)) {
        Ok(_) => Some(read_manifest(path)?),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    let glyph_dir = path.join("glyphs");
    if !glyph_dir.exists() {
        fs::create_dir(&glyph_dir)?;
    }
    require_directory(&glyph_dir)?;
    let mut glyph_files = Vec::with_capacity(font.glyphs.len());
    for (name, glyph) in &font.glyphs {
        let bytes = json_bytes(glyph)?;
        let file = format!(
            "{:x}-{:x}.json",
            Sha1::digest(name.as_bytes()),
            Sha1::digest(&bytes)
        );
        let glyph_path = glyph_dir.join(&file);
        match fs::symlink_metadata(&glyph_path) {
            Ok(_) => {
                require_file(&glyph_path)?;
                if fs::read(&glyph_path)? != bytes {
                    return Err(invalid("existing glyph revision has different contents"));
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                atomic_write(&glyph_path, &bytes)?;
            }
            Err(error) => return Err(error.into()),
        }
        glyph_files.push(GlyphFile {
            name: name.clone(),
            file,
        });
    }
    // New revision files are durable before the manifest can refer to them.
    sync_dir(&glyph_dir)?;
    let mut metadata = font.clone();
    metadata.glyphs.clear();
    let manifest = Manifest {
        format: FORMAT.into(),
        version: VERSION,
        font: metadata,
        glyph_files,
    };
    atomic_write(&path.join(MANIFEST), &json_bytes(&manifest)?)?;
    // Publication has succeeded. A post-commit sync or pruning failure must not
    // falsely report that the document was left unsaved.
    if let Err(error) = sync_dir(path) {
        tracing::warn!(%error, path = %path.display(), "could not sync project directory after save");
        // Retain old revisions if manifest durability is uncertain.
        return Ok(());
    }
    if let Some(previous) = previous {
        prune_previous(&glyph_dir, &previous, &manifest);
    }
    Ok(())
}

fn read_manifest(path: &Path) -> Result<Manifest, Error> {
    let manifest_path = path.join(MANIFEST);
    require_file(&manifest_path)?;
    let manifest: Manifest = serde_json::from_reader(File::open(manifest_path)?)?;
    if manifest.format != FORMAT {
        return Err(Error::InvalidFcfFormat(manifest.format));
    }
    if manifest.version != VERSION {
        return Err(Error::UnsupportedFcfVersion(manifest.version));
    }
    if !manifest.font.glyphs.is_empty() {
        return Err(invalid("project metadata must not embed glyphs"));
    }
    let mut names = HashSet::new();
    let mut files = HashSet::new();
    for entry in &manifest.glyph_files {
        if !valid_filename(&entry.file) {
            return Err(invalid("project glyph filename has an invalid format"));
        }
        if !names.insert(&entry.name) || !files.insert(&entry.file) {
            return Err(invalid("project has duplicate glyph names or files"));
        }
    }
    Ok(manifest)
}

fn valid_filename(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.len() == 86
        && bytes[40] == b'-'
        && &bytes[81..] == b".json"
        && bytes[..40]
            .iter()
            .chain(&bytes[41..81])
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
}

fn prune_previous(dir: &Path, previous: &Manifest, current: &Manifest) {
    let keep: HashSet<_> = current
        .glyph_files
        .iter()
        .map(|entry| &entry.file)
        .collect();
    for entry in &previous.glyph_files {
        if keep.contains(&entry.file) {
            continue;
        }
        let path = dir.join(&entry.file);
        // Only files explicitly owned by the previous, validated manifest are
        // eligible. Never traverse or remove unrelated files or symlinks.
        if require_file(&path).is_ok() {
            if let Err(error) = fs::remove_file(&path) {
                tracing::warn!(%error, path = %path.display(), "could not prune obsolete project glyph");
            }
        }
    }
}

fn json_bytes(value: &impl Serialize) -> Result<Vec<u8>, Error> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn invalid(message: &str) -> Error {
    io::Error::new(io::ErrorKind::InvalidData, message).into()
}
