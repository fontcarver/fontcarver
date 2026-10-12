use fc_model::{Font, GlyphName, Kerning, KerningGroups, MasterId};

/// Kerning-only undo data, independent of glyph outlines, covering both pair directions.
#[derive(Clone, Debug)]
pub struct HorizontalKerningSnapshot {
    masters: Vec<(MasterId, Kerning, Kerning)>,
    groups: KerningGroups,
    memberships: Vec<(GlyphName, Option<String>, Option<String>)>,
}

impl PartialEq for HorizontalKerningSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.groups == other.groups
            && self.memberships == other.memberships
            && self.masters.len() == other.masters.len()
            && self.masters.iter().zip(&other.masters).all(
                |((left_id, left, left_v), (right_id, right, right_v))| {
                    left_id == right_id
                        && crate::kerning::same_records(left, right)
                        && crate::kerning::same_records(left_v, right_v)
                },
            )
    }
}

impl HorizontalKerningSnapshot {
    pub(super) fn capture(font: &Font) -> Self {
        Self {
            masters: font
                .masters
                .iter()
                .map(|master| {
                    (
                        master.id.clone(),
                        master.kerning_h.clone(),
                        master.kerning_v.clone(),
                    )
                })
                .collect(),
            groups: font.kerning_groups.clone(),
            memberships: font
                .glyphs
                .values()
                .map(|glyph| {
                    (
                        glyph.name.clone(),
                        glyph.kern_group_side1.clone(),
                        glyph.kern_group_side2.clone(),
                    )
                })
                .collect(),
        }
    }

    pub(super) fn restore(&self, font: &mut Font) {
        for (id, kerning, vertical) in &self.masters {
            if let Some(master) = font.masters.iter_mut().find(|master| &master.id == id) {
                master.kerning_h = kerning.clone();
                master.kerning_v = vertical.clone();
            }
        }
        font.kerning_groups = self.groups.clone();
        for (name, side1, side2) in &self.memberships {
            if let Some(glyph) = font.glyph_mut(name) {
                glyph.kern_group_side1 = side1.clone();
                glyph.kern_group_side2 = side2.clone();
            }
        }
    }

    pub(super) fn estimated_bytes(&self) -> usize {
        let pairs = self
            .masters
            .iter()
            .map(|(_, kerning, vertical)| kerning.0.len() + vertical.0.len())
            .sum::<usize>();
        let members = self
            .groups
            .side1
            .values()
            .chain(self.groups.side2.values())
            .map(|members| members.len())
            .sum::<usize>();
        pairs
            .saturating_mul(128)
            .saturating_add(members.saturating_mul(64))
            .saturating_add(self.memberships.len().saturating_mul(128))
    }
}
