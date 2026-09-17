//! Stored horizontal kerning queries and source editing eligibility.

mod eligibility;
mod groups;

pub use eligibility::{check_horizontal_editing, HorizontalKerningEditBlock};
pub use groups::validate_group;
pub(crate) use groups::validate_pair_side;
pub use groups::{group_membership, KerningGroupSide};

use std::collections::{BTreeMap, BTreeSet};

use fc_model::{Font, GlyphName, KernPair, KernSide, Kerning, MasterId};

use crate::Error;

// Imported invalid values are retained verbatim. An unchanged NaN payload is
// the same stored record even though IEEE numeric equality considers it unequal.
pub(crate) fn same_records(left: &Kerning, right: &Kerning) -> bool {
    left.0.len() == right.0.len()
        && left
            .0
            .iter()
            .zip(&right.0)
            .all(|((left_key, left), (right_key, right))| {
                left_key == right_key && (left == right || left.to_bits() == right.to_bits())
            })
}

/// Validate one stored horizontal record independently of source ownership.
///
/// This is also usable when reporting invalid imported data without deleting it.
pub fn validate_horizontal_record(font: &Font, record: &KernPair) -> Result<(), Error> {
    if !record.value.is_finite() {
        return Err(Error::InvalidCommand("kerning must be finite".into()));
    }
    validate_pair_side(font, &record.left, KerningGroupSide::Side1)?;
    validate_pair_side(font, &record.right, KerningGroupSide::Side2)
}

/// Finds the stored horizontal rule supplying a concrete glyph pair's value.
///
/// Precedence is glyph/glyph, glyph/group, group/glyph, then group/group, per
/// <https://unifiedfontobject.org/versions/ufo3/kerning.plist/>. `None` means no
/// rule; a returned pair with value zero is an explicit override. Membership
/// comes from `Font.kerning_groups`, not the redundant glyph membership fields.
/// Pair sides are logical positions; the query does not reverse them for RTL.
///
/// Missing glyphs/masters, ambiguous or malformed membership for either queried
/// glyph, and a non-finite winning value return an error. Unrelated unresolved
/// records are left untouched; this is not a whole-font validation pass.
///
/// This low-level query does not inspect FEA or imported layout tables and does
/// not establish editing eligibility or the final shaped advance. API callers
/// must call [`check_horizontal_editing`] before exposing authoring controls.
pub fn resolve_stored_horizontal(
    font: &Font,
    first: &GlyphName,
    second: &GlyphName,
    master: &MasterId,
) -> Result<Option<KernPair>, Error> {
    let master = font
        .master(master)
        .ok_or_else(|| Error::MissingMaster(master.clone()))?;
    for name in [first, second] {
        if font.glyph(name).is_none() {
            return Err(Error::MissingGlyph(name.clone()));
        }
    }
    let first_group = membership(&font.kerning_groups.side1, first, "public.kern1.")?;
    let second_group = membership(&font.kerning_groups.side2, second, "public.kern2.")?;
    let first_glyph = KernSide::Glyph(first.clone());
    let second_glyph = KernSide::Glyph(second.clone());
    let candidates = [
        Some((first_glyph.clone(), second_glyph.clone())),
        second_group.map(|group| (first_glyph, KernSide::Group(group.to_owned()))),
        first_group.map(|group| (KernSide::Group(group.to_owned()), second_glyph)),
        first_group.zip(second_group).map(|(first, second)| {
            (
                KernSide::Group(first.to_owned()),
                KernSide::Group(second.to_owned()),
            )
        }),
    ];
    for pair in candidates.into_iter().flatten() {
        if let Some(&value) = master.kerning_h.0.get(&pair) {
            if !value.is_finite() {
                return Err(Error::InvalidCommand(format!(
                    "non-finite horizontal kerning for `{first}` / `{second}`"
                )));
            }
            let record = KernPair {
                left: pair.0,
                right: pair.1,
                value,
            };
            validate_horizontal_record(font, &record)?;
            return Ok(Some(record));
        }
    }
    Ok(None)
}

fn membership<'a>(
    groups: &'a BTreeMap<String, BTreeSet<GlyphName>>,
    glyph: &GlyphName,
    prefix: &str,
) -> Result<Option<&'a str>, Error> {
    let mut found = None;
    for (name, members) in groups {
        if !members.contains(glyph) {
            continue;
        }
        if name
            .strip_prefix(prefix)
            .is_none_or(|name| name.trim().is_empty())
            || name.chars().any(char::is_control)
        {
            return Err(Error::InvalidCommand(format!(
                "invalid kerning group `{name}` for `{glyph}`; expected `{prefix}` and a name"
            )));
        }
        if found.is_some() {
            return Err(Error::InvalidCommand(format!(
                "glyph `{glyph}` belongs to multiple `{prefix}` groups"
            )));
        }
        found = Some(name.as_str());
    }
    Ok(found)
}
