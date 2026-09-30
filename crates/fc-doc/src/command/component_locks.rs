//! Preflight protection for locked outline component references.

use fc_model::{Component, Font, GlyphName, Layer, LayerId, LayerKind};

use crate::Error;

use super::{Command, DesignSpaceCommand};

pub(super) fn preflight(font: &Font, command: &Command) -> Result<(), Error> {
    match command {
        Command::SetComponent {
            glyph,
            layer: id,
            index,
            component,
        } => {
            if let Some(current) = layer(font, glyph, id).and_then(|l| l.components.get(*index)) {
                if current.locked {
                    let mut unlocked = current.clone();
                    unlocked.locked = false;
                    if component != &unlocked {
                        return Err(locked_error());
                    }
                }
            }
        }
        Command::RemoveComponents {
            glyph,
            layer: id,
            indices,
        }
        | Command::DecomposeComponents {
            glyph,
            layer: id,
            indices,
        } => {
            if let Some(target) = layer(font, glyph, id) {
                for index in indices {
                    if target.components.get(*index).is_some_and(|c| c.locked) {
                        return Err(locked_error());
                    }
                }
            }
        }
        Command::TransformSelection {
            glyph,
            layer: id,
            points,
            components,
            anchors,
            ..
        } => {
            if let Some(target) = layer(font, glyph, id) {
                if points.is_empty() && components.is_empty() && anchors.is_empty() {
                    ensure_unlocked(target)?;
                } else {
                    for index in components {
                        if target.components.get(*index).is_some_and(|c| c.locked) {
                            return Err(locked_error());
                        }
                    }
                }
            }
        }
        Command::ReplaceLayerOutline {
            glyph,
            layer: id,
            components,
            ..
        } => {
            if let Some(target) = layer(font, glyph, id) {
                preserve_locked(target, components)?;
            }
        }
        Command::RemoveLayer { glyph, layer: id }
        | Command::CenterMetrics {
            glyph, layer: id, ..
        }
        | Command::SetMetrics {
            glyph,
            layer: id,
            lsb: Some(_),
            ..
        } => {
            if let Some(target) = layer(font, glyph, id) {
                ensure_unlocked(target)?;
            }
        }
        Command::UpdateMetrics { glyph, master } => {
            if let Some(target) = font.glyph(glyph) {
                for layer in &target.layers {
                    if matches!(&layer.kind, LayerKind::Master(id) if master.as_ref().is_none_or(|m| m == id))
                    {
                        ensure_unlocked(layer)?;
                    }
                }
            }
        }
        Command::CopyLayer { glyph, to, .. } => {
            if matches!(to, LayerKind::Background(_)) {
                if let Some(target) = font
                    .glyph(glyph)
                    .and_then(|g| g.layers.iter().find(|l| l.kind == *to))
                {
                    ensure_unlocked(target)?;
                }
            }
        }
        Command::SwapWithBackground { glyph, master } => {
            if let Some(target) = font.glyph(glyph) {
                for layer in &target.layers {
                    if matches!(&layer.kind, LayerKind::Master(id) | LayerKind::Background(id) if id == master)
                    {
                        ensure_unlocked(layer)?;
                    }
                }
            }
        }
        Command::ReplaceGlyph { glyph } => {
            if let Some(current) = font.glyph(&glyph.name) {
                for layer in &current.layers {
                    let components = glyph
                        .layer(&layer.id)
                        .map_or(&[][..], |l| l.components.as_slice());
                    preserve_locked(layer, components)?;
                }
            }
        }
        Command::RemoveGlyphs { names } => {
            for name in names {
                if let Some(glyph) = font.glyph(name) {
                    for layer in &glyph.layers {
                        ensure_unlocked(layer)?;
                    }
                }
            }
        }
        Command::DesignSpace(DesignSpaceCommand::Set { masters, .. }) => {
            for layer in font.glyphs.values().flat_map(|g| &g.layers) {
                let master = match &layer.kind {
                    LayerKind::Master(id)
                    | LayerKind::Background(id)
                    | LayerKind::Smart { master: id, .. }
                    | LayerKind::Brace { master: id, .. }
                    | LayerKind::Bracket { master: id, .. }
                    | LayerKind::Backup { of: id, .. } => id,
                    LayerKind::Custom(_) => continue,
                };
                if !masters.iter().any(|m| m.id == *master) {
                    ensure_unlocked(layer)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn layer<'a>(font: &'a Font, glyph: &GlyphName, id: &LayerId) -> Option<&'a Layer> {
    font.glyph(glyph).and_then(|g| g.layer(id))
}

fn ensure_unlocked(layer: &Layer) -> Result<(), Error> {
    if layer.components.iter().any(|c| c.locked) {
        return Err(locked_error());
    }
    Ok(())
}

fn preserve_locked(layer: &Layer, replacement: &[Component]) -> Result<(), Error> {
    for (index, component) in layer.components.iter().enumerate() {
        if component.locked && replacement.get(index) != Some(component) {
            return Err(locked_error());
        }
    }
    Ok(())
}

fn locked_error() -> Error {
    Error::InvalidCommand("component is locked; unlock it before editing".into())
}
