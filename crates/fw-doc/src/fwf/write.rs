use std::{
    ffi::OsString,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use flate2::{write::GzEncoder, Compression};
use fw_model::Font;

use crate::Error;

pub(super) fn write(path: &Path, font: &Font, document_path: Option<&Path>) -> Result<(), Error> {
    let envelope = super::envelope(font, document_path);
    let temporary_path = temporary_path(path);
    if let Err(error) = write_temporary(&temporary_path, path, &envelope) {
        let _ = fs::remove_file(&temporary_path);
        return Err(error);
    }
    if let Err(error) = fs::rename(&temporary_path, path) {
        let _ = fs::remove_file(&temporary_path);
        return Err(error.into());
    }
    Ok(())
}

fn write_temporary(
    temporary_path: &Path,
    target_path: &Path,
    envelope: &super::WriteEnvelope<'_>,
) -> Result<(), Error> {
    let file = File::create(temporary_path)?;
    if is_gzip_path(target_path) {
        let encoder = GzEncoder::new(file, Compression::default());
        let mut writer = BufWriter::new(encoder);
        serde_json::to_writer_pretty(&mut writer, envelope)?;
        writer.flush()?;
        let encoder = writer.into_inner().map_err(|error| error.into_error())?;
        encoder.finish()?.sync_all()?;
    } else {
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, envelope)?;
        writer.flush()?;
        writer
            .into_inner()
            .map_err(|error| error.into_error())?
            .sync_all()?;
    }
    Ok(())
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut path_with_suffix = OsString::from(path.as_os_str());
    path_with_suffix.push(".tmp");
    PathBuf::from(path_with_suffix)
}

fn is_gzip_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.to_ascii_lowercase().ends_with(".fwf.gz"))
}
