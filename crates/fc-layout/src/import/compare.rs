//! Conservative semantic graph comparison after actual candidate compilation.
use super::{graph::Graph, LayoutImport};
use crate::Error;
use fc_model::GlyphName;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

/// Prove canonical imported rule/feature graphs equal after actual compilation.
///
/// This deliberately rejects transformations for which this bounded normalizer
/// cannot prove equivalence. Text-corpus shaping is an additional acceptance
/// check, never a substitute for this whole-graph check.
pub fn equivalent_layout(
    original: &LayoutImport,
    candidate_bytes: &[u8],
    glyph_order: &[GlyphName],
) -> Result<(), Error> {
    original.decompile()?;
    let candidate = LayoutImport::from_bytes(candidate_bytes, glyph_order)?;
    candidate.decompile()?;
    let fail = |message: &str| {
        Err(Error::LayoutConversion(format!(
            "semantic equivalence could not be proven: {message}"
        )))
    };
    if canonical_text(&original.gdef_source) != canonical_text(&candidate.gdef_source) {
        return fail("GDEF records differ");
    }
    if canonical_text(&original.base_source) != canonical_text(&candidate.base_source) {
        return fail("BASE records differ");
    }
    for tag in ["GSUB", "GPOS"] {
        let before = original.graphs.iter().find(|graph| graph.tag == tag);
        let after = candidate.graphs.iter().find(|graph| graph.tag == tag);
        match (before, after) {
            (None, None) => (),
            (Some(before), Some(after))
                if canonical_graph(before, original)? == canonical_graph(after, &candidate)? => {}
            _ => return fail(&format!("{tag} ordered lookup/feature graph differs")),
        }
    }
    Ok(())
}
fn canonical_graph(graph: &Graph, import: &LayoutImport) -> Result<String, Error> {
    let mut signatures = BTreeMap::new();
    for index in 0..graph.lookups.len() {
        signature(index, graph, import, &mut signatures)?;
    }
    // Distinct equal-body lookups remain distinct vertices: sharing affects how
    // often a lookup executes when several features are enabled together.
    let mut occurrences = BTreeMap::<String, usize>::new();
    let mut identities = BTreeMap::new();
    for (index, signature) in &signatures {
        let occurrence = occurrences.entry(signature.clone()).or_default();
        identities.insert(*index, format!("{signature}#{occurrence}"));
        *occurrence += 1;
    }
    let mut inventory: Vec<_> = graph
        .lookups
        .iter()
        .enumerate()
        .map(|(index, lookup)| {
            let edges: Vec<_> = lookup
                .rules
                .references
                .iter()
                .filter_map(|id| identities.get(&(*id as usize)))
                .collect();
            format!("{}:{edges:?}", identities[&index])
        })
        .collect();
    inventory.sort();
    let mut registrations = Vec::new();
    for registration in &graph.registrations {
        let mut lookups = registration.lookups.clone();
        // OT executes selected lookups in LookupList order, not feature-array order.
        lookups.sort();
        lookups.dedup();
        let lookups: Vec<_> = lookups
            .iter()
            .filter_map(|index| identities.get(&(*index as usize)).cloned())
            .collect();
        registrations.push(format!(
            "{}:{}:{}:{}:{lookups:?}",
            registration.script, registration.language, registration.feature, registration.required
        ));
    }
    registrations.sort();
    let active: BTreeSet<_> = graph
        .registrations
        .iter()
        .flat_map(|r| r.lookups.iter().copied())
        .collect();
    let execution: Vec<_> = active
        .iter()
        .filter_map(|id| identities.get(&(*id as usize)))
        .collect();
    Ok(format!("{inventory:?}\n{registrations:?}\n{execution:?}"))
}
fn signature(
    index: usize,
    graph: &Graph,
    import: &LayoutImport,
    cache: &mut BTreeMap<usize, String>,
) -> Result<String, Error> {
    if let Some(value) = cache.get(&index) {
        return Ok(value.clone());
    }
    let lookup = graph
        .lookups
        .get(index)
        .ok_or_else(|| Error::LayoutConversion("invalid contextual lookup reference".into()))?;
    let mut replacements = BTreeMap::new();
    for reference in &lookup.rules.references {
        replacements.insert(
            format!("FC_{}_{reference}", graph.tag),
            signature(*reference as usize, graph, import, cache)?,
        );
    }
    let mut statements = Vec::new();
    for statement in &lookup.rules.statements {
        let mut tokens = Vec::new();
        for token in statement.split_whitespace() {
            let (key, suffix) = token.strip_suffix(';').map_or((token, ""), |t| (t, ";"));
            tokens.push(
                replacements
                    .get(key)
                    .map_or_else(|| token.to_owned(), |value| format!("({value}){suffix}")),
            );
        }
        statements.push(canonical_text(&tokens.join(" ")));
    }
    // Mark class generated names are scoped by original lookup/subtable index;
    // replacing declaration identifiers with their complete declaration removes
    // numbering differences while retaining mark glyph/anchor semantics.
    let mut mark_classes: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for preamble in &lookup.rules.preamble {
        if let Some(name) = preamble
            .split_whitespace()
            .last()
            .and_then(|s| s.strip_suffix(';'))
        {
            if let Some((content, _)) = preamble.rsplit_once(name) {
                mark_classes
                    .entry(name.into())
                    .or_default()
                    .push(canonical_text(content));
            }
        }
    }
    let mut replacements: Vec<_> = mark_classes
        .into_iter()
        .map(|(name, mut definitions)| {
            definitions.sort();
            (
                name,
                format!(
                    "{:x}",
                    Sha256::digest(format!("{definitions:?}").as_bytes())
                ),
            )
        })
        .collect();
    replacements.sort_by_key(|(name, _)| std::cmp::Reverse(name.len()));
    for (name, definition) in replacements {
        for statement in &mut statements {
            *statement = statement.replace(&name, &definition);
        }
    }
    let attach = import
        .mark_attach
        .get(&(lookup.flags >> 8))
        .map(|gids| named_set(import, gids));
    let filter = lookup
        .filter
        .and_then(|id| import.mark_sets.get(id as usize))
        .map(|gids| named_set(import, gids));
    let kind = match (graph.tag.as_str(), lookup.kind) {
        ("GSUB", 5) => 6,
        ("GPOS", 7) => 8,
        (_, kind) => kind,
    };
    let value = format!(
        "{kind}:{}:{attach:?}:{filter:?}:{statements:?}",
        lookup.flags & 0xff
    );
    let value = format!("{:x}", Sha256::digest(value.as_bytes()));
    cache.insert(index, value.clone());
    Ok(value)
}
fn named_set(import: &LayoutImport, gids: &[u16]) -> Vec<String> {
    let mut names: Vec<_> = gids
        .iter()
        .filter_map(|gid| import.glyph_order.get(*gid as usize))
        .map(ToString::to_string)
        .collect();
    names.sort();
    names
}
fn canonical_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::{
        binary::Rules,
        graph::{Lookup, Registration},
    };
    #[test]
    fn equal_lookup_bodies_do_not_erase_feature_sharing_identity() {
        let make = |middle| {
            let mut rules = Rules::default();
            rules
                .push("pos \\a \\b <0 0 -40 0>;".into(), vec![1, 2])
                .unwrap();
            let lookup = Lookup {
                kind: 2,
                flags: 0,
                filter: None,
                rules,
            };
            Graph {
                tag: "GPOS".into(),
                lookups: vec![lookup.clone(), lookup],
                registrations: [("c001", 0), ("c002", middle), ("c003", 1)]
                    .into_iter()
                    .map(|(feature, lookup)| Registration {
                        script: "latn".into(),
                        language: "dflt".into(),
                        feature: feature.into(),
                        required: false,
                        lookups: vec![lookup],
                    })
                    .collect(),
            }
        };
        let import = LayoutImport::default();
        assert_ne!(
            canonical_graph(&make(0), &import).unwrap(),
            canonical_graph(&make(1), &import).unwrap()
        );
    }
}
