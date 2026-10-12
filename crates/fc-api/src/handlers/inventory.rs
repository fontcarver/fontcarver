//! Atomic inventory collection edits using the existing font-info undo snapshot.
//!
//! Also serves the sidebar's repertoire coverage: how much of a category,
//! subcategory, script or Unicode block the open document encodes, and which of
//! its characters are still missing.

use std::collections::{BTreeMap, BTreeSet};

use fc_doc::{Command, Document};
use fc_glyphdata::{repertoire_members, repertoire_total};
use serde_json::{Map, Value};

use crate::{
    Api, ApiError, CoverageRow, InventoryCoverage, InventoryCoverageParams, MissingMembers,
    MissingMembersParams,
};

const INVENTORY_KEY: &str = "com.fontcarver.inventory";

/// The most missing scalars one request may return.
const MAX_MISSING: usize = 8_192;

/// Reports coverage for every repertoire the document's glyphs fall into.
///
/// Rows appear only for repertoires the document already touches, which keeps
/// the sidebar to the sections it shows today. `present` counts characters of
/// the repertoire that any glyph encodes, `glyphs` counts glyphs classified by
/// their first scalar — the same rule the sidebar filter uses.
pub(crate) fn coverage(
    api: &Api,
    params: InventoryCoverageParams,
) -> Result<InventoryCoverage, ApiError> {
    api.session
        .with_doc(params.doc, |document| {
            let mut categories = Tally::default();
            let mut sub_categories = Tally::default();
            let mut scripts = Tally::default();
            let mut blocks = Tally::default();
            for glyph in document.font.glyphs.values() {
                let first = glyph.unicodes.first().copied();
                let encoded: Vec<u32> = glyph
                    .unicodes
                    .iter()
                    .copied()
                    .filter(|cp| fc_glyphdata::is_assigned(*cp))
                    .collect();
                let Some(first) = first.filter(|cp| fc_glyphdata::is_assigned(*cp)) else {
                    continue;
                };
                let category = fc_glyphdata::category_name(fc_glyphdata::category(first));
                categories.count(category.to_owned(), &encoded);
                sub_categories.count(
                    format!("{category}/{}", fc_glyphdata::sub_category(first)),
                    &encoded,
                );
                scripts.count(fc_glyphdata::script(first).to_owned(), &encoded);
                blocks.count(fc_glyphdata::block_name(first).to_owned(), &encoded);
            }
            Ok(InventoryCoverage {
                categories: categories.rows(fc_glyphdata::RepertoireKind::Category),
                sub_categories: sub_categories.rows(fc_glyphdata::RepertoireKind::SubCategory),
                scripts: scripts.rows(fc_glyphdata::RepertoireKind::Script),
                blocks: blocks.rows(fc_glyphdata::RepertoireKind::Block),
                total_glyphs: u32::try_from(document.font.glyphs.len()).unwrap_or(u32::MAX),
            })
        })
        .map_err(ApiError::from)?
}

/// Lists the characters of one repertoire that no glyph of the document encodes.
pub(crate) fn missing(api: &Api, params: MissingMembersParams) -> Result<MissingMembers, ApiError> {
    if params.value.len() > 128 {
        return Err(ApiError::invalid_params(
            "repertoire names are at most 128 bytes",
        ));
    }
    let limit = (params.limit as usize).min(MAX_MISSING);
    api.session
        .with_doc(params.doc, |document| {
            let encoded: BTreeSet<u32> = document
                .font
                .glyphs
                .values()
                .flat_map(|glyph| glyph.unicodes.iter().copied())
                .collect();
            let mut scalars = Vec::new();
            let mut total = 0_u32;
            for scalar in repertoire_members(params.kind.into(), &params.value) {
                if encoded.contains(&scalar) {
                    continue;
                }
                total = total.saturating_add(1);
                if scalars.len() < limit {
                    scalars.push(scalar);
                }
            }
            Ok(MissingMembers {
                recipe: recipe_for(&scalars),
                scalars,
                total,
            })
        })
        .map_err(ApiError::from)?
}

