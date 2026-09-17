use crate::Error;
use fc_model::{Font, GlyphName, KernSide};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use ts_rs::TS;

/// Logical pair position of a horizontal kerning group.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum KerningGroupSide {
    /// First glyph in a pair; canonical `public.kern1.*` names.
    Side1,
    /// Second glyph in a pair; canonical `public.kern2.*` names.
    Side2,
}

impl KerningGroupSide {
    /// Canonical UFO prefix for this position.
    pub fn prefix(self) -> &'static str {
        match self {
            Self::Side1 => "public.kern1.",
            Self::Side2 => "public.kern2.",
        }
    }
    /// Authoritative group membership for this position.
    pub fn groups(self, font: &Font) -> &BTreeMap<String, BTreeSet<GlyphName>> {
        match self {
            Self::Side1 => &font.kerning_groups.side1,
            Self::Side2 => &font.kerning_groups.side2,
        }
    }
    pub(crate) fn groups_mut(self, font: &mut Font) -> &mut BTreeMap<String, BTreeSet<GlyphName>> {
        match self {
            Self::Side1 => &mut font.kerning_groups.side1,
            Self::Side2 => &mut font.kerning_groups.side2,
        }
    }
    pub(crate) fn validate_name(self, name: &str) -> Result<(), Error> {
        if name
            .strip_prefix(self.prefix())
            .is_none_or(|tail| tail.trim().is_empty())
            || name.chars().any(char::is_control)
        {
            return Err(Error::InvalidCommand(format!(
                "invalid kerning group `{name}`; expected {} and a name",
                self.prefix()
            )));
        }
        Ok(())
    }
    pub(crate) fn set_membership(self, font: &mut Font, glyph: &GlyphName, group: Option<String>) {
        if let Some(glyph) = font.glyph_mut(glyph) {
            match self {
                Self::Side1 => glyph.kern_group_side1 = group,
                Self::Side2 => glyph.kern_group_side2 = group,
            }
        }
    }
}

/// Resolve a glyph's authoritative membership, rejecting ambiguous side data.
pub fn group_membership<'a>(
    font: &'a Font,
    glyph: &GlyphName,
    side: KerningGroupSide,
) -> Result<Option<&'a str>, Error> {
    if font.glyph(glyph).is_none() {
        return Err(Error::MissingGlyph(glyph.clone()));
    }
    super::membership(side.groups(font), glyph, side.prefix())
}
/// Validate one group's canonical name, glyph references and side membership.
pub fn validate_group(font: &Font, side: KerningGroupSide, name: &str) -> Result<(), Error> {
    side.validate_name(name)?;
    let members = side
        .groups(font)
        .get(name)
        .ok_or_else(|| Error::InvalidCommand(format!("kerning group `{name}` does not exist")))?;
    for glyph in members {
        group_membership(font, glyph, side)?;
    }
    Ok(())
}
pub(crate) fn validate_pair_side(
    font: &Font,
    value: &KernSide,
    side: KerningGroupSide,
) -> Result<(), Error> {
    match value {
        KernSide::Glyph(name) => {
            group_membership(font, name, side)?;
            Ok(())
        }
        KernSide::Group(name) => validate_group(font, side, name),
    }
}
