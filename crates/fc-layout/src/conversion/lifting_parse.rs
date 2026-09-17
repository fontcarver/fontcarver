//! Bounded whole-lookup extraction for optional model lifting.

use std::collections::{BTreeMap, BTreeSet};

use fc_model::{Anchor, Font, GlyphName, KernSide, Kerning, KerningGroups, Tag};

use crate::import::{
    binary::{Reader, Result, MAX_RULES},
    graph::Graph,
};

pub(super) fn exclusive(graph: &Graph, tags: &[&str]) -> Result<Vec<u16>> {
    let selected: BTreeSet<_> = graph
        .registrations
        .iter()
        .filter(|registration| tags.contains(&registration.feature.as_str()))
        .flat_map(|registration| registration.lookups.iter().copied())
        .collect();
    if selected.is_empty() {
        return Err("no eligible registered lookup".into());
    }
    for registration in &graph.registrations {
        if !tags.contains(&registration.feature.as_str())
            && registration.lookups.iter().any(|id| selected.contains(id))
        {
            return Err("lookup is shared with another feature".into());
        }
    }
    if graph.lookups.iter().any(|lookup| {
        lookup
            .rules
            .references
            .iter()
            .any(|id| selected.contains(id))
    }) {
        return Err("lookup is called by contextual positioning".into());
    }
    Ok(selected.into_iter().collect())
}

fn subtable(font: &Font, index: u16, expected: u16) -> Result<Reader<'_>> {
    let bytes = font
        .extra
        .raw_tables
        .get(&Tag(*b"GPOS"))
        .ok_or("missing GPOS")?;
    let list = Reader::new(bytes).offset16(8)?;
    if index >= list.u16(0)? {
        return Err("lookup index outside GPOS".into());
    }
    let lookup = list.offset16(2 + usize::from(index) * 2)?;
    if lookup.u16(4)? != 1 {
        return Err("multiple subtables require an unimplemented precedence proof".into());
    }
    let mut sub = lookup.offset16(6)?;
    let mut kind = lookup.u16(0)?;
    if kind == 9 {
        if sub.u16(0)? != 1 {
            return Err("unknown extension format".into());
        }
        kind = sub.u16(2)?;
        let offset = sub.u32(4)? as usize;
        if offset == 0 {
            return Err("null extension offset".into());
        }
        sub = sub.at(offset)?;
    }
    if kind != expected {
        return Err("lookup type cannot be lifted to this model".into());
    }
    Ok(sub)
}

pub(super) fn kerning(
    font: &Font,
    graph: &Graph,
    names: &[GlyphName],
) -> Result<(Vec<u16>, Kerning, KerningGroups)> {
    let selected = exclusive(graph, &["kern"])?;
    if selected.len() != 1 {
        return Err("multiple kern lookups require an unimplemented additive proof".into());
    }
    let sub = subtable(font, selected[0], 2)?;
    if sub.u16(4)? != 4 || sub.u16(6)? != 0 {
        return Err("kerning requires xAdvance-only Value1 and an absent Value2 record".into());
    }
    let coverage = sub.offset16(2)?.coverage(names)?;
    let mut pairs = Kerning::default();
    let mut groups = KerningGroups::default();
    match sub.u16(0)? {
        1 => {
            if usize::from(sub.u16(8)?) != coverage.len() {
                return Err("pair set count differs from coverage".into());
            }
            for (index, left) in coverage.into_iter().enumerate() {
                let set = sub.offset16(10 + index * 2)?;
                for pair in 0..usize::from(set.u16(0)?) {
                    let right = set.u16(2 + pair * 4)?;
                    let left = names
                        .get(usize::from(left))
                        .ok_or("invalid left glyph")?
                        .clone();
                    let right = names
                        .get(usize::from(right))
                        .ok_or("invalid right glyph")?
                        .clone();
                    let value = f64::from(set.i16(4 + pair * 4)?);
                    if pairs
                        .0
                        .insert((KernSide::Glyph(left), KernSide::Glyph(right)), value)
                        .is_some()
                    {
                        return Err("duplicate glyph pair".into());
                    }
                    if pairs.0.len() > MAX_RULES {
                        return Err("kerning rule limit".into());
                    }
                }
            }
        }
        2 => {
            let first = sub.offset16(8)?.classes(names)?;
            let second = sub.offset16(10)?.classes(names)?;
            let first_count = usize::from(sub.u16(12)?);
            let second_count = usize::from(sub.u16(14)?);
            if first_count == 0
                || second_count == 0
                || first_count.saturating_mul(second_count) > MAX_RULES
            {
                return Err("kerning class matrix limit".into());
            }
            if first.iter().any(|class| usize::from(*class) >= first_count)
                || second
                    .iter()
                    .any(|class| usize::from(*class) >= second_count)
            {
                return Err("class outside matrix".into());
            }
            // Materialize both class-zero complements by glyph identity. No
            // wildcard membership or pairwise glyph expansion enters the model.
            for gid in coverage {
                let index = usize::from(gid);
                groups
                    .side1
                    .entry(group_name(1, first[index]))
                    .or_default()
                    .insert(names[index].clone());
            }
            for (index, name) in names.iter().enumerate() {
                groups
                    .side2
                    .entry(group_name(2, second[index]))
                    .or_default()
                    .insert(name.clone());
            }
            for first_class in 0..first_count {
                let left = group_name(1, first_class as u16);
                if !groups.side1.contains_key(&left) {
                    continue;
                }
                for second_class in 0..second_count {
                    let right = group_name(2, second_class as u16);
                    if !groups.side2.contains_key(&right) {
                        continue;
                    }
                    let value =
                        f64::from(sub.i16(16 + (first_class * second_count + second_class) * 2)?);
                    pairs.0.insert(
                        (KernSide::Group(left.clone()), KernSide::Group(right)),
                        value,
                    );
                }
            }
        }
        _ => return Err("unsupported PairPos format".into()),
    }
    Ok((selected, pairs, groups))
}

