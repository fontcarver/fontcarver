//! Whole-document validation.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{Font, GlyphName, KernSide, Kerning, MasterId};

/// A non-fatal document consistency issue.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum Issue {
    /// More than one glyph is assigned the same Unicode scalar value.
    DuplicateUnicode {
        /// Duplicated scalar value.
        unicode: u32,
        /// Glyphs carrying the value, in glyph order.
        glyphs: Vec<GlyphName>,
    },
    /// A component base or construction source is absent.
    DanglingComponent {
        /// Glyph containing the component.
        glyph: GlyphName,
        /// Missing component base.
        base: GlyphName,
    },
    /// A glyph lacks a main layer for one master.
    MissingMasterLayer {
        /// Affected glyph.
        glyph: GlyphName,
        /// Master without a layer.
        master: MasterId,
    },
    /// A kerning pair refers to a group that does not exist on that side.
    MissingKerningGroup {
        /// Master containing the pair.
        master: MasterId,
        /// Missing group name.
        group: String,
        /// Kerning side, either 1 or 2.
        side: u8,
        /// Whether the pair belongs to vertical kerning.
        vertical: bool,
    },
}

/// Validates cross-references and per-master layer coverage.
pub fn validate(font: &Font) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut unicode_owners: BTreeMap<u32, Vec<GlyphName>> = BTreeMap::new();

    for (name, glyph) in &font.glyphs {
        for unicode in &glyph.unicodes {
            unicode_owners
                .entry(*unicode)
                .or_default()
                .push(name.clone());
        }
        for master in &font.masters {
            if glyph.master_layer(&master.id).is_none() {
                issues.push(Issue::MissingMasterLayer {
                    glyph: name.clone(),
                    master: master.id.clone(),
                });
            }
        }
        for base in glyph.layers.iter().flat_map(crate::font::component_bases) {
            if font.glyph(base).is_none() {
                issues.push(Issue::DanglingComponent {
                    glyph: name.clone(),
                    base: base.clone(),
                });
            }
        }
    }
    issues.extend(
        unicode_owners
            .into_iter()
            .filter(|(_, glyphs)| glyphs.len() > 1)
            .map(|(unicode, glyphs)| Issue::DuplicateUnicode { unicode, glyphs }),
    );

    for master in &font.masters {
        validate_kerning(font, &master.id, &master.kerning_h, false, &mut issues);
        validate_kerning(font, &master.id, &master.kerning_v, true, &mut issues);
    }
    issues
}

fn validate_kerning(
    font: &Font,
    master: &MasterId,
    kerning: &Kerning,
    vertical: bool,
    issues: &mut Vec<Issue>,
) {
    for (left, right) in kerning.0.keys() {
        if let KernSide::Group(group) = left {
            if !font.kerning_groups.side1.contains_key(group) {
                issues.push(Issue::MissingKerningGroup {
                    master: master.clone(),
                    group: group.clone(),
                    side: 1,
                    vertical,
                });
            }
        }
        if let KernSide::Group(group) = right {
            if !font.kerning_groups.side2.contains_key(group) {
                issues.push(Issue::MissingKerningGroup {
                    master: master.clone(),
                    group: group.clone(),
                    side: 2,
                    vertical,
                });
            }
        }
    }
}
