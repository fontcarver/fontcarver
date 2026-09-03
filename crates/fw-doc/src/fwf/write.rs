use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

use flate2::{write::GzEncoder, Compression};
use fw_model::Font;

use crate::Error;

pub(super) fn write(path: &Path, font: &Font, document_path: Option<&Path>) -> Result<(), Error> {
    let envelope = super::envelope(font, document_path);
    let file = File::create(path)?;
    if is_gzip_path(path) {
        let encoder = GzEncoder::new(file, Compression::default());
        let mut writer = BufWriter::new(encoder);
        serde_json::to_writer_pretty(&mut writer, &envelope)?;
        writer.flush()?;
        let encoder = writer.into_inner().map_err(|error| error.into_error())?;
        encoder.finish()?;
    } else {
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, &envelope)?;
        writer.flush()?;
    }
    Ok(())
}

fn is_gzip_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.to_ascii_lowercase().ends_with(".fwf.gz"))
}
