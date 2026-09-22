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
