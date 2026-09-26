//! FontCarver's static TrueType compilation and preservation pipeline.
//!
//! WOFF2 support is backed by `woofwoof`, which vendors Google's native WOFF2
//! implementation; building this crate therefore requires a C++11 compiler.

#![warn(missing_docs)]

mod bracket;
mod color;
mod error;
mod export;
mod feature_generation;
mod feature_validation;
mod instance;
mod kerning;
mod large_font;
mod math;
mod metadata;
mod otf;
mod otf_preservation;
mod otf_preservation_aat;
mod otf_vertical;
mod preprocess;
mod preserve;
mod shaping;
mod source;
mod tt;
mod variable;
mod variable_cff;
mod vertical;
mod webfont;

pub use error::Error;
pub use export::{
    export, export_with_progress, touched_since_import, ExportFormat, ExportOptions, ExportReport,
    ExportStage, ExportWarning,
};
pub use feature_generation::{
    freeze_feature_source, generate_feature_source, generation_is_stale, GeneratedFeatureSource,
};
pub use feature_validation::{
    validate_feature_candidate, FeatureDiagnostic, FeatureDiagnosticSeverity,
    FeatureDiagnosticSpan, FeatureValidation,
};
pub use instance::instantiate;
pub use kerning::horizontal_kerning_warnings;
pub use preprocess::prepare_postscript_export;
pub use preserve::preserve_merge;
pub use shaping::shaping_fingerprint;
pub use source::{
    compile_static, compile_static_with_metadata, compile_variable, CompileOptions, FcSource,
    StaticCompile,
};
pub use webfont::{wrap_woff1, wrap_woff2};

mod instance_metrics;