/// Condenses sorted scalars into the "Add Glyphs" range grammar.
fn recipe_for(scalars: &[u32]) -> String {
    let mut items: Vec<String> = Vec::new();
    let mut index = 0;
    while index < scalars.len() {
        let start = scalars[index];
        let mut end = start;
        while index + 1 < scalars.len() && scalars[index + 1] == end + 1 {
            index += 1;
            end = scalars[index];
        }
        items.push(if end > start + 1 {
            format!("U+{start:04X}-U+{end:04X}")
        } else if end == start + 1 {
            format!("U+{start:04X} U+{end:04X}")
        } else {
            format!("U+{start:04X}")
        });
        index += 1;
    }
    items.join(" ")
}

/// Per-repertoire glyph counts and the distinct characters they encode.
#[derive(Default)]
struct Tally(BTreeMap<String, (u32, BTreeSet<u32>)>);

impl Tally {
    fn count(&mut self, value: String, encoded: &[u32]) {
        let entry = self.0.entry(value).or_default();
        entry.0 += 1;
        entry.1.extend(encoded.iter().copied());
    }

    fn rows(self, kind: fc_glyphdata::RepertoireKind) -> Vec<CoverageRow> {
        self.0
            .into_iter()
            .map(|(value, (glyphs, encoded))| {
                // Only characters of this repertoire count towards its coverage:
                // a glyph may also carry scalars from another script or block.
                let present = u32::try_from(
                    encoded
                        .iter()
                        .filter(|scalar| fc_glyphdata::repertoire_contains(kind, &value, **scalar))
                        .count(),
                )
                .unwrap_or(u32::MAX);
                let total = repertoire_total(kind, &value);
                CoverageRow {
                    value,
                    present,
                    total,
                    glyphs,
                }
            })
            .collect()
    }
}

/// Read one collection without treating incompatible metadata as an empty value.
pub(crate) fn read_collection<'a>(
    user_data: &'a Value,
    key: &str,
) -> Result<Option<&'a Value>, ApiError> {
    let Some(root) = object_or_absent(user_data, "font info user data")? else {
        return Ok(None);
    };
    let Some(inventory) = root.get(INVENTORY_KEY) else {
        return Ok(None);
    };
    Ok(object_or_absent(inventory, "font inventory metadata")?
        .and_then(|inventory| inventory.get(key)))
}

/// Replace one validated collection while preserving its sibling metadata.
pub(crate) fn write_collection(
    document: &mut Document,
    key: &str,
    value: Value,
    label: &str,
) -> Result<(), ApiError> {
    if read_collection(&document.font.info.user_data, key)? == Some(&value) {
        return Ok(());
    }
    let mut info = document.font.info.clone();
    let mut root = object_or_absent(&info.user_data, "font info user data")?
        .cloned()
        .unwrap_or_default();
    let mut inventory = root
        .get(INVENTORY_KEY)
        .map(|value| object_or_absent(value, "font inventory metadata"))
        .transpose()?
        .flatten()
        .cloned()
        .unwrap_or_default();
    inventory.insert(key.to_owned(), value);
    root.insert(INVENTORY_KEY.to_owned(), Value::Object(inventory));
    info.user_data = Value::Object(root);
    document
        .apply(
            Command::SetFontInfo {
                info: Box::new(info),
            },
            label,
        )
        .map_err(ApiError::from)?;
    Ok(())
}

fn object_or_absent<'a>(
    value: &'a Value,
    description: &str,
) -> Result<Option<&'a Map<String, Value>>, ApiError> {
    match value {
        Value::Null => Ok(None),
        Value::Object(object) => Ok(Some(object)),
        _ => Err(ApiError::invalid_params(format!(
            "{description} must be an object or null; existing metadata was preserved"
        ))),
    }
}
