use std::collections::{BTreeSet, HashSet};

use fw_model::{Font, Glyph, GlyphName, LayerId};

use crate::Error;

use super::apply::{glyph_mut, unique_names, validate_glyph_outlines};

pub(super) fn add(font: &mut Font, glyph: &Glyph) -> Result<(), Error> {
    validate_glyph_outlines(glyph)?;
    font.insert_glyph(glyph.clone()).map_err(Error::from)
}

pub(super) fn remove(font: &mut Font, names: &[GlyphName]) -> Result<(), Error> {
    let mut seen = HashSet::new();
    for name in names {
        if seen.insert(name) && font.glyph(name).is_none() {
            return Err(Error::MissingGlyph(name.clone()));
        }
    }
    for name in names {
        font.remove_glyph(name);
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
    font.insert_glyph(glyph).map_err(Error::from)
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
