//! Native class kerning matrices, bounded independently of declared counts.

use super::{error, integer, layout::Lookup, number, reader::find_lookup, strings, MAX_ITEMS};
use crate::Error;
use fc_model::{Font, GlyphName, KernSide};
use std::collections::BTreeSet;

pub(super) struct KernClass {
    left: Vec<Vec<GlyphName>>,
    right: Vec<Vec<GlyphName>>,
    values: Vec<f64>,
    pub(super) table: String,
}
pub(super) fn read_kern_class<'a>(
    value: &str,
    lines: &mut impl Iterator<Item = &'a str>,
) -> Result<KernClass, Error> {
    let t = strings::tokens(value)?;
    if t.len() != 4 || t[0].ends_with('+') {
        return Err(error("unsupported KernClass2 header/class zero"));
    }
    let left = integer(&t[0])?;
    let right = integer(&t[1])?;
    integer(&t[2])?;
    if left < 1 || right < 1 || left.checked_mul(right).is_none_or(|n| n > MAX_ITEMS as i64) {
        return Err(error("kerning class matrix exceeds limit"));
    }
    let mut groups = |count: i64| -> Result<Vec<Vec<GlyphName>>, Error> {
        let mut groups = vec![Vec::new()];
        for _ in 1..count {
            let line = lines
                .next()
                .ok_or_else(|| error("truncated kerning class members"))?;
            let (length, names) = line.split_once(' ').unwrap_or((line, ""));
            let length = integer(length)?;
            if length < 0 || length as usize != names.len() {
                return Err(error("kerning class string length mismatch"));
            }
            groups.push(names.split_whitespace().map(GlyphName::from).collect());
        }
        Ok(groups)
    };
    let left = groups(left)?;
    let right = groups(right)?;
    let mut values = Vec::new();
    while values.len() < left.len() * right.len() {
        let line = lines
            .next()
            .ok_or_else(|| error("truncated kerning matrix"))?;
        for value in line.split_whitespace() {
            values.push(number(value)?);
        }
    }
    if values.len() != left.len() * right.len() {
        return Err(error("kerning matrix count mismatch"));
    }
    Ok(KernClass {
        left,
        right,
        values,
        table: t[3].clone(),
    })
}
pub(super) fn install_kern_class(
    font: &mut Font,
    lookups: &[Lookup],
    class: KernClass,
) -> Result<(), Error> {
    if !find_lookup(lookups, &class.table)?.simple_kern() {
        return Err(error(
            "kerning classes with non-default script/flags are unsupported",
        ));
    }
    font.feature_settings.writer_policy.kerning =
        !find_lookup(lookups, &class.table)?.features.is_empty();
    let mut left_names = Vec::new();
    let mut right_names = Vec::new();
    for (groups, target, names, side) in [
        (
            &class.left,
            &mut font.kerning_groups.side1,
            &mut left_names,
            "public.kern1",
        ),
        (
            &class.right,
            &mut font.kerning_groups.side2,
            &mut right_names,
            "public.kern2",
        ),
    ] {
        let mut seen = BTreeSet::new();
        for (index, members) in groups.iter().enumerate() {
            let name = format!("{side}.SFD{index}");
            for member in members {
                if !font.glyphs.contains_key(member) || !seen.insert(member) {
                    return Err(error(
                        "missing or overlapping same-side kerning class membership",
                    ));
                }
            }
            if index > 0 {
                target.insert(
                    name.clone(),
                    members.iter().cloned().collect::<BTreeSet<_>>(),
                );
            }
            names.push(name);
        }
    }
    for (groups, side1) in [
        (&font.kerning_groups.side1, true),
        (&font.kerning_groups.side2, false),
    ] {
        for (name, members) in groups {
            for member in members {
                if let Some(glyph) = font.glyphs.get_mut(member) {
                    if side1 {
                        glyph.kern_group_side1 = Some(name.clone());
                    } else {
                        glyph.kern_group_side2 = Some(name.clone());
                    }
                }
            }
        }
    }
    for (left, _) in class.left.iter().enumerate() {
        for (right, _) in class.right.iter().enumerate() {
            let value = class.values[left * class.right.len() + right];
            if left == 0 || right == 0 {
                if value != 0.0 {
                    return Err(error("nonzero default class kerning is unsupported"));
                }
                continue;
            }
            if value != 0.0 {
                font.masters[0].kerning_h.0.insert(
                    (
                        KernSide::Group(left_names[left].clone()),
                        KernSide::Group(right_names[right].clone()),
                    ),
                    value,
                );
            }
        }
    }
    Ok(())
}
