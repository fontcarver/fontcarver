//! Strike metadata updates without sending the complete glyph bitmap collection.
use fc_doc::DocId;
use fc_model::BdfValue;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;
/// Replace strike metadata, retaining all stored glyphs and their pixels.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct BitmapConfigureParams {
    /// Open document.
    pub doc: DocId,
    /// Revision read by the metadata editor.
    #[ts(type = "number")]
    pub expected_generation: u64,
    /// Strike size.
    pub ppem: u16,
    /// Strike depth.
    pub depth: u8,
    /// Pixel ascent.
    pub ascent: i16,
    /// Pixel descent magnitude.
    pub descent: i16,
    /// BDF properties.
    pub properties: BTreeMap<String, BdfValue>,
}

/// One pixel grid for the document's units per em, with its strike metrics.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, TS)]
pub struct BitmapGridChoice {
    /// Pixels per em.
    pub ppem: u16,
    /// Font units covered by one pixel.
    pub units_per_pixel: f64,
    /// Whether one pixel is a whole number of font units.
    pub exact: bool,
    /// Pixel ascent a new strike at this size would take from the master.
    pub ascent: i16,
    /// Pixel descent magnitude a new strike at this size would take.
    pub descent: i16,
}

/// Ask for the exact pixel grids, and judge one requested size.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct BitmapGridParams {
    /// Open document.
    pub doc: DocId,
    /// Master whose vertical metrics scale into the strike, or the default.
    #[serde(default)]
    pub master: Option<fc_model::MasterId>,
    /// Size to judge; omitted returns only the exact grids.
    #[serde(default)]
    pub ppem: Option<u16>,
}

/// Exact pixel grids, plus the verdict and nearest exact size for a request.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct BitmapGridInfo {
    /// Observed document revision.
    #[ts(type = "number")]
    pub generation: u64,
    /// Units per em the grids were computed from.
    pub units_per_em: u16,
    /// Every size in the usable range whose pixel is a whole number of units.
    pub presets: Vec<BitmapGridChoice>,
    /// The requested size, exact or not; null when no size was requested.
    pub requested: Option<BitmapGridChoice>,
    /// Nearest exact size when the request was not exact.
    pub nearest: Option<u16>,
}
