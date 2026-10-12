//! Bounded horizontal kerning queries for editing source fonts.

use fc_doc::{
    kerning::{HorizontalKerningEditBlock, KerningGroupSide},
    DocId,
};
use fc_model::{KernPair, KernSide, MasterId};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Source ownership at a particular document generation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct KerningStatus {
    /// Generation that all fields refer to.
    #[ts(type = "number")]
    pub generation: u64,
    /// Whether source ownership permits authoring.
    pub editable: bool,
    /// Why authoring is unavailable.
    pub reason: Option<HorizontalKerningEditBlock>,
}

/// Select a concrete glyph pair and master.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct KerningResolveParams {
    /// Open document.
    pub doc: DocId,
    /// Persistent master identifier.
    pub master: MasterId,
    /// First glyph name.
    pub left: String,
    /// Second glyph name.
    pub right: String,
}

/// Stored rules applying to a concrete pair, without compiled positioning.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct KerningResolution {
    /// Eligibility and generation.
    pub status: KerningStatus,
    /// Master inspected.
    pub master: MasterId,
    /// First glyph.
    pub left: String,
    /// Second glyph.
    pub right: String,
    /// First-side authoritative membership.
    pub left_group: Option<String>,
    /// Second-side authoritative membership.
    pub right_group: Option<String>,
    /// Winning adjustment, zero if absent; null if authoring is unsupported.
    pub value: Option<f64>,
    /// Winning stored rule, retaining explicit zero.
    pub rule: Option<KernPair>,
    /// At most four candidate records, in exception priority order.
    pub records: Vec<KernPair>,
    /// Whether the winning rule is an exception to a broader class rule.
    ///
    /// True when the rule names a concrete glyph on a side where that glyph
    /// also belongs to a kerning group, so the pair deliberately departs from
    /// its class. False for a class rule, and false when there is no rule.
    #[serde(default)]
    #[ts(as = "Option<bool>", optional)]
    pub exception: bool,
}

/// Largest number of boundaries one exception request may carry.
pub const KERNING_BOUNDARY_LIMIT: usize = 256;

/// Select every editable boundary of one shaped run in one master.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct KerningBoundariesParams {
    /// Open document.
    pub doc: DocId,
    /// Master inspected.
    pub master: MasterId,
    /// Read the vertical table instead of the horizontal one.
    #[serde(default)]
    pub vertical: bool,
    /// Ordered `[left, right]` glyph names, at most [`KERNING_BOUNDARY_LIMIT`].
    pub pairs: Vec<[String; 2]>,
}

/// What applies at one boundary, without any compiled positioning.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct KerningBoundary {
    /// Winning adjustment, zero when no rule applies; null when unreadable.
    pub value: Option<f64>,
    /// Whether the winning rule departs from the class rule that would apply.
    pub exception: bool,
}

/// Boundary answers in request order, at one document generation.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct KerningBoundaries {
    /// Eligibility and generation.
    pub status: KerningStatus,
    /// One entry per requested boundary, in request order.
    pub items: Vec<KerningBoundary>,
}

/// Select a glyph's two authoritative kerning memberships.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GlyphKerningParams {
    /// Open document.
    pub doc: DocId,
    /// Persistent glyph name.
    pub glyph: String,
}

/// A glyph's memberships; invalid imported membership is reported separately.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GlyphKerning {
    /// Eligibility and generation.
    pub status: KerningStatus,
    /// First-side group.
    pub side1: Option<String>,
    /// Second-side group.
    pub side2: Option<String>,
    /// Membership diagnostics, without discarding the underlying records.
    pub diagnostics: Vec<String>,
}

/// Bounded pair page for one master.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct KerningPairsParams {
    /// Open document.
    pub doc: DocId,
    /// Master inspected.
    pub master: MasterId,
    /// Required generation for continuation pages.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub generation: Option<u64>,
    /// Number of records to skip.
    pub offset: u32,
    /// Maximum records, at most 200.
    pub limit: u32,
}

/// Stored pair data, including diagnostics for unresolved imported records.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct KerningPairRecord {
    /// First pair side.
    pub left: KernSide,
    /// Second pair side.
    pub right: KernSide,
    /// Stored value; null represents a non-finite imported value.
    pub value: Option<f64>,
    /// Invalid references or values, if present.
    pub diagnostic: Option<String>,
    /// Whether this record is an exception to a broader class rule.
    #[serde(default)]
    #[ts(as = "Option<bool>", optional)]
    pub exception: bool,
}

/// Bounded stored pair page.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct KerningPairs {
    /// Eligibility and generation.
    pub status: KerningStatus,
    /// Total stored records for this master.
    pub total: u32,
    /// Requested page, without group expansion.
    pub items: Vec<KerningPairRecord>,
}

/// Bounded group page, optionally filtered by name.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct KerningGroupsParams {
    /// Open document.
    pub doc: DocId,
    /// Logical pair position.
    pub side: KerningGroupSide,
    /// Required generation for continuation pages.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub generation: Option<u64>,
    /// Case-sensitive name filter.
    #[serde(default)]
    pub query: Option<String>,
    /// Number of matching groups to skip.
    pub offset: u32,
    /// Maximum groups, at most 200.
    pub limit: u32,
}

/// Group summary with bounded member examples and removal impact.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct KerningGroupSummary {
    /// Canonical group name.
    pub name: String,
    /// Number of member glyphs, including unresolved imported references.
    pub member_count: u32,
    /// Number of horizontal pair records referencing this side in all masters.
    pub pair_count: u32,
    /// Up to eight members for an impact preview.
    pub sample_members: Vec<String>,
    /// Invalid imported membership, if present.
    pub diagnostic: Option<String>,
}

/// A bounded page of group summaries.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct KerningGroupsResult {
    /// Eligibility and generation.
    pub status: KerningStatus,
    /// Matching group count.
    pub total: u32,
    /// Requested summaries.
    pub items: Vec<KerningGroupSummary>,
}

/// A bounded member page for one group.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct KerningMembersParams {
    /// Open document.
    pub doc: DocId,
    /// Logical pair position.
    pub side: KerningGroupSide,
    /// Canonical group name.
    pub name: String,
    /// Required generation for continuation pages.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub generation: Option<u64>,
    /// Number of members to skip.
    pub offset: u32,
    /// Maximum members, at most 200.
    pub limit: u32,
}

/// Members of a group, with the same generation contract as group pages.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct KerningMembers {
    /// Eligibility and generation.
    pub status: KerningStatus,
    /// Stored member count.
    pub total: u32,
    /// Requested member names.
    pub items: Vec<String>,
}
