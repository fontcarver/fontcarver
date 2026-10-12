//! Estimate class and Cartesian-rule allocations without expanding glyph lists.

use std::{collections::BTreeMap, ops::Range};

use fea_rs::{Kind, Node, NodeOrToken, ParseTree};

const MAX_RULE_RECORDS: usize = 100_000;
const MAX_TOTAL_RECORDS: usize = 1_000_000;

/// Return the first over-budget source range, in deterministic source order.
pub(super) fn exceeded(tree: &ParseTree) -> Option<Range<usize>> {
    let mut classes = BTreeMap::<String, usize>::new();
    let mut total = 0usize;
    let mut pending = vec![tree.root().clone()];
    while let Some(node) = pending.pop() {
        if node.kind() == Kind::MarkClassNode {
            let name = node
                .iter_children()
                .filter_map(|item| {
                    item.as_token()
                        .filter(|token| token.kind == Kind::NamedGlyphClass)
                        .map(|token| token.text.to_string())
                })
                .last();
            let added = node
                .iter_children()
                .find(|item| glyph_item(item.kind()))
                .map_or(0, |item| cardinality(item, &classes));
            if let Some(name) = name {
                let count = classes
                    .get(&name)
                    .copied()
                    .unwrap_or(0)
                    .saturating_add(added);
                total = total.saturating_add(added);
                if count > MAX_RULE_RECORDS || total > MAX_TOTAL_RECORDS {
                    return Some(node.range());
                }
                classes.insert(name, count);
            }
            continue;
        }
        if node.kind() == Kind::GlyphClassDefNode {
            let name = node.iter_children().find_map(|item| {
                item.as_token()
                    .filter(|token| token.kind == Kind::NamedGlyphClass)
                    .map(|token| token.text.to_string())
            });
            let count = node
                .iter_children()
                .skip_while(|item| item.kind() != Kind::Eq)
                .skip(1)
                .map(|item| cardinality(item, &classes))
                .fold(0usize, usize::saturating_add);
            total = total.saturating_add(count);
            if count > MAX_RULE_RECORDS || total > MAX_TOTAL_RECORDS {
                return Some(node.range());
            }
            if let Some(name) = name {
                classes.insert(name, count);
            }
            continue;
        }
        if node.kind() == Kind::GlyphClass {
            let count = cardinality(&NodeOrToken::Node(node.clone()), &classes);
            total = total.saturating_add(count);
            if count > MAX_RULE_RECORDS || total > MAX_TOTAL_RECORDS {
                return Some(node.range());
            }
            continue;
        }
        let ligature = node.kind() == Kind::GsubType4;
        let enumerated_pair = node.kind() == Kind::GposType2
            && node.iter_children().any(|item| item.kind() == Kind::EnumKw);
        if ligature || enumerated_pair {
            let mut combinations = 1usize;
            let mut inputs = 0usize;
            for item in node
                .iter_children()
                .take_while(|item| !ligature || item.kind() != Kind::ByKw)
            {
                if glyph_item(item.kind()) {
                    inputs += 1;
                    combinations = combinations.saturating_mul(cardinality(item, &classes));
                }
            }
            // Each expanded ligature owns its entire input sequence, while an
            // enumerated pair owns two glyph records. Counting only rules would
            // leave long ligatures an independent allocation multiplier.
            let records = combinations.saturating_mul(inputs);
            total = total.saturating_add(records);
            if records > MAX_RULE_RECORDS || total > MAX_TOTAL_RECORDS {
                return Some(node.range());
            }
        }
        for item in node.iter_children() {
            if item.kind() == Kind::NamedGlyphClass {
                total = total.saturating_add(cardinality(item, &classes));
                if total > MAX_TOTAL_RECORDS {
                    return Some(node.range());
                }
            }
        }
        let children: Vec<_> = node
            .iter_children()
            .filter_map(NodeOrToken::as_node)
            .cloned()
            .collect();
        pending.extend(children.into_iter().rev());
    }
    None
}

fn glyph_item(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::GlyphName | Kind::Cid | Kind::NamedGlyphClass | Kind::GlyphClass | Kind::GlyphRange
    )
}

fn cardinality(item: &NodeOrToken, classes: &BTreeMap<String, usize>) -> usize {
    match item {
        NodeOrToken::Token(token) => match token.kind {
            Kind::GlyphName | Kind::Cid => 1,
            // Undefined/forward references are rejected by semantic validation;
            // valid references have already been visited in source order.
            Kind::NamedGlyphClass => classes.get(token.text.as_str()).copied().unwrap_or(1),
            _ => 0,
        },
        NodeOrToken::Node(node) if node.kind() == Kind::GlyphClass => node
            .iter_children()
            .map(|item| cardinality(item, classes))
            .fold(0usize, usize::saturating_add),
        NodeOrToken::Node(node) if node.kind() == Kind::GlyphRange => range_count(node),
        _ => 0,
    }
}

fn range_count(node: &Node) -> usize {
    let ends: Vec<_> = node
        .iter_tokens()
        .filter(|token| matches!(token.kind, Kind::GlyphName | Kind::Cid))
        .collect();
    let [start, end] = ends.as_slice() else {
        return MAX_RULE_RECORDS + 1;
    };
    if start.kind == Kind::Cid && end.kind == Kind::Cid {
        return decimal_distance(
            start.text.trim_start_matches('\\').as_bytes(),
            end.text.trim_start_matches('\\').as_bytes(),
        )
        .unwrap_or(MAX_RULE_RECORDS + 1);
    }
    let (start, end) = (start.text.as_bytes(), end.text.as_bytes());
    if start == end {
        return 1;
    }
    if start.len() != end.len() {
        // CIDs need not have the same decimal width; names with different
        // widths are invalid, and the semantic validator still diagnoses them.
        return decimal_distance(start, end).unwrap_or(MAX_RULE_RECORDS + 1);
    }
    let mut first = start.iter().zip(end).position(|(a, b)| a != b).unwrap_or(0);
    let mut last = start
        .iter()
        .zip(end)
        .rposition(|(a, b)| a != b)
        .unwrap_or(first);
    if first == last && start[first].is_ascii_alphabetic() && end[first].is_ascii_alphabetic() {
        usize::from(start[first].abs_diff(end[first])) + 1
    } else {
        // Equal trailing digits still multiply the range size: g0000-g0200
        // denotes 201 names, not the three differing digits 0..2.
        while first > 0 && start[first - 1].is_ascii_digit() && end[first - 1].is_ascii_digit() {
            first -= 1;
        }
        while last + 1 < start.len()
            && start[last + 1].is_ascii_digit()
            && end[last + 1].is_ascii_digit()
        {
            last += 1;
        }
        decimal_distance(&start[first..=last], &end[first..=last]).unwrap_or(MAX_RULE_RECORDS + 1)
    }
}

fn decimal_distance(start: &[u8], end: &[u8]) -> Option<usize> {
    let start: usize = std::str::from_utf8(start).ok()?.parse().ok()?;
    let end: usize = std::str::from_utf8(end).ok()?.parse().ok()?;
    Some(start.abs_diff(end).saturating_add(1))
}

#[cfg(test)]
mod tests {
    #[test]
    fn short_cid_ranges_do_not_exceed_the_budget() {
        let source: std::sync::Arc<str> =
            "@C = [\\9-\\10]; feature liga { sub @C @C by \\9; } liga;".into();
        let resolver = move |_: &std::path::Path| Ok(source.clone());
        let (tree, _) =
            fea_rs::parse::parse_root("cid.fea".into(), None, Box::new(resolver)).unwrap();
        assert!(super::exceeded(&tree).is_none());
    }
}
