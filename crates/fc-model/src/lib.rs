//! FontCarver's persistent document model, invariants, and read-only queries.

#![warn(missing_docs)]

mod anchor;
mod bitmap;
mod component;
mod contour;
pub mod error;
mod extra;
mod features;
mod font;
mod glyph;
mod guide;
mod ids;
mod info;
mod kerning;
mod layer;
mod master;
pub mod query;

pub use anchor::{Anchor, AnchorKind};
pub use bitmap::{BdfValue, Bitmap, BitmapGlyph, BitmapRef, BitmapStrike};
pub use component::Component;
pub use contour::{Contour, Point, PointKind, TtIndex};
pub use error::Error;
pub use extra::{Extra, SourceProvenance};
pub use features::{
    AssembledFea, FeaSourceLocation, FeaSourceMapping, FeatureBlock, FeatureClass, FeatureSourceId,
    FeatureSourceKind, Features, FeaturesSource,
};
pub use font::{Axis, CidInfo, DesignLocation, Font, Instance};
pub use glyph::{Glyph, MetricsKeys};
pub use guide::Guide;
pub use ids::{GlyphId, GlyphName, LayerId, MasterId, Tag};
pub use info::{FontInfo, GaspRecord, NamePlatform, NameRecord};
pub use kerning::{KernPair, KernSide, Kerning, KerningGroups};
pub use layer::{ColorLayer, DiagonalHint, HintMask, Image, Layer, LayerKind, PsHints, StemHint};
pub use master::{Master, MasterMetrics, PsPrivate, Stem, TtMasterData, Zone};
pub use query::{
    dependents, flatten_layer, flatten_layer_by_id, glyph_for_unicode, layer_bbox,
    masters_compatible, validate, Incompatibility, Issue,
};
