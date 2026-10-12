//! Embedded bitmap decoding, writing and table replacement.
mod png_codec;
mod read;
mod write;
pub use read::read_strikes;
pub use write::replace_tables;

use crate::Error;
fn error(message: impl Into<String>) -> Error {
    Error::Parse {
        table: "bitmap",
        reason: message.into(),
    }
}
fn checked<T>(value: Result<T, fc_model::Error>) -> Result<T, Error> {
    value.map_err(|e| error(e.to_string()))
}
