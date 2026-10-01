//! Native outline table writers.

mod cff;
mod cff2;
mod charstring;
mod encoding;
mod hints;
mod subroutinize;

pub use cff::{write_cff, write_cff_with_subroutines, CffOutput};
pub use cff2::{
    cff2_outline, private_layout, write_cff2, Cff2Delta, Cff2Glyph, Cff2Outline, Cff2Private,
};

/// Invalid or unrepresentable input to an outline writer.
#[derive(Debug, thiserror::Error)]
#[error("cannot write CFF: {0}")]
pub struct WriteError(pub String);
