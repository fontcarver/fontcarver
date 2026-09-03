//! Reading sfnt and web-font containers into FontWorkshop's document model.

pub mod error;
mod read;

pub use error::Error;
pub use read::{
    collection_warning, face_count, face_names, read_bytes, read_path, sniff, Container,
    ReadOptions, ReadOutcome, Warning,
};
