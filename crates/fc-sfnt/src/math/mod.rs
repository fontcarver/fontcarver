//! Bounded semantic MATH 1.0 import and export.
//!
//! Implemented from the OpenType MATH specification. Glyph references are
//! resolved through persistent names; no imported numeric IDs are retained.

mod read;
mod write;
pub use read::read_table;
pub use write::write_table;

use crate::Error;

const MAX_TABLE_BYTES: usize = 16 * 1024 * 1024;

fn error(message: impl Into<String>) -> Error {
    Error::Parse {
        table: "MATH",
        reason: message.into(),
    }
}
