//! Document-relative source image paths and relocation during native saves.

use std::path::{Component, Path, PathBuf};

use fc_model::{Font, Glyph};

use crate::Error;

/// Resolves a linked image without requiring the image file to exist.
///
/// Native documents use their containing directory; UFO packages use `images/`.
pub fn resolve_image_path(document_path: Option<&Path>, file_name: &str) -> Result<PathBuf, Error> {
    if file_name.is_empty() {
        return Err(Error::InvalidCommand(
            "source image has no file name".into(),
        ));
    }
    let image = Path::new(file_name);
    if image.is_absolute() {
        return absolute_path(image);
    }
    let document = document_path.ok_or_else(|| {
        Error::InvalidCommand("save the document before linking a source image".into())
    })?;
    absolute_path(&image_directory(document)?.join(image))
}

/// Converts an image's filesystem path to a link relative to its document.
pub fn relative_image_path(document_path: &Path, source_path: &Path) -> Result<String, Error> {
    let base = image_directory(document_path)?;
    let source = absolute_path(source_path)?;
    let base_parts = base.components().collect::<Vec<_>>();
    let source_parts = source.components().collect::<Vec<_>>();
    let common = base_parts
        .iter()
        .zip(&source_parts)
        .take_while(|(left, right)| left == right)
        .count();
    if common == 0 {
        return Err(Error::InvalidCommand(
            "source image and document must be on the same filesystem volume".into(),
        ));
    }
    let mut result = PathBuf::new();
    for _ in common..base_parts.len() {
        result.push("..");
    }
    for part in &source_parts[common..] {
        result.push(part.as_os_str());
    }
    result
        .to_str()
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| Error::InvalidCommand("source image path is not a valid file name".into()))
}

/// Keeps every linked image pointing at the same file after relocating a document.
/// Embedded image names and documents without an existing path are unchanged.
pub fn rebase_image_paths(
    font: &mut Font,
    old_path: Option<&Path>,
    new_path: &Path,
) -> Result<(), Error> {
    if !relocates_images(old_path, new_path)? {
        return Ok(());
    }
    let Some(old_path) = old_path else {
        return Ok(());
    };
    for glyph in font.glyphs.values_mut() {
        rebase_glyph_images(glyph, old_path, new_path)?;
    }
    Ok(())
}

pub(crate) fn relocates_images(old_path: Option<&Path>, new_path: &Path) -> Result<bool, Error> {
    match old_path {
        Some(old_path) => Ok(image_directory(old_path)? != image_directory(new_path)?),
        None => Ok(false),
    }
}

pub(crate) fn rebase_glyph_images(
    glyph: &mut Glyph,
    old_path: &Path,
    new_path: &Path,
) -> Result<(), Error> {
    for image in glyph
        .layers
        .iter_mut()
        .filter_map(|layer| layer.background_image.as_mut())
    {
        if image.data.is_none() {
            let source = resolve_image_path(Some(old_path), &image.file_name)?;
            image.file_name = relative_image_path(new_path, &source)?;
        }
    }
    Ok(())
}

fn image_directory(document_path: &Path) -> Result<PathBuf, Error> {
    let path = absolute_path(document_path)?;
    if path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("ufo"))
    {
        Ok(path.join("images"))
    } else {
        path.parent().map(Path::to_path_buf).ok_or_else(|| {
            Error::InvalidCommand("document path has no containing directory".into())
        })
    }
}

fn absolute_path(path: &Path) -> Result<PathBuf, Error> {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            component => result.push(component.as_os_str()),
        }
    }
    Ok(result)
}
