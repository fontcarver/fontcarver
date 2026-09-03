//! Document sessions, commands, and persistence for FontWorkshop.

#![warn(missing_docs)]

pub mod autosave;
mod cache;
pub mod command;
pub mod document;
pub mod error;
pub mod fwf;
pub mod session;
pub mod undo;

pub use command::{Change, Command};
pub use document::{Document, DocumentEvent, EditToken};
pub use error::Error;
pub use session::{DefaultFontOpener, DocId, FontOpener, Session};
pub use undo::{Snapshot, UndoEntry, UndoStack};
