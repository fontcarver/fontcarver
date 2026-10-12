//! Exclude unresolved source records from compilation without altering documents.
use crate::ExportWarning;
use fc_model::{Font, GlyphName, KernSide, Kerning, KerningGroups, MasterId};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct PreparedKerning {
    pub(crate) pairs: Kerning,
    pub(crate) groups: KerningGroups,
    pub(crate) warnings: Vec<ExportWarning>,
}

struct Side {
    groups: BTreeMap<String, BTreeSet<GlyphName>>,
    ambiguous: BTreeSet<GlyphName>,
    omitted: usize,
}

fn prepare_side(font: &Font, source: &BTreeMap<String, BTreeSet<GlyphName>>, prefix: &str) -> Side {
    let mut counts = BTreeMap::<&GlyphName, usize>::new();
    let valid_name = |name: &str| {
        name.strip_prefix(prefix)
            .is_some_and(|tail| !tail.trim().is_empty())
            && !name.chars().any(char::is_control)
    };
    let mut ambiguous = BTreeSet::new();
    for (name, members) in source {
        for member in members {
            *counts.entry(member).or_default() += 1;
            if !valid_name(name) {
                ambiguous.insert(member.clone());
            }
        }
    }
    ambiguous.extend(
        counts
            .into_iter()
            .filter(|(_, count)| *count > 1)
            .map(|(name, _)| name.clone()),
    );
    let groups = source
        .iter()
        .filter(|(name, members)| {
            valid_name(name)
                && members
                    .iter()
                    .all(|name| font.glyph(name).is_some() && !ambiguous.contains(name))
        })
        .map(|(name, members)| {
            (
                name.clone(),
                members
                    .iter()
                    .filter(|name| font.glyph(name).is_some_and(|glyph| glyph.export))
                    .cloned()
                    .collect(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    Side {
        omitted: source.len() - groups.len(),
        groups,
        ambiguous,
    }
}

impl Side {
    fn valid(&self, font: &Font, side: &KernSide) -> bool {
        match side {
            KernSide::Glyph(name) => {
                font.glyph(name).is_some_and(|glyph| glyph.export) && !self.ambiguous.contains(name)
            }
            KernSide::Group(name) => self.groups.contains_key(name),
        }
    }
}

pub(crate) fn prepare(font: &Font, master: &MasterId) -> PreparedKerning {
    let side1 = prepare_side(font, &font.kerning_groups.side1, "public.kern1.");
    let side2 = prepare_side(font, &font.kerning_groups.side2, "public.kern2.");
    let mut pairs = Kerning::default();
    let mut omitted_pairs = 0;
    if let Some(master) = font.master(master) {
        for ((left, right), value) in &master.kerning_h.0 {
            if value.is_finite() && side1.valid(font, left) && side2.valid(font, right) {
                pairs.0.insert((left.clone(), right.clone()), *value);
            } else {
                omitted_pairs += 1;
            }
        }
    }
    let mut warnings = Vec::new();
    if omitted_pairs > 0 || side1.omitted + side2.omitted > 0 {
        warnings.push(ExportWarning {
            code: "invalid-horizontal-kerning",
            message: format!("Omitted {omitted_pairs} unresolved, non-exporting or non-finite horizontal kerning records and {} invalid groups; source records are retained", side1.omitted + side2.omitted),
        });
    }
    PreparedKerning {
        pairs,
        groups: KerningGroups {
            side1: side1.groups,
            side2: side2.groups,
        },
        warnings,
    }
}

/// Diagnostics for horizontal records excluded from compiled layout.
pub fn horizontal_kerning_warnings(font: &Font, master: &MasterId) -> Vec<ExportWarning> {
    prepare(font, master).warnings
}
