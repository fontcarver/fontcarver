use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    path::Path,
};

use flate2::read::GzDecoder;

use crate::Error;

pub(super) fn read(path: &Path) -> Result<super::ReadEnvelope, Error> {
    read_envelope(path)
}

pub(super) fn read_metadata(path: &Path) -> Result<super::MetadataEnvelope, Error> {
    read_envelope(path)
}

fn read_envelope<T>(path: &Path) -> Result<T, Error>
where
    T: serde::de::DeserializeOwned,
{
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let is_gzip = reader
        .fill_buf()?
        .get(..2)
        .is_some_and(|magic| magic == [0x1f, 0x8b]);
    if is_gzip {
        from_reader(GzDecoder::new(reader))
    } else {
        from_reader(reader)
    }
}

fn from_reader<T>(reader: impl Read) -> Result<T, Error>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_reader(reader).map_err(Error::from)
}
