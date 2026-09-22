//! Native outline table writers.

mod cff;
mod cff2;
mod charstring;
mod encoding;

pub use cff::{write_cff, CffOutput};
pub use cff2::{cff2_outline, write_cff2, Cff2Delta, Cff2Glyph, Cff2Outline};

/// Invalid or unrepresentable input to an outline writer.
#[derive(Debug, thiserror::Error)]
#[error("cannot write CFF: {0}")]
pub struct WriteError(pub String);
