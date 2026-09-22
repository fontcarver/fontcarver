//! Reading sfnt and web-font containers into FontCarver's document model.

#![warn(missing_docs)]

pub mod bitmap;
pub mod error;
mod read;
pub mod tt;
pub mod write;

pub use error::Error;
pub use read::{
    collection_warning, face_count, face_names, read_bytes, read_path, sniff, Container,
    ReadOptions, ReadOutcome, Warning,
};
