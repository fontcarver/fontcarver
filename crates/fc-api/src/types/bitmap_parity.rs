//! Bounded bitmap conversion and reconstruction workflows.
use fc_doc::DocId;
use fc_model::MasterId;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Pixel filter used when a copy between strikes changes the pixel size.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, TS)]
pub enum BitmapScaleFilter {
    /// Take the source sample under each destination pixel's centre.
    #[default]
    Nearest,
    /// Average the source over each destination pixel's exact footprint.
    AreaAverage,
}

impl From<BitmapScaleFilter> for fc_raster::BitmapScaleFilter {
    fn from(value: BitmapScaleFilter) -> Self {
        match value {
            BitmapScaleFilter::Nearest => Self::Nearest,
            BitmapScaleFilter::AreaAverage => Self::AreaAverage,
        }
    }
}

/// Copy flattened glyph pixels into another strike, optionally scaling to its ppem.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct BitmapTransferParams {
    /// Open document.
    pub doc: DocId,
    /// Revision read before the operation.
    #[ts(type = "number")]
    pub expected_generation: u64,
    /// Source strike size.
    pub source_ppem: u16,
    /// Source strike depth.
    pub source_depth: u8,
    /// Destination strike size; an absent strike is created.
    pub target_ppem: u16,
    /// Destination coverage or RGBA depth.
    pub target_depth: u8,
    /// Glyph names; empty selects all images in the source strike.
    pub glyphs: Vec<String>,
    /// Scale pixels, origins and metrics by target/source ppem.
    pub scale: bool,
    /// Pixel filter used while scaling; omitted means nearest neighbour.
    #[serde(default)]
    #[ts(optional)]
    pub filter: Option<BitmapScaleFilter>,
}

/// Replace outline pixels with references to a persistent editable pixel glyph.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct BitmapPixelComponentsParams {
    /// Open document.
    pub doc: DocId,
    /// Revision read before the operation.
    #[ts(type = "number")]
    pub expected_generation: u64,
    /// Source strike size.
    pub ppem: u16,
    /// Source strike depth.
    pub depth: u8,
    /// Destination master, or the default master.
    pub master: Option<MasterId>,
    /// Glyph names; empty selects all images in the source strike.
    pub glyphs: Vec<String>,
    /// Minimum coverage, 1..255.
    pub threshold: u8,
    /// Name of the reusable glyph, whose UPM-square outline represents one pixel.
    pub pixel_glyph: String,
}

/// Detect an exact orthogonal pixel grid in source outlines.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct BitmapDetectParams {
    /// Open document.
    pub doc: DocId,
    /// Master to inspect, or the default master.
    pub master: Option<MasterId>,
}

/// Conservative grid-detection result; no candidate means no supported pixel evidence.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct BitmapPixelDetection {
    /// Observed document revision.
    #[ts(type = "number")]
    pub generation: u64,
    /// Detected strike size.
    pub ppem: Option<u16>,
    /// Font units per detected pixel.
    pub units_per_pixel: Option<f64>,
    /// Number of distinct staircase glyphs supporting the candidate.
    pub evidence_glyphs: u32,
}

/// Reconstruct a monochrome strike only when supported pixel-grid evidence exists.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct BitmapReconstructParams {
    /// Open document.
    pub doc: DocId,
    /// Revision read by detection UI.
    #[ts(type = "number")]
    pub expected_generation: u64,
    /// Master to inspect, or the default master.
    pub master: Option<MasterId>,
    /// Selected glyph names; empty reconstructs all outlines.
    pub glyphs: Vec<String>,
}
