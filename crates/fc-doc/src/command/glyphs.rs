use std::collections::{BTreeSet, HashSet};

use fc_model::{Font, Glyph, GlyphName, KernSide, LayerId};

use crate::Error;

use super::apply::{glyph_mut, unique_names, validate_glyph_outlines};

pub(super) fn add(font: &mut Font, glyph: &Glyph) -> Result<(), Error> {
    validate_glyph_outlines(glyph)?;
    font.insert_glyph(glyph.clone()).map_err(Error::from)
}

pub(super) fn rename(font: &mut Font, from: &GlyphName, to: &GlyphName) -> Result<(), Error> {
    if from != to && font.glyph(from).is_some() && font.glyph(to).is_none() {
        let paired = font
            .masters
            .iter()
            .flat_map(|master| master.kerning_h.0.keys().chain(master.kerning_v.0.keys()))
            .any(|(left, right)| {
                matches!(left, KernSide::Glyph(name) if name == to)
                    || matches!(right, KernSide::Glyph(name) if name == to)
            });
        let grouped = font
            .kerning_groups
            .side1
            .values()
            .chain(font.kerning_groups.side2.values())
            .any(|members| members.contains(to));
        if paired || grouped {
            return Err(Error::InvalidCommand(format!(
                "glyph `{to}` already has unresolved kerning references; rename would change existing source records"
            )));
        }
    }
    font.rename_glyph(from, to).map_err(Error::from)
}

pub(super) fn remove(font: &mut Font, names: &[GlyphName]) -> Result<(), Error> {
    let mut seen = HashSet::new();
    for name in names {
        if seen.insert(name) && font.glyph(name).is_none() {
            return Err(Error::MissingGlyph(name.clone()));
        }
    }
    // Resolve dependants before removing any referenced image, preserving their pixels.
    for strike in &mut font.bitmap_strikes {
        let affected: Vec<_> = strike
            .glyphs
            .iter()
            .filter(|(name, glyph)| {
                !seen.contains(name) && glyph.refs.iter().any(|r| seen.contains(&r.base))
            })
            .map(|(name, _)| name.clone())
            .collect();
        let resolved = affected
            .iter()
            .map(|name| Ok((name.clone(), strike.flattened(name)?)))
            .collect::<Result<Vec<_>, fc_model::Error>>()?;
        for (name, glyph) in resolved {
            strike.glyphs.insert(name, glyph);
        }
        for name in names {
            strike.glyphs.remove(name);
        }
    }
    if let Some(cid) = &mut font.cid {
        cid.cid_map.retain(|_, name| !seen.contains(name));
    }
    for name in names {
        font.remove_glyph(name);
        for members in font
            .kerning_groups
            .side1
            .values_mut()
            .chain(font.kerning_groups.side2.values_mut())
        {
            members.remove(name);
        }
        for master in &mut font.masters {
            for pairs in [&mut master.kerning_h, &mut master.kerning_v] {
                pairs.0.retain(|(left, right), _| {
                    !matches!(left, fc_model::KernSide::Glyph(glyph) if glyph == name)
                        && !matches!(right, fc_model::KernSide::Glyph(glyph) if glyph == name)
                });
            }
        }
    }
    Ok(())
}

pub(super) fn set_meta(
    font: &mut Font,
    name: &GlyphName,
    export: &Option<bool>,
    color_label: &Option<Option<u8>>,
    tags: &Option<BTreeSet<String>>,
    note: &Option<String>,
) -> Result<(), Error> {
    let glyph = glyph_mut(font, name)?;
    if let Some(value) = export {
        glyph.export = *value;
    }
    if let Some(value) = color_label {
        glyph.color_label = *value;
    }
    if let Some(value) = tags {
        glyph.tags.clone_from(value);
    }
    if let Some(value) = note {
        glyph.note.clone_from(value);
    }
    Ok(())
}

pub(super) fn duplicate(
    font: &mut Font,
    source: &GlyphName,
    name: &GlyphName,
) -> Result<(), Error> {
    let mut glyph = font
        .glyph(source)
        .cloned()
        .ok_or_else(|| Error::MissingGlyph(source.clone()))?;
    glyph.name = name.clone();
    glyph.unicodes.clear();
    glyph.variation_selectors.clear();
    for layer in &mut glyph.layers {
        layer.id = LayerId::new();
    }
    font.insert_glyph(glyph).map_err(Error::from)?;
    for strike in &mut font.bitmap_strikes {
        if let Some(bitmap) = strike.glyphs.get(source).cloned() {
            strike.glyphs.insert(name.clone(), bitmap);
        }
    }
    Ok(())
}

pub(super) fn set_order(font: &mut Font, order: &[GlyphName]) -> Result<(), Error> {
    if order.len() != font.glyphs.len()
        || !unique_names(order)
        || order.iter().any(|name| font.glyph(name).is_none())
    {
        return Err(Error::InvalidCommand(
            "glyph order must contain every glyph exactly once".to_owned(),
        ));
    }
    for (target, name) in order.iter().enumerate() {
        let current = font
            .glyphs
            .get_index_of(name)
            .ok_or_else(|| Error::MissingGlyph(name.clone()))?;
        font.glyphs.move_index(current, target);
    }
    Ok(())
}
