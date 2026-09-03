use fw_model::{Component, Font, GlyphName, Guide, LayerId, LayerKind, MasterId};

use crate::Error;

use super::{
    apply::{checked_indices, layer_mut, remove_indices, replace_layer_outline},
    Command,
};

pub(super) fn apply(font: &mut Font, command: &Command) -> Result<(), Error> {
    match command {
        Command::AddComponent {
            glyph,
            layer,
            component,
        } => mutate_components(font, glyph, layer, |components| {
            components.push(component.clone());
            Ok(())
        }),
        Command::SetComponent {
            glyph,
            layer,
            index,
            component,
        } => mutate_components(font, glyph, layer, |components| {
            let len = components.len();
            let target = components
                .get_mut(*index)
                .ok_or_else(|| index_error("component", *index, len))?;
            *target = component.clone();
            Ok(())
        }),
        Command::RemoveComponents {
            glyph,
            layer,
            indices,
        } => mutate_components(font, glyph, layer, |components| {
            let indices = checked_indices(indices, components.len(), "component")?;
            remove_indices(components, &indices);
            Ok(())
        }),
        Command::DecomposeComponents {
            glyph,
            layer,
            indices,
        } => decompose(font, glyph, layer, indices),
        Command::AddAnchor {
            glyph,
            layer,
            anchor,
        } => {
            layer_mut(font, glyph, layer)?.anchors.push(anchor.clone());
            Ok(())
        }
        Command::SetAnchor {
            glyph,
            layer,
            index,
            anchor,
        } => {
            let anchors = &mut layer_mut(font, glyph, layer)?.anchors;
            let len = anchors.len();
            let target = anchors
                .get_mut(*index)
                .ok_or_else(|| index_error("anchor", *index, len))?;
            *target = anchor.clone();
            Ok(())
        }
        Command::RemoveAnchors {
            glyph,
            layer,
            indices,
        } => {
            let anchors = &mut layer_mut(font, glyph, layer)?.anchors;
            let indices = checked_indices(indices, anchors.len(), "anchor")?;
            remove_indices(anchors, &indices);
            Ok(())
        }
        Command::AddGuide {
            glyph,
            layer,
            master,
            guide,
        } => {
            guide_vec(font, glyph, layer, master)?.push(guide.clone());
            Ok(())
        }
        Command::SetGuide {
            glyph,
            layer,
            master,
            index,
            guide,
        } => {
            let guides = guide_vec(font, glyph, layer, master)?;
            let len = guides.len();
            let target = guides
                .get_mut(*index)
                .ok_or_else(|| index_error("guide", *index, len))?;
            *target = guide.clone();
            Ok(())
        }
        Command::RemoveGuide {
            glyph,
            layer,
            master,
            index,
        } => {
            let guides = guide_vec(font, glyph, layer, master)?;
            if *index >= guides.len() {
                return Err(index_error("guide", *index, guides.len()));
            }
            guides.remove(*index);
            Ok(())
        }
        _ => Err(Error::Internal(
            "non-structure command routed to structure handler".to_owned(),
        )),
    }
}

fn mutate_components(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    mutate: impl FnOnce(&mut Vec<Component>) -> Result<(), Error>,
) -> Result<(), Error> {
    let target = layer_mut(font, glyph, layer)?;
    let contours = target.contours.clone();
    let mut components = target.components.clone();
    let anchors = target.anchors.clone();
    mutate(&mut components)?;
    replace_layer_outline(font, glyph, layer, &contours, &components, &anchors)
}

fn decompose(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    requested: &[usize],
) -> Result<(), Error> {
    let target = font
        .glyph(glyph)
        .and_then(|glyph| glyph.layer(layer))
        .ok_or_else(|| Error::MissingLayer {
            glyph: glyph.clone(),
            layer: layer.clone(),
        })?;
    let master = master_for_layer(font, &target.kind);
    let indices = checked_indices(requested, target.components.len(), "component")?;
    let selected = indices
        .iter()
        .map(|&index| target.components[index].clone())
        .collect::<Vec<_>>();
    let mut contours = target.contours.clone();
    let mut components = target.components.clone();
    let anchors = target.anchors.clone();
    for component in selected {
        let path = fw_model::flatten_layer(font, &component.base, &master)?;
        let source = fw_model::Contour::from_bezpath(&path);
        contours.extend(fw_geom::transform(&source, component.transform));
    }
    remove_indices(&mut components, &indices);
    replace_layer_outline(font, glyph, layer, &contours, &components, &anchors)
}

fn master_for_layer(font: &Font, kind: &LayerKind) -> MasterId {
    match kind {
        LayerKind::Master(master)
        | LayerKind::Background(master)
        | LayerKind::Brace { master, .. }
        | LayerKind::Bracket { master, .. } => master.clone(),
        LayerKind::Backup { of, .. } => of.clone(),
        LayerKind::Custom(_) => font.default_master().id.clone(),
    }
}

fn guide_vec<'a>(
    font: &'a mut Font,
    glyph: &Option<GlyphName>,
    layer: &Option<LayerId>,
    master: &Option<MasterId>,
) -> Result<&'a mut Vec<Guide>, Error> {
    match (glyph, layer, master) {
        (Some(glyph), Some(layer), None) => Ok(&mut layer_mut(font, glyph, layer)?.guides),
        (None, None, Some(master)) => font
            .masters
            .iter_mut()
            .find(|candidate| candidate.id == *master)
            .map(|master| &mut master.guides)
            .ok_or_else(|| Error::MissingMaster(master.clone())),
        (None, None, None) => Ok(&mut font.guides),
        _ => Err(Error::InvalidCommand(
            "guide scope must be glyph+layer, master, or font".to_owned(),
        )),
    }
}

fn index_error(what: &str, index: usize, len: usize) -> Error {
    Error::InvalidCommand(format!(
        "{what} index {index} is out of bounds for length {len}"
    ))
}
