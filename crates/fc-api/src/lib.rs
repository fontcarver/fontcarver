//! Typed kernel API contracts and dispatch for FontCarver frontends.

#![warn(missing_docs)]

mod api;
pub mod error;
mod handlers;
pub mod types;

pub use api::{Api, COMPILE_STAGE, FAILED_STAGE, METHODS, PRESERVE_STAGE, WRITE_STAGE};
pub use error::ApiError;
pub use fc_doc::{Change, Command, DocId, DocumentEvent, ResolvedMetrics};
pub use fc_model::{FontInfo, Glyph, GlyphId, LayerId, Master, MasterId};
pub use types::*;
