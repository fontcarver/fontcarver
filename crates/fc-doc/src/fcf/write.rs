use std::{
    ffi::OsString,
    fs::{self, File},
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
};

use fc_model::Font;
use flate2::{write::GzEncoder, Compression};
use serde::Serialize;
use serde_json::ser::Formatter;

use crate::Error;

pub(super) fn write(path: &Path, font: &Font, document_path: Option<&Path>) -> Result<(), Error> {
    for master in &font.masters {
        if master
            .kerning_h
            .0
            .values()
            .chain(master.kerning_v.0.values())
            .any(|value| !value.is_finite())
        {
            return Err(Error::NonFiniteKerning(master.id.clone()));
        }
    }
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
        write_json(&mut writer, envelope)?;
        writer.flush()?;
        let encoder = writer.into_inner().map_err(|error| error.into_error())?;
        encoder.finish()?.sync_all()?;
    } else {
        let mut writer = BufWriter::new(file);
        write_json(&mut writer, envelope)?;
        writer.flush()?;
        writer
            .into_inner()
            .map_err(|error| error.into_error())?
            .sync_all()?;
    }
    Ok(())
}

fn write_json(writer: impl Write, envelope: &super::WriteEnvelope<'_>) -> Result<(), Error> {
    let mut serializer =
        serde_json::Serializer::with_formatter(writer, PerGlyphLineFormatter::default());
    envelope.serialize(&mut serializer)?;
    Ok(())
}

#[derive(Default)]
struct Container {
    has_value: bool,
}

/// Pretty-prints JSON except inside each direct value of `font.glyphs`.
#[derive(Default)]
struct PerGlyphLineFormatter {
    containers: Vec<Container>,
    object_key: String,
    capturing_key: bool,
    next_object_is_glyph_map: bool,
    glyph_map_depth: Option<usize>,
    compact_value_depth: Option<usize>,
}

impl PerGlyphLineFormatter {
    fn compact(&self) -> bool {
        self.compact_value_depth.is_some()
    }

    fn begin_container<W>(&mut self, writer: &mut W, opening: u8) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        writer.write_all(&[opening])?;
        self.containers.push(Container::default());
        if self.next_object_is_glyph_map && opening == b'{' {
            self.next_object_is_glyph_map = false;
            self.glyph_map_depth = Some(self.containers.len());
        }
        Ok(())
    }

    fn end_container<W>(&mut self, writer: &mut W, closing: u8) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        let ended_depth = self.containers.len();
        let compact = self.compact();
        let Some(container) = self.containers.pop() else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "JSON formatter closed an unopened container",
            ));
        };
        if !compact && container.has_value {
            writer.write_all(b"\n")?;
            write_indent(writer, self.containers.len())?;
        }
        writer.write_all(&[closing])?;
        if self.compact_value_depth == Some(ended_depth) {
            self.compact_value_depth = None;
        }
        if self.glyph_map_depth == Some(ended_depth) {
            self.glyph_map_depth = None;
        }
        Ok(())
    }

    fn begin_value<W>(&self, writer: &mut W, first: bool) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        if self.compact() {
            if !first {
                writer.write_all(b",")?;
            }
        } else {
            writer.write_all(if first { b"\n" } else { b",\n" })?;
            write_indent(writer, self.containers.len())?;
        }
        Ok(())
    }

    fn mark_value(&mut self) {
        if let Some(container) = self.containers.last_mut() {
            container.has_value = true;
        }
    }
}

impl Formatter for PerGlyphLineFormatter {
    fn begin_array<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.begin_container(writer, b'[')
    }

    fn end_array<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.end_container(writer, b']')
    }

    fn begin_array_value<W>(&mut self, writer: &mut W, first: bool) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.begin_value(writer, first)
    }

    fn end_array_value<W>(&mut self, _writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.mark_value();
        Ok(())
    }

    fn begin_object<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.begin_container(writer, b'{')
    }

    fn end_object<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.end_container(writer, b'}')
    }

    fn begin_object_key<W>(&mut self, writer: &mut W, first: bool) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.begin_value(writer, first)?;
        self.object_key.clear();
        self.capturing_key = true;
        Ok(())
    }

    fn end_object_key<W>(&mut self, _writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.capturing_key = false;
        if self.containers.len() == 2 && self.object_key == "glyphs" {
            self.next_object_is_glyph_map = true;
        }
        Ok(())
    }

    fn begin_object_value<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        writer.write_all(if self.compact() { b":" } else { b": " })?;
        if self.glyph_map_depth == Some(self.containers.len()) {
            self.compact_value_depth = Some(self.containers.len() + 1);
        }
        Ok(())
    }

    fn end_object_value<W>(&mut self, _writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        self.mark_value();
        Ok(())
    }

    fn write_string_fragment<W>(&mut self, writer: &mut W, fragment: &str) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        if self.capturing_key {
            self.object_key.push_str(fragment);
        }
        writer.write_all(fragment.as_bytes())
    }
}

fn write_indent(writer: &mut (impl Write + ?Sized), depth: usize) -> io::Result<()> {
    for _ in 0..depth {
        writer.write_all(b"  ")?;
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
        .map(str::to_ascii_lowercase)
        // `.fwf.gz` is the pre-rename spelling and still writes gzip.
        .is_some_and(|name| name.ends_with(".fcf.gz") || name.ends_with(".fwf.gz"))
}