fn group_name(side: u8, class: u16) -> String {
    format!("public.kern{side}.FCImported{class}")
}

type AnchorMap = BTreeMap<GlyphName, Vec<Anchor>>;
type ExtractedAnchors = (Vec<u16>, AnchorMap);

pub(super) fn anchors(font: &Font, graph: &Graph, names: &[GlyphName]) -> Result<ExtractedAnchors> {
    let selected = exclusive(graph, &["mark", "mkmk", "abvm", "blwm"])?;
    let core: BTreeSet<_> = graph
        .registrations
        .iter()
        .filter(|registration| matches!(registration.feature.as_str(), "mark" | "mkmk"))
        .flat_map(|registration| registration.lookups.iter().copied())
        .collect();
    if core.is_empty() || core.len() > 2 || selected.len() > 4 {
        return Err("anchor lift supports one mark and one mkmk lookup plus at most two exact writer clones".into());
    }
    let mut kinds = BTreeSet::new();
    let mut extracted = BTreeMap::new();
    for &index in &selected {
        let lookup = graph
            .lookups
            .get(usize::from(index))
            .ok_or("missing lookup")?;
        if !matches!(lookup.kind, 4 | 6) || (core.contains(&index) && !kinds.insert(lookup.kind)) {
            return Err(
                "anchor lift requires whole nonshared mark-to-base/mark-to-mark lookups".into(),
            );
        }
        let incompatible_tag = if lookup.kind == 4 { "mkmk" } else { "mark" };
        if graph.registrations.iter().any(|registration| {
            registration.lookups.contains(&index) && registration.feature == incompatible_tag
        }) {
            return Err("anchor lookup belongs to an incompatible feature".into());
        }
        extracted.insert(index, lookup_anchors(font, index, lookup.kind, names)?);
    }
    // The pinned writer also emits identical abvm/blwm lookup copies. Leaving
    // those source-owned would reorder generated kerning ahead of the copy.
    // Transfer only exact copies of a selected mark/mkmk lookup; the final
    // compiler proof still checks their individual feature registrations/order.
    for &index in selected.iter().filter(|index| !core.contains(index)) {
        let lookup = &graph.lookups[usize::from(index)];
        if !core.iter().any(|core_index| {
            let original = &graph.lookups[usize::from(*core_index)];
            lookup.kind == original.kind
                && lookup.flags == original.flags
                && lookup.filter == original.filter
                && extracted.get(&index) == extracted.get(core_index)
        }) {
            return Err("abvm/blwm lookup is not an exact copy of a selected anchor lookup".into());
        }
    }
    let mut output: AnchorMap = BTreeMap::new();
    for index in core {
        let anchors = extracted
            .remove(&index)
            .ok_or("missing extracted anchors")?;
        for (name, additions) in anchors {
            for anchor in additions {
                merge_anchor(&mut output, &name, anchor)?;
            }
        }
    }
    Ok((selected, output))
}

