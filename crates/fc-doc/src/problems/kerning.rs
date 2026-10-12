use std::collections::BTreeMap;

use fc_model::{Font, KernSide};

use crate::kerning::KerningGroupSide;

use super::{Collector, Location};

pub(super) fn scan(font: &Font, out: &mut Collector) {
    for (side, groups) in [
        (1, &font.kerning_groups.side1),
        (2, &font.kerning_groups.side2),
    ] {
        let mut memberships = BTreeMap::new();
        for (group, members) in groups {
            let position = if side == 1 {
                KerningGroupSide::Side1
            } else {
                KerningGroupSide::Side2
            };
            if let Err(error) = position.validate_name(group) {
                out.error(
                    "invalid-kerning-group",
                    error.to_string(),
                    &Location::default(),
                );
            }
            for member in members {
                let at = Location::glyph(member);
                let Some(glyph) = font.glyphs.get(member) else {
                    out.error(
                        "missing-group-member",
                        format!("Side {side} group {group} contains missing glyph {member}."),
                        &Location::default(),
                    );
                    continue;
                };
                if let Some(previous) = memberships.insert(member, group) {
                    out.error(
                        "duplicate-group-membership",
                        format!("Glyph belongs to side {side} groups {previous} and {group}."),
                        &at,
                    );
                }
                let declared = if side == 1 {
                    &glyph.kern_group_side1
                } else {
                    &glyph.kern_group_side2
                };
                if declared.as_ref() != Some(group) {
                    out.error("inconsistent-group-membership", format!("Side {side} group {group} disagrees with the glyph's group assignment."), &at);
                }
            }
        }
        for (name, glyph) in &font.glyphs {
            let declared = if side == 1 {
                &glyph.kern_group_side1
            } else {
                &glyph.kern_group_side2
            };
            if let Some(group) = declared {
                if !groups
                    .get(group)
                    .is_some_and(|members| members.contains(name))
                {
                    out.error("inconsistent-group-membership", format!("Glyph declares side {side} group {group} without matching group membership."), &Location::glyph(name));
                }
            }
        }
    }
    for master in &font.masters {
        let at = Location {
            master: Some(master.id.clone()),
            ..Location::default()
        };
        for (direction, kerning) in [
            ("horizontal", &master.kerning_h),
            ("vertical", &master.kerning_v),
        ] {
            for ((left, right), value) in &kerning.0 {
                if !value.is_finite() {
                    out.error(
                        "invalid-kerning-value",
                        format!("A {direction} kerning pair has a nonfinite adjustment."),
                        &at,
                    );
                }
                for (side, key, groups) in [
                    (1, left, &font.kerning_groups.side1),
                    (2, right, &font.kerning_groups.side2),
                ] {
                    match key {
                        KernSide::Glyph(name) if !font.glyphs.contains_key(name) => out.error("missing-kerning-glyph", format!("A {direction} side {side} kerning pair refers to missing glyph {name}."), &at),
                        KernSide::Group(name) if !groups.contains_key(name) => out.error("missing-kerning-group", format!("A {direction} side {side} kerning pair refers to missing group {name}."), &at),
                        _ => {}
                    }
                }
            }
        }
    }
}
