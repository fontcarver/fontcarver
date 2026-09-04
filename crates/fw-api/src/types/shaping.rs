//! Shaping API types.

use fw_doc::DocId;
use fw_model::MasterId;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub use fw_layout::{ShapedGlyph, ShapedRun};

/// Parameters for shaping one run of text with the document's own font.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ShapeParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Text to shape.
    pub text: String,
    /// Master to shape with, or the default master when absent.
    #[serde(default)]
    pub master: Option<MasterId>,
    /// Feature tags to force on or off.
    #[serde(default)]
    pub features: Vec<(String, bool)>,
    /// ISO 15924 script code, or automatic detection when absent.
    #[serde(default)]
    pub script: Option<String>,
    /// BCP 47 language tag, or the shaper default when absent.
    #[serde(default)]
    pub language: Option<String>,
    /// `ltr`, `rtl`, `ttb`, or `btt`; automatic detection when absent.
    #[serde(default)]
    pub direction: Option<String>,
    /// Rendering size; positions come back in font units when absent.
    #[serde(default)]
    pub size: Option<f64>,
}
