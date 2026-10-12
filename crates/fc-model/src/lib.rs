//! FontCarver's persistent document model, invariants, and read-only queries.

#![warn(missing_docs)]

mod anchor;
mod bitmap;
mod color;
pub mod colrv1;
pub use colrv1::{ColrColorLine, ColrColorStop, ColrGraph, ColrPaint, ColrTransform};
mod component;
mod construction;
mod contour;
mod designspace;
mod effects;
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
mod node_links;
mod ps;
pub mod query;
mod smart;
pub mod svg;
mod tt;

pub use anchor::{Anchor, AnchorKind};
pub use bitmap::{
    validate_strike_property, BdfValue, Bitmap, BitmapGlyph, BitmapRef, BitmapStrike,
    MAX_BITMAP_BYTES, MAX_BITMAP_SIZE, MAX_FONT_BITMAP_BYTES, MAX_PROPERTY_NAME_CHARS,
    MAX_PROPERTY_VALUE_BYTES, MAX_STRIKE_PROPERTIES,
};
pub use color::{ColorPalette, ColorTable, ColrLayer, PaletteColor, MAX_COLOR_ITEMS};
pub use component::Component;
pub use construction::{ConstructionKind, GlyphConstruction, CONSTRUCTION_PREFIXES};
pub use contour::{Contour, Point, PointKind, TtIndex};
pub use effects::{
    validate_effect_mask, EffectBoolean, EffectCap, EffectJoin, EffectNib, EffectProgression,
    LayerEffect, LayerEffectKind, PenPoint, MAX_LAYER_EFFECTS,
};
pub use error::Error;
pub use extra::{Extra, SourceProvenance};
pub use feature_draft::{
    FeatureAnchorState, FeatureDraft, FeatureGlyphMembership, FeatureKerningState,
    FeatureMasterKerning, LayoutConversion,
};
pub use feature_generation::{
    FeatureGenerationSettings, FeatureGenerator, FeatureLigatureMapping, FeatureLocalizedMapping,
    FeatureSubstitution, FeatureWriterPolicy, GeneratedFeatureRecord, FEATURE_GENERATORS,
};
pub use features::{
    aalt_feature_order, automatic_class_registry, family_kind, family_names, generate_family,
    AssembledFea, FeaSourceLocation, FeaSourceMapping, FeatureBlock, FeatureClass, FeatureSourceId,
    FeatureSourceKind, FeatureTokenError, Features, FeaturesSource, GeneratedFamily,
    GeneratedItemKind, GeneratedRules, GenerationIssue, GENERATED_NAME_PREFIX,
};
pub use font::{Axis, CidInfo, DesignLocation, Font, Instance, VirtualMaster};
pub use glyph::{Glyph, MetricsKeys};
pub use guide::{Guide, GuideAnchor};
pub use ids::{GlyphId, GlyphName, LayerId, MasterId, Tag};
pub use info::{FontInfo, GaspRecord, InstanceOverrides, NamePlatform, NameRecord};
pub use kerning::{KernPair, KernSide, Kerning, KerningGroups};
pub use layer::{
    BracketCondition, ColorLayer, DiagonalHint, FlexHint, HintMask, Image, Layer, LayerKind,
    PsHints, StemHint,
};
pub use master::{Master, MasterMetrics, PsPrivate, Stem, TtMasterData, Zone};
pub use math::{
    MathAssembly, MathAssemblyLayout, MathConstants, MathConstruction, MathDevice, MathGlyphInfo,
    MathKern, MathKerns, MathPart, MathTable, MathValue, MathVariant, PlacedMathPart,
    MAX_MATH_ITEMS,
};
pub use node_links::{GuidedNode, NodeAxis, NodeLink, NodeLinkedGuide, NodeRef, ServantHandle};
pub use ps::FlexGeometry;
pub use query::{
    dependents, flatten_layer, flatten_layer_by_id, flatten_smart_component, glyph_for_unicode,
    layer_bbox, masters_compatible, validate, Incompatibility, Issue,
};
pub use smart::{smart_layer, SmartAxis};
pub use svg::{SvgDocument, SvgTable};
pub use tt::TtInstructionBinding;

pub use query::UnicodeIndex;
