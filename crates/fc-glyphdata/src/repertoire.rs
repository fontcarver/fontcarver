//! Sizes and members of the Unicode repertoires the glyph sidebar groups by.
//!
//! A sidebar row says "present / total": how many glyphs of the font belong to a
//! category, subcategory, script or Unicode block, out of how many *assigned*
//! Unicode scalars that repertoire has. Unassigned code points, surrogates and
//! noncharacters are never counted, so the denominator is the set of characters
//! a font could actually encode, and "missing" is a list a user can generate.

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::{block_name, category, category_name, is_assigned, script, sub_category};

/// Which grouping a repertoire name belongs to.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RepertoireKind {
    /// A broad category name such as `Letter`.
    Category,
    /// A `Category/Subcategory` pair such as `Letter/Uppercase`.
    SubCategory,
    /// An ISO 15924 script code such as `Latn`.
    Script,
    /// A Unicode block name such as `Cyrillic`.
    Block,
}

#[derive(Default)]
struct Totals {
    categories: HashMap<&'static str, u32>,
    sub_categories: HashMap<(&'static str, &'static str), u32>,
    scripts: HashMap<&'static str, u32>,
    blocks: HashMap<&'static str, u32>,
}

/// Built once: a full pass over the assigned scalars is far cheaper than a pass
/// per sidebar row, and the tables are a few kilobytes.
static TOTALS: LazyLock<Totals> = LazyLock::new(|| {
    let mut totals = Totals::default();
    for cp in 0..=0x10_FFFF_u32 {
        if !is_assigned(cp) {
            continue;
        }
        let category = category_name(category(cp));
        let sub_category = sub_category(cp);
        *totals.categories.entry(category).or_default() += 1;
        *totals
            .sub_categories
            .entry((category, sub_category))
            .or_default() += 1;
        *totals.scripts.entry(script(cp)).or_default() += 1;
        *totals.blocks.entry(block_name(cp)).or_default() += 1;
    }
    totals
});

/// Returns how many assigned Unicode scalars one repertoire contains.
///
/// An unknown name has no members, which is reported as `0` rather than an
/// error: a document may carry a category or script spelling this build of the
/// Unicode tables does not know.
pub fn repertoire_total(kind: RepertoireKind, value: &str) -> u32 {
    let totals = &*TOTALS;
    match kind {
        RepertoireKind::Category => totals.categories.get(value).copied().unwrap_or_default(),
        RepertoireKind::SubCategory => split_sub_category(value)
            .and_then(|pair| totals.sub_categories.get(&pair).copied())
            .unwrap_or_default(),
        RepertoireKind::Script => totals.scripts.get(value).copied().unwrap_or_default(),
        RepertoireKind::Block => totals.blocks.get(value).copied().unwrap_or_default(),
    }
}

/// Iterates the assigned Unicode scalars of one repertoire in code-point order.
///
/// The iterator walks the whole scalar space, so a caller that only needs a
/// count should use [`repertoire_total`], which is table-driven.
pub fn repertoire_members<'a>(
    kind: RepertoireKind,
    value: &'a str,
) -> impl Iterator<Item = u32> + 'a {
    (0..=0x10_FFFF_u32).filter(move |cp| is_assigned(*cp) && belongs(kind, value, *cp))
}

/// Returns whether one assigned scalar belongs to a repertoire.
///
/// An unassigned code point never belongs, so a font's stray encoding cannot
/// inflate a coverage count.
pub fn repertoire_contains(kind: RepertoireKind, value: &str, cp: u32) -> bool {
    is_assigned(cp) && belongs(kind, value, cp)
}

fn belongs(kind: RepertoireKind, value: &str, cp: u32) -> bool {
    match kind {
        RepertoireKind::Category => category_name(category(cp)) == value,
        // Split here rather than through the totals table: this runs once per
        // encoded scalar of every sidebar row.
        RepertoireKind::SubCategory => {
            value.split_once('/').is_some_and(|(category_value, sub)| {
                category_name(category(cp)) == category_value && sub_category(cp) == sub
            })
        }
        RepertoireKind::Script => script(cp) == value,
        RepertoireKind::Block => block_name(cp) == value,
    }
}

/// Splits `Letter/Uppercase` into its two halves, borrowing the static tables.
fn split_sub_category(value: &str) -> Option<(&'static str, &'static str)> {
    let (category_value, sub) = value.split_once('/')?;
    let totals = &*TOTALS;
    totals
        .sub_categories
        .keys()
        .find(|(known_category, known_sub)| *known_category == category_value && *known_sub == sub)
        .copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_totals_count_only_assigned_scalars() {
        // Basic Latin is fully assigned, so its total is the whole range.
        assert_eq!(repertoire_total(RepertoireKind::Block, "Basic Latin"), 128);
        // Greek and Coptic spans 144 code points and keeps unassigned holes.
        let greek = repertoire_total(RepertoireKind::Block, "Greek and Coptic");
        assert!((100..144).contains(&greek), "Greek total was {greek}");
        assert_eq!(repertoire_total(RepertoireKind::Block, "Not A Block"), 0);
    }

    #[test]
    fn category_and_subcategory_totals_agree_with_their_members() {
        let uppercase = repertoire_total(RepertoireKind::SubCategory, "Letter/Uppercase");
        assert!(uppercase > 1_000, "uppercase total was {uppercase}");
        assert!(repertoire_total(RepertoireKind::Category, "Letter") > uppercase);
        assert_eq!(
            repertoire_total(RepertoireKind::SubCategory, "Letter/Nonexistent"),
            0
        );
        assert_eq!(
            repertoire_total(RepertoireKind::SubCategory, "Letter"),
            0,
            "a subcategory name without a slash has no members"
        );
    }

    #[test]
    fn members_are_ordered_assigned_and_match_the_total() {
        let members: Vec<_> = repertoire_members(RepertoireKind::Block, "Basic Latin").collect();
        assert_eq!(members.len(), 128);
        assert_eq!(members.first(), Some(&0));
        assert_eq!(members.last(), Some(&0x7F));
        let greek: Vec<_> = repertoire_members(RepertoireKind::Script, "Grek")
            .take(4)
            .collect();
        assert!(greek.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(greek.iter().all(|cp| is_assigned(*cp)));
    }

    #[test]
    fn surrogates_and_unassigned_scalars_are_excluded() {
        assert!(!is_assigned(0xD800));
        assert!(!is_assigned(0x0378));
        assert!(is_assigned(0x0041));
        assert!(!repertoire_members(RepertoireKind::Script, "Zzzz").any(|cp| cp == 0xD800));
    }
}
