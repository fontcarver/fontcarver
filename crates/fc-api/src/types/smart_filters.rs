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
