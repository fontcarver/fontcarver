//! Saved project filters and the bounded smart-filter expression tree.

use fc_doc::DocId;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Which glyph metadata participates in text search.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub enum GlyphQueryScope {
    /// Search names, Unicode scalars and notes.
    #[default]
    All,
    /// Search only the glyph name.
    Name,
    /// Search assigned Unicode characters and hexadecimal scalar notation.
    Unicode,
    /// Search the glyph note.
    Note,
}

/// What kind of literal a smart-filter field compares against.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SmartFilterFieldKind {
    /// Text, compared with `eq`, `ne`, `contains` or a bounded regex.
    Text,
    /// A finite number, compared with the ordering operators.
    Number,
    /// A Unicode scalar, written `U+0041` or as a decimal number.
    Scalar,
    /// `true` or `false`.
    Boolean,
}

/// One predicate the smart-filter grammar accepts, by its canonical name.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct SmartFilterField {
    /// Canonical field name, as the parser and the builder both spell it.
    pub field: &'static str,
    /// Which literals and operators the field accepts.
    pub kind: SmartFilterFieldKind,
}

/// The complete predicate vocabulary, in the order the builder offers it.
///
/// One list serves the parser, the evaluator and the visual builder: the
/// frontend's own table is generated from this one, so a field can never exist
/// on one side of the boundary and not the other.
pub const SMART_FILTER_FIELDS: &[SmartFilterField] = &[
    field("name", SmartFilterFieldKind::Text),
    field("paths", SmartFilterFieldKind::Number),
    field("components", SmartFilterFieldKind::Number),
    field("tags", SmartFilterFieldKind::Text),
    field("script", SmartFilterFieldKind::Text),
    field("category", SmartFilterFieldKind::Text),
    field("subcategory", SmartFilterFieldKind::Text),
    field("case", SmartFilterFieldKind::Text),
    field("master_compatible", SmartFilterFieldKind::Boolean),
    field("export", SmartFilterFieldKind::Boolean),
    field("has_unicode", SmartFilterFieldKind::Boolean),
    field("has_components", SmartFilterFieldKind::Boolean),
    field("contains_component", SmartFilterFieldKind::Text),
    field("has_hints", SmartFilterFieldKind::Boolean),
    field("has_ps_hints", SmartFilterFieldKind::Boolean),
    field("has_tt_hints", SmartFilterFieldKind::Boolean),
    field("has_corners", SmartFilterFieldKind::Boolean),
    field("has_anchors", SmartFilterFieldKind::Boolean),
    field("has_special_layers", SmartFilterFieldKind::Boolean),
    field("has_custom_glyph_info", SmartFilterFieldKind::Boolean),
    field("has_annotations", SmartFilterFieldKind::Boolean),
    field("is_auto_aligned", SmartFilterFieldKind::Boolean),
    field("has_metrics_keys", SmartFilterFieldKind::Boolean),
    field("metrics_out_of_sync", SmartFilterFieldKind::Boolean),
    field("has_kerning_groups", SmartFilterFieldKind::Boolean),
    field("color_label", SmartFilterFieldKind::Number),
    field("layer_color_label", SmartFilterFieldKind::Number),
    field("is_hangul_key", SmartFilterFieldKind::Boolean),
    field("note", SmartFilterFieldKind::Text),
    field("unicode", SmartFilterFieldKind::Scalar),
    field("changed", SmartFilterFieldKind::Boolean),
    field("locked", SmartFilterFieldKind::Boolean),
    field("is_placeholder", SmartFilterFieldKind::Boolean),
    field("width", SmartFilterFieldKind::Number),
    field("block", SmartFilterFieldKind::Text),
];

const fn field(field: &'static str, kind: SmartFilterFieldKind) -> SmartFilterField {
    SmartFilterField { field, kind }
}

/// Comparison applied by a smart-filter predicate.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SmartFilterOperator {
    /// Equal to the value.
    Eq,
    /// Not equal to the value.
    Ne,
    /// Contains the literal text.
    Contains,
    /// Matches a bounded regular expression.
    Regex,
    /// Greater than the numeric value.
    Gt,
    /// Greater than or equal to the numeric value.
    Gte,
    /// Less than the numeric value.
    Lt,
    /// Less than or equal to the numeric value.
    Lte,
}

/// Recursive expression shared by the text editor and visual builder.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum SmartFilterNode {
    /// Every child must match.
    All {
        /// Nested rules.
        children: Vec<SmartFilterNode>,
    },
    /// At least one child must match.
    Any {
        /// Nested rules.
        children: Vec<SmartFilterNode>,
    },
    /// No child may match.
    None {
        /// Nested rules.
        children: Vec<SmartFilterNode>,
    },
    /// A comparison of a named glyph property.
    Predicate {
        /// Canonical predicate identifier.
        field: String,
        /// Comparison operation.
        operator: SmartFilterOperator,
        /// Literal string, boolean or number, represented as text.
        value: String,
    },
}

/// One project-persisted named smart filter.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct SavedSmartFilter {
    /// User-visible unique name.
    pub name: String,
    /// Canonical text expression.
    pub expression: String,
}

/// Parameters for retrieving the current project's saved filters.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct SmartFiltersGetParams {
    /// Open project.
    pub doc: DocId,
}

/// Atomic replacement of a project's saved filter collection.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct SmartFiltersSetParams {
    /// Open project.
    pub doc: DocId,
    /// Complete new collection.
    pub filters: Vec<SavedSmartFilter>,
}

/// The project's saved smart filters.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct SmartFiltersResult {
    /// Named rules in user order.
    pub filters: Vec<SavedSmartFilter>,
}

/// Request for validation and parsing without changing the project.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct SmartFilterParseParams {
    /// Text expression to parse.
    pub expression: String,
}

/// A validated canonical expression and its visual-builder tree.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct SmartFilterParseResult {
    /// Canonical text representation.
    pub expression: String,
    /// Validated nested rule tree.
    pub root: SmartFilterNode,
}
