use std::collections::BTreeSet;

use fc_model::{Font, GlyphName, Layer, LayerId, LayerKind, MasterId};

use crate::Error;

use super::{apply::glyph_mut, Command};

pub(super) fn apply(font: &mut Font, command: &Command) -> Result<(), Error> {
    match command {
        Command::SetLayerColor {
            glyph,
            layer,
            color,
        } => set_color(font, glyph, layer, *color),
        Command::SetLayerImage {
            glyph,
            layer,
            image,
        } => {
            if font.glyph(glyph).is_some_and(|g| g.locked) {
                return Err(Error::InvalidCommand("the image glyph is locked".into()));
            }
            let target = super::apply::layer_mut(font, glyph, layer)?;
            if let Some(current) = target.background_image.as_ref().filter(|i| i.locked) {
                let mut unlocked = current.clone();
                unlocked.locked = false;
                if image.as_ref() != Some(&unlocked) && image.as_ref() != Some(current) {
                    return Err(Error::InvalidCommand("the image is locked".into()));
                }
            }
            if let Some(image) = image {
                super::apply::ensure_finite(&image.transform.as_coeffs(), "image transform")?;
                if image
                    .transform
                    .as_coeffs()
                    .iter()
                    .any(|v| v.abs() > 1_000_000.)
                    || image.transform.determinant().abs() < 1e-12
                    || image.file_name.len() > 4096
                    || image.file_name.contains('\0')
                    || image
                        .data
                        .as_ref()
                        .is_some_and(|d| d.len() > 16 * 1024 * 1024)
                {
                    return Err(Error::InvalidCommand(
                        "invalid source image placement or size".into(),
                    ));
                }
            }
            target.background_image.clone_from(image);
            Ok(())
        }
        Command::SetConstructions {
            glyph,
            layer,
            constructions,
        } => {
            let target = super::apply::layer_mut(font, glyph, layer)?;
            let mut proposed = target.clone();
            proposed.constructions.clone_from(constructions);
            proposed.validate_constructions()?;
            target.constructions.clone_from(constructions);
            Ok(())
        }
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
        Command::RenameLayer { glyph, layer, name } => rename(font, glyph, layer, name),
        Command::RemoveLayer { glyph, layer } => remove(font, glyph, layer),
        Command::CopyLayer { glyph, from, to } => copy(font, glyph, from, to),
        Command::CopySelectionToBackground {
            glyph,
            from,
            master,
            contours,
            components,
        } => copy_selection(font, glyph, from, master, contours, components),
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

fn set_color(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    color: Option<u8>,
) -> Result<(), Error> {
    if color.is_some_and(|value| value > 11) {
        return Err(Error::InvalidCommand(
            "layer color label must be between 0 and 11".into(),
        ));
    }
    let target = super::apply::layer_mut(font, glyph, layer)?;
    if target.user_data.is_null() {
        if color.is_none() {
            return Ok(());
        }
        target.user_data = serde_json::json!({});
    }
    let data = target.user_data.as_object_mut().ok_or_else(|| {
        Error::InvalidCommand("layer user data must be an object to store a color label".into())
    })?;
    if let Some(value) = color {
        data.insert("com.fontcarver.colorLabel".into(), serde_json::json!(value));
    } else {
        data.remove("com.fontcarver.colorLabel");
    }
    Ok(())
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

fn rename(
    font: &mut Font,
    glyph_name: &GlyphName,
    layer_id: &LayerId,
    name: &str,
) -> Result<(), Error> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::InvalidCommand(
            "layer name cannot be empty".to_owned(),
        ));
    }
    super::apply::layer_mut(font, glyph_name, layer_id)?.name = name.to_owned();
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

fn copy_selection(
    font: &mut Font,
    glyph_name: &GlyphName,
    from: &LayerId,
    master: &MasterId,
    contours: &[usize],
    components: &[usize],
) -> Result<(), Error> {
    if contours.is_empty() && components.is_empty() {
        return Err(Error::InvalidCommand(
            "select at least one path to copy".to_owned(),
        ));
    }
    if font.master(master).is_none() {
        return Err(Error::MissingMaster(master.clone()));
    }
    let glyph = glyph_mut(font, glyph_name)?;
    let source = glyph
        .layers
        .iter()
        .find(|layer| layer.id == *from)
        .ok_or_else(|| Error::MissingLayer {
            glyph: glyph_name.clone(),
            layer: from.clone(),
        })?;
    let destination = LayerKind::Background(master.clone());
    if source.kind == destination {
        return Err(Error::InvalidCommand(
            "source and destination background must be different layers".to_owned(),
        ));
    }
    let mut contours = selected(&source.contours, contours, "contour")?;
    // These are independent paths: retaining source identities can collide with
    // earlier copies and accidentally bind them to destination locks or links.
    for contour in &mut contours {
        contour.id = None;
        for point in &mut contour.points {
            point.id = None;
        }
    }
    let components = selected(&source.components, components, "component")?;
    let new_background = Layer {
        kind: destination.clone(),
        width: source.width,
        vertical_width: source.vertical_width,
        vertical_origin: source.vertical_origin,
        ..Layer::default()
    };
    if let Some(background) = glyph
        .layers
        .iter_mut()
        .find(|layer| layer.kind == destination)
    {
        background.contours.extend(contours);
        background.components.extend(components);
    } else {
        glyph.layers.push(Layer {
            contours,
            components,
            ..new_background
        });
    }
    Ok(())
}

fn selected<T: Clone>(values: &[T], indices: &[usize], kind: &str) -> Result<Vec<T>, Error> {
    indices
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|index| {
            values.get(index).cloned().ok_or_else(|| {
                Error::InvalidCommand(format!("{kind} index {index} is out of bounds"))
            })
        })
        .collect()
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
        &mut master_layer.constructions,
        &mut background_layer.constructions,
    );
    std::mem::swap(
        &mut master_layer.node_links,
        &mut background_layer.node_links,
    );
    std::mem::swap(
        &mut master_layer.linked_guides,
        &mut background_layer.linked_guides,
    );
    std::mem::swap(&mut master_layer.guides, &mut background_layer.guides);
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
