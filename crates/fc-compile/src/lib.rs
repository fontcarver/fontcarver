//! Font Cutter's static TrueType compilation and preservation pipeline.
//!
//! WOFF2 support is backed by `woofwoof`, which vendors Google's native WOFF2
//! implementation; building this crate therefore requires a C++11 compiler.

#![warn(missing_docs)]

mod error;
mod export;
mod large_font;
mod metadata;
mod preserve;
mod shaping;
mod source;
mod webfont;

pub use error::Error;
pub use export::{
    export, export_with_progress, touched_since_import, ExportFormat, ExportOptions, ExportReport,
    ExportStage, ExportWarning,
};
pub use preserve::preserve_merge;
pub use shaping::shaping_fingerprint;
pub use source::{compile_static, CompileOptions, FcSource};
pub use webfont::{wrap_woff1, wrap_woff2};
