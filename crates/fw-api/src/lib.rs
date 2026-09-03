//! Typed kernel API contracts and dispatch for FontWorkshop frontends.

#![warn(missing_docs)]

mod api;
pub mod error;
mod handlers;
pub mod types;

pub use api::{Api, METHODS};
pub use error::ApiError;
pub use fw_doc::{Change, Command, DocId, DocumentEvent};
pub use fw_model::{FontInfo, Glyph, GlyphId, LayerId, Master, MasterId};
pub use types::*;
