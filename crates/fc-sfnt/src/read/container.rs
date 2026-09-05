use std::{borrow::Cow, fs, path::Path};

use read_fonts::FileRef;

use crate::Error;

/// Kind of supported sfnt container.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Container {
    /// A single raw sfnt face.
    Sfnt,
    /// A TrueType/OpenType collection.
    Ttc,
    /// A WOFF 1 web-font container.
    Woff,
    /// A WOFF 2 web-font container.
    Woff2,
}

/// Detects an sfnt container from its leading magic bytes.
#[must_use]
pub fn sniff(bytes: &[u8]) -> Option<Container> {
    match bytes.get(..4)? {
        b"\0\x01\0\0" | b"OTTO" | b"true" => Some(Container::Sfnt),
        b"ttcf" => Some(Container::Ttc),
        b"wOFF" => Some(Container::Woff),
        b"wOF2" => Some(Container::Woff2),
        _ => None,
    }
}

pub(crate) fn read_file(path: &Path) -> Result<Vec<u8>, Error> {
    fs::read(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })
}

pub(crate) fn decode(bytes: &[u8]) -> Result<(Container, Cow<'_, [u8]>), Error> {
    let container = sniff(bytes).ok_or(Error::UnsupportedContainer)?;
    let decoded = match container {
        Container::Sfnt | Container::Ttc => Cow::Borrowed(bytes),
        Container::Woff => {
            Cow::Owned(wuff::decompress_woff1(bytes).map_err(|error| Error::Parse {
                table: "WOFF",
                reason: format!("{error:?}"),
            })?)
        }
        Container::Woff2 => {
            Cow::Owned(wuff::decompress_woff2(bytes).map_err(|error| Error::Parse {
                table: "WOFF2",
                reason: format!("{error:?}"),
            })?)
        }
    };
    Ok((container, decoded))
}

pub(crate) fn decoded_face_count(bytes: &[u8]) -> Result<u32, Error> {
    match FileRef::new(bytes).map_err(|error| Error::parse("sfnt", error))? {
        FileRef::Font(_) => Ok(1),
        FileRef::Collection(collection) => Ok(collection.len()),
    }
}
