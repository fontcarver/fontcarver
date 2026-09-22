//! PostScript hint analysis and bounded TrueType program authoring.

#![warn(missing_docs)]

mod error;
pub mod external;
pub mod ps;
pub mod tt;

pub use error::Error;
