//! Document sessions, commands, and persistence for Font Cutter.

#![warn(missing_docs)]

pub mod autosave;
mod cache;
pub mod command;
pub mod document;
pub mod double_option;
pub mod edit;
pub mod error;
pub mod fcf;
pub mod metrics;
pub mod session;
pub mod undo;

pub use command::{
    Change, Command, CurveKind, DeleteMode, HandleMode, Nib, PathOp, PointRef, SegmentKind,
    SimplifyOptions, StrokeCap, StrokeJoin, StrokeOptions,
};
pub use document::{Document, DocumentEvent, EditToken};
pub use error::Error;
pub use metrics::{MetricsSource, ResolvedMetrics};
pub use session::{DefaultFontOpener, DocId, FontOpener, Session};
pub use undo::{Snapshot, UndoEntry, UndoStack};
