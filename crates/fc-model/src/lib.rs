//! FontCarver's persistent document model, invariants, and read-only queries.

#![warn(missing_docs)]

mod anchor;
mod bitmap;
mod color;
pub mod colrv1;
pub use colrv1::{ColrColorLine, ColrColorStop, ColrGraph, ColrPaint, ColrTransform};
mod component;
mod contour;
mod designspace;
pub mod error;
mod extra;
mod feature_draft;
mod feature_generation;
mod features;
mod font;
mod glyph;
mod guide;
mod ids;
mod info;
mod kerning;
mod layer;
mod master;
mod math;
mod ps;
pub mod query;
mod smart;
pub mod svg;
mod tt;

pub use anchor::{Anchor, AnchorKind};
pub use bitmap::{
    BdfValue, Bitmap, BitmapGlyph, BitmapRef, BitmapStrike, MAX_BITMAP_BYTES, MAX_BITMAP_SIZE,
    MAX_FONT_BITMAP_BYTES,
};
pub use color::{ColorPalette, ColorTable, ColrLayer, PaletteColor, MAX_COLOR_ITEMS};
pub use component::Component;
pub use contour::{Contour, Point, PointKind, TtIndex};
pub use error::Error;
pub use extra::{Extra, SourceProvenance};
pub use feature_draft::{
    FeatureAnchorState, FeatureDraft, FeatureGlyphMembership, FeatureKerningState,
    FeatureMasterKerning, LayoutConversion,
};
pub use feature_generation::{
    FeatureGenerationSettings, FeatureGenerator, FeatureLigatureMapping, FeatureLocalizedMapping,
    FeatureSubstitution, FeatureWriterPolicy, GeneratedFeatureRecord,
};
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
pub use math::{
    MathAssembly, MathAssemblyLayout, MathConstants, MathConstruction, MathDevice, MathGlyphInfo,
    MathKern, MathKerns, MathPart, MathTable, MathValue, MathVariant, PlacedMathPart,
    MAX_MATH_ITEMS,
};
pub use query::{
    dependents, flatten_layer, flatten_layer_by_id, flatten_smart_component, glyph_for_unicode,
    layer_bbox, masters_compatible, validate, Incompatibility, Issue,
};
pub use smart::{smart_layer, SmartAxis};
pub use svg::{SvgDocument, SvgTable};
pub use tt::TtInstructionBinding;

pub use query::UnicodeIndex;
