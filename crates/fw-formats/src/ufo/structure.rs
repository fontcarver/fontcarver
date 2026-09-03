//! Conversion of UFO groups, kerning, and feature text.

use fw_model::{
    FeatureBlock, Features, FeaturesSource, Font, GlyphName, KernSide, Kerning, KerningGroups,
};
use norad::Name;

use crate::Error;

const UFO_GROUPS_KEY: &str = "ufo.groups";

pub(crate) fn groups_from_norad(groups: &norad::Groups) -> KerningGroups {
    let mut result = KerningGroups::default();
    for (name, members) in groups {
        let members = members
            .iter()
            .map(|member| GlyphName::from(member.to_string()))
            .collect();
        if name.starts_with("public.kern1.") {
            result.side1.insert(name.to_string(), members);
        } else if name.starts_with("public.kern2.") {
            result.side2.insert(name.to_string(), members);
        }
    }
    result
}

pub(crate) fn groups_to_norad(font: &Font) -> Result<norad::Groups, Error> {
    let mut groups = norad::Groups::default();
    for (name, members) in font
        .kerning_groups
        .side1
        .iter()
        .chain(&font.kerning_groups.side2)
    {
        groups.insert(
            valid_name(name)?,
            members
                .iter()
                .map(|name| valid_name(name.as_ref()))
                .collect::<Result<_, _>>()?,
        );
    }
    if let Some(other) = font
        .extra
        .lib
        .get(UFO_GROUPS_KEY)
        .and_then(serde_json::Value::as_object)
    {
        for (name, members) in other {
            let members = members
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .map(valid_name)
                .collect::<Result<Vec<_>, _>>()?;
            groups.insert(valid_name(name)?, members);
        }
    }
    Ok(groups)
}

pub(crate) fn kerning_from_norad(kerning: &norad::Kerning) -> Kerning {
    Kerning(
        kerning
            .iter()
            .flat_map(|(left, rights)| {
                rights
                    .iter()
                    .map(move |(right, value)| ((kern_side(left), kern_side(right)), *value))
            })
            .collect(),
    )
}

pub(crate) fn kerning_to_norad(kerning: &Kerning) -> Result<norad::Kerning, Error> {
    let mut result = norad::Kerning::default();
    for ((left, right), value) in &kerning.0 {
        result
            .entry(kern_name(left)?)
            .or_default()
            .insert(kern_name(right)?, *value);
    }
    Ok(result)
}

fn kern_side(name: &Name) -> KernSide {
    if name.starts_with("public.kern1.") || name.starts_with("public.kern2.") {
        KernSide::Group(name.to_string())
    } else {
        KernSide::Glyph(GlyphName::from(name.to_string()))
    }
}

fn kern_name(side: &KernSide) -> Result<Name, Error> {
    match side {
        KernSide::Glyph(name) => valid_name(name.as_ref()),
        KernSide::Group(name) => valid_name(name),
    }
}

pub(crate) fn features_from_norad(features: &str) -> Features {
    Features {
        features: (!features.is_empty())
            .then(|| FeatureBlock {
                tag: "fea".to_owned(),
                code: features.to_owned(),
                automatic: false,
                ..FeatureBlock::default()
            })
            .into_iter()
            .collect(),
        source_of_truth: FeaturesSource::Fea,
        ..Features::default()
    }
}

pub(crate) fn features_to_norad(features: &Features) -> String {
    features
        .prefixes
        .iter()
        .filter(|block| !block.disabled)
        .map(|block| block.code.as_str())
        .chain(features.classes.iter().map(|class| class.code.as_str()))
        .chain(
            features
                .features
                .iter()
                .filter(|block| !block.disabled)
                .map(|block| block.code.as_str()),
        )
        .collect::<Vec<_>>()
        .join("\n")
}

fn valid_name(value: &str) -> Result<Name, Error> {
    Name::new(value).map_err(|_| Error::InvalidName(value.to_owned()))
}
