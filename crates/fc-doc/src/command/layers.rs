use fc_model::{Font, GlyphName, LayerId, LayerKind};

use crate::Error;

use super::{apply::glyph_mut, Command};

pub(super) fn apply(font: &mut Font, command: &Command) -> Result<(), Error> {
    match command {
        Command::SetLayerEffects {
            glyph,
            layer,
            effects,
        } => {
            let target = super::apply::layer_mut(font, glyph, layer)?;
            let mut proposed = target.clone();
            proposed.effects.clone_from(effects);
            proposed.validate_effects()?;
            target.effects.clone_from(effects);
            Ok(())
        }
        Command::AddLayer { glyph, layer } => add(font, glyph, layer),
        Command::RemoveLayer { glyph, layer } => remove(font, glyph, layer),
        Command::CopyLayer { glyph, from, to } => copy(font, glyph, from, to),
        Command::SwapWithBackground { glyph, master } => swap(font, glyph, master),
        Command::SetMaster { master } => {
            let current = font
                .master(&master.id)
                .ok_or_else(|| Error::MissingMaster(master.id.clone()))?;
            if !crate::kerning::same_records(&current.kerning_h, &master.kerning_h) {
                crate::kerning::check_horizontal_editing(font)?;
                for ((left, right), value) in &master.kerning_h.0 {
                    super::apply::ensure_finite(&[*value], "kerning")?;
                    crate::kerning::validate_pair_side(
                        font,
                        left,
                        crate::kerning::KerningGroupSide::Side1,
                    )?;
                    crate::kerning::validate_pair_side(
                        font,
                        right,
                        crate::kerning::KerningGroupSide::Side2,
                    )?;
                }
            }
            let target = font
                .masters
                .iter_mut()
                .find(|candidate| candidate.id == master.id)
                .ok_or_else(|| Error::MissingMaster(master.id.clone()))?;
            *target = master.clone();
            Ok(())
        }
        Command::SetFontGuides { guides } => {
            font.guides.clone_from(guides);
            Ok(())
        }
        _ => Err(Error::Internal(
            "non-layer command routed to layer handler".to_owned(),
        )),
    }
}

fn add(font: &mut Font, glyph_name: &GlyphName, layer: &fc_model::Layer) -> Result<(), Error> {
    if matches!(layer.kind, LayerKind::Master(_)) {
        return Err(Error::InvalidCommand(
            "AddLayer cannot add a master layer".to_owned(),
        ));
    }
    for contour in &layer.contours {
        contour.try_to_bezpath()?;
    }
    let position = font
        .glyphs
        .get_index_of(glyph_name)
        .ok_or_else(|| Error::MissingGlyph(glyph_name.clone()))?;
    let original = font
        .glyph(glyph_name)
        .cloned()
        .ok_or_else(|| Error::MissingGlyph(glyph_name.clone()))?;
    if original
        .layers
        .iter()
        .any(|candidate| candidate.id == layer.id)
    {
        return Err(Error::InvalidCommand(format!(
            "layer `{}` already exists",
            layer.id
        )));
    }
    let mut replacement = original.clone();
    replacement.layers.push(layer.clone());
    font.remove_glyph(glyph_name);
    if let Err(error) = font.insert_glyph(replacement) {
        font.insert_glyph(original).map_err(|restore| {
            Error::Internal(format!(
                "failed to restore glyph after `{error}`: {restore}"
            ))
        })?;
        move_to(font, glyph_name, position);
        return Err(error.into());
    }
    move_to(font, glyph_name, position);
    Ok(())
}

fn remove(font: &mut Font, glyph_name: &GlyphName, layer_id: &LayerId) -> Result<(), Error> {
    let glyph = glyph_mut(font, glyph_name)?;
    let index = glyph
        .layers
        .iter()
        .position(|layer| layer.id == *layer_id)
        .ok_or_else(|| Error::MissingLayer {
            glyph: glyph_name.clone(),
            layer: layer_id.clone(),
        })?;
    if matches!(glyph.layers[index].kind, LayerKind::Master(_)) {
        return Err(Error::InvalidCommand(
            "master layers cannot be removed".to_owned(),
        ));
    }
    glyph.layers.remove(index);
    Ok(())
}

fn copy(
    font: &mut Font,
    glyph_name: &GlyphName,
    from: &LayerId,
    to: &LayerKind,
) -> Result<(), Error> {
    if !matches!(to, LayerKind::Background(_) | LayerKind::Backup { .. }) {
        return Err(Error::InvalidCommand(
            "CopyLayer destination must be Background or Backup".to_owned(),
        ));
    }
    let glyph = glyph_mut(font, glyph_name)?;
    let mut copy = glyph
        .layers
        .iter()
        .find(|layer| layer.id == *from)
        .cloned()
        .ok_or_else(|| Error::MissingLayer {
            glyph: glyph_name.clone(),
            layer: from.clone(),
        })?;
    copy.kind = to.clone();
    if matches!(to, LayerKind::Background(_)) {
        if let Some(index) = glyph.layers.iter().position(|layer| layer.kind == *to) {
            copy.id = glyph.layers[index].id.clone();
            glyph.layers[index] = copy;
        } else {
            copy.id = LayerId::new();
            glyph.layers.push(copy);
        }
    } else {
        copy.id = LayerId::new();
        glyph.layers.push(copy);
    }
    Ok(())
}

fn swap(font: &mut Font, glyph_name: &GlyphName, master: &fc_model::MasterId) -> Result<(), Error> {
    let glyph = glyph_mut(font, glyph_name)?;
    let master_index = glyph
        .layers
        .iter()
        .position(|layer| matches!(&layer.kind, LayerKind::Master(id) if id == master))
        .ok_or_else(|| {
            Error::InvalidCommand(format!(
                "glyph `{glyph_name}` has no master layer `{master}`"
            ))
        })?;
    let background_index = glyph
        .layers
        .iter()
        .position(|layer| matches!(&layer.kind, LayerKind::Background(id) if id == master))
        .ok_or_else(|| {
            Error::InvalidCommand(format!(
                "glyph `{glyph_name}` has no background for `{master}`"
            ))
        })?;
    let (master_layer, background_layer) =
        two_mut(&mut glyph.layers, master_index, background_index);
    std::mem::swap(&mut master_layer.contours, &mut background_layer.contours);
    std::mem::swap(&mut master_layer.effects, &mut background_layer.effects);
    std::mem::swap(
        &mut master_layer.components,
        &mut background_layer.components,
    );
    std::mem::swap(&mut master_layer.anchors, &mut background_layer.anchors);
    Ok(())
}

fn two_mut<T>(values: &mut [T], a: usize, b: usize) -> (&mut T, &mut T) {
    debug_assert_ne!(a, b);
    if a < b {
        let (left, right) = values.split_at_mut(b);
        (&mut left[a], &mut right[0])
    } else {
        let (left, right) = values.split_at_mut(a);
        (&mut right[0], &mut left[b])
    }
}

fn move_to(font: &mut Font, glyph: &GlyphName, position: usize) {
    if let Some(current) = font.glyphs.get_index_of(glyph) {
        font.glyphs.move_index(current, position);
    }
}