fn lookup_anchors(font: &Font, index: u16, kind: u16, names: &[GlyphName]) -> Result<AnchorMap> {
    let sub = subtable(font, index, kind)?;
    if sub.u16(0)? != 1 || sub.u16(6)? != 1 {
        return Err("anchor lift currently requires one static mark class".into());
    }
    let marks = sub.offset16(2)?.coverage(names)?;
    let bases = sub.offset16(4)?.coverage(names)?;
    let mark_array = sub.offset16(8)?;
    let base_array = sub.offset16(10)?;
    if usize::from(mark_array.u16(0)?) != marks.len()
        || usize::from(base_array.u16(0)?) != bases.len()
    {
        return Err("anchor array length differs from coverage".into());
    }
    let mut output = BTreeMap::new();
    for (i, gid) in marks.into_iter().enumerate() {
        if mark_array.u16(2 + i * 4)? != 0 {
            return Err("mark class is out of range".into());
        }
        let anchor = xy(mark_array.offset16(4 + i * 4)?, "_top")?;
        add_anchor(&mut output, names, gid, anchor)?;
    }
    for (i, gid) in bases.into_iter().enumerate() {
        if base_array.u16(2 + i * 2)? == 0 {
            continue;
        }
        let anchor = xy(base_array.offset16(2 + i * 2)?, "top")?;
        add_anchor(&mut output, names, gid, anchor)?;
    }
    Ok(output)
}

fn xy(reader: Reader<'_>, name: &str) -> Result<Anchor> {
    if reader.u16(0)? != 1 {
        return Err("point/device/variation anchors are not proven model mappings".into());
    }
    Ok(Anchor {
        name: name.into(),
        x: f64::from(reader.i16(2)?),
        y: f64::from(reader.i16(4)?),
    })
}

fn add_anchor(
    output: &mut BTreeMap<GlyphName, Vec<Anchor>>,
    names: &[GlyphName],
    gid: u16,
    anchor: Anchor,
) -> Result<()> {
    let name = names
        .get(usize::from(gid))
        .ok_or("anchor glyph outside original order")?;
    merge_anchor(output, name, anchor)
}

fn merge_anchor(output: &mut AnchorMap, name: &GlyphName, anchor: Anchor) -> Result<()> {
    let anchors = output.entry(name.clone()).or_default();
    if let Some(previous) = anchors.iter().find(|previous| previous.name == anchor.name) {
        if previous != &anchor {
            return Err("lookups disagree about the named model anchor".into());
        }
    } else {
        anchors.push(anchor);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::graph::Registration;

    fn fixture(subtable: &[u16]) -> (Font, Graph, Vec<GlyphName>) {
        let mut font = Font::new_empty("Pair extraction", 1000);
        let words: Vec<_> = [1, 0, 0, 0, 10, 1, 4, 2, 0, 1, 8]
            .into_iter()
            .chain(subtable.iter().copied())
            .collect();
        font.extra.raw_tables.insert(
            Tag(*b"GPOS"),
            words.into_iter().flat_map(u16::to_be_bytes).collect(),
        );
        let graph = Graph {
            registrations: vec![Registration {
                script: "DFLT".into(),
                language: "dflt".into(),
                feature: "kern".into(),
                required: false,
                lookups: vec![0],
            }],
            ..Default::default()
        };
        (
            font,
            graph,
            [".notdef", "A", "V", "acute", "ligature", "grave"]
                .into_iter()
                .map(Into::into)
                .collect(),
        )
    }

    #[test]
    fn encoded_zero_second_value_is_not_absent_value_record() {
        let (font, graph, names) =
            fixture(&[1, 20, 4, 4, 1, 12, 1, 2, (-80_i16) as u16, 0, 1, 1, 1]);
        assert!(kerning(&font, &graph, &names)
            .unwrap_err()
            .contains("absent Value2"));
    }

    #[test]
    fn class_zero_complements_and_zero_cells_are_materialized_without_pair_expansion() {
        let (font, graph, names) = fixture(&[
            2,
            24,
            4,
            0,
            32,
            40,
            2,
            2,
            0,
            0,
            (-20_i16) as u16,
            (-80_i16) as u16,
            1,
            2,
            1,
            2,
            1,
            1,
            1,
            1,
            1,
            2,
            1,
            1,
        ]);
        let (_, pairs, groups) = kerning(&font, &graph, &names).unwrap();
        assert_eq!(
            groups.side1["public.kern1.FCImported0"],
            ["V".into()].into()
        );
        assert_eq!(
            groups.side2["public.kern2.FCImported0"],
            [
                ".notdef".into(),
                "A".into(),
                "acute".into(),
                "ligature".into(),
                "grave".into()
            ]
            .into()
        );
        assert_eq!(pairs.0.len(), 4);
        assert_eq!(pairs.0.values().filter(|value| **value == 0.0).count(), 2);
        assert!(pairs.0.keys().all(|(left, right)| matches!(
            (left, right),
            (KernSide::Group(_), KernSide::Group(_))
        )));
    }

    #[test]
    fn glyph_zero_adjustment_remains_an_explicit_pair() {
        let (font, graph, names) = fixture(&[1, 18, 4, 0, 1, 12, 1, 2, 0, 1, 1, 1]);
        let (_, pairs, _) = kerning(&font, &graph, &names).unwrap();
        assert_eq!(
            pairs
                .0
                .get(&(KernSide::Glyph("A".into()), KernSide::Glyph("V".into()))),
            Some(&0.0)
        );
    }
}
