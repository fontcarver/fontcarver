//! Native outline table writers.

mod cff;
mod charstring;
mod encoding;

pub use cff::{write_cff, CffOutput};

/// Invalid or unrepresentable input to an outline writer.
#[derive(Debug, thiserror::Error)]
#[error("cannot write CFF: {0}")]
pub struct WriteError(pub String);
