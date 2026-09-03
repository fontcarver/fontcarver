use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    path::Path,
};

use flate2::read::GzDecoder;

use crate::Error;

pub(super) fn read(path: &Path) -> Result<super::ReadEnvelope, Error> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let is_gzip = reader
        .fill_buf()?
        .get(..2)
        .is_some_and(|magic| magic == [0x1f, 0x8b]);
    let envelope = if is_gzip {
        from_reader(GzDecoder::new(reader))?
    } else {
        from_reader(reader)?
    };
    super::validate(envelope)
}

fn from_reader(reader: impl Read) -> Result<super::ReadEnvelope, Error> {
    serde_json::from_reader(reader).map_err(Error::from)
}
