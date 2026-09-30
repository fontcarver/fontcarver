//! Component policy: locked-reference preflight and anchor alignment.

use std::collections::{BTreeSet, HashSet};

use fc_model::{
    Anchor, AnchorKind, Component, Font, GlyphName, Layer, LayerId, LayerKind, MasterId,
};
use kurbo::{Affine, Point};

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

/// Places every automatically aligned component of one layer by its anchors.
///
/// A mark component is positioned by matching its own mark anchor — the one
/// whose stored name begins with `_` — onto the base attachment anchor of the
/// same name. The attachment anchor is looked for in this order, which is the
/// order a composite is read in:
///
/// 1. the carrier layer's own anchors, then
/// 2. the base glyphs of the components before this one, nearest first, each
///    anchor transformed by that component's own transform.
///
/// Which mark anchor is used is `Component::anchor` when it names one, and
/// otherwise the first mark anchor the base layer declares. Only the
/// translation changes: a component's scale, rotation and skew are the
/// designer's and are left exactly as they are, and a component whose pair
/// cannot be resolved is left alone rather than moved somewhere arbitrary.
///
/// The base layer is the one belonging to the carrier layer's master, so
/// alignment is per-master, and resolution never recurses into a component's
/// own components, so a cyclic reference cannot loop here.
///
/// A **locked** component is skipped: preserving it is exactly what its lock
/// promises, and one locked mark does not stop the rest of the layer from being
/// placed. Placement is progressive, so a composite attaching to an earlier
/// auto-aligned component reads where that component has just been put.
pub(super) fn align_auto_components(
    font: &mut Font,
    glyph: &GlyphName,
    id: &LayerId,
) -> Result<Vec<usize>, Error> {
    let target = layer(font, glyph, id).ok_or_else(|| {
        Error::InvalidCommand(format!("glyph {glyph} has no layer {id} to align"))
    })?;
    let master = match &target.kind {
        LayerKind::Master(master)
        | LayerKind::Background(master)
        | LayerKind::Smart { master, .. }
        | LayerKind::Brace { master, .. }
        | LayerKind::Bracket { master, .. }
        | LayerKind::Backup { of: master, .. } => Some(master.clone()),
        LayerKind::Custom(_) => None,
    };
    // The working copy is updated as each component is placed, so a composite
    // attaching to an earlier auto-aligned component reads where that component
    // has just been put rather than where it used to be.
    let mut working = target.components.clone();
    let own_anchors = target.anchors.clone();
    let mut moved = Vec::new();
    for index in 0..working.len() {
        let component = working[index].clone();
        // A locked component is preserved, which is exactly what its lock
        // promises; aligning the rest of the layer is not blocked by it.
        if !component.auto_align || component.locked {
            continue;
        }
        let Some(base) = base_layer(font, &component.base, master.as_ref()) else {
            continue;
        };
        let Some((name, mark)) = mark_anchor(base, component.anchor.as_deref()) else {
            continue;
        };
        let Some(attachment) = attachment_point(
            font,
            &own_anchors,
            &working[..index],
            master.as_ref(),
            &name,
        ) else {
            continue;
        };
        // Keep the linear part, solve for the translation that lands the mark
        // anchor on the attachment point.
        let linear = component.transform;
        let placed = Affine::new([
            linear.as_coeffs()[0],
            linear.as_coeffs()[1],
            linear.as_coeffs()[2],
            linear.as_coeffs()[3],
            0.0,
            0.0,
        ]) * mark;
        let offset = (attachment.x - placed.x, attachment.y - placed.y);
        if !offset.0.is_finite() || !offset.1.is_finite() {
            continue;
        }
        let mut coeffs = linear.as_coeffs();
        if coeffs[4] == offset.0 && coeffs[5] == offset.1 {
            continue;
        }
        coeffs[4] = offset.0;
        coeffs[5] = offset.1;
        working[index].transform = Affine::new(coeffs);
        moved.push(index);
    }
    if moved.is_empty() {
        return Ok(moved);
    }
    let target = font
        .glyph_mut(glyph)
        .and_then(|g| g.layers.iter_mut().find(|l| l.id == *id))
        .ok_or_else(|| Error::InvalidCommand(format!("glyph {glyph} has no layer {id}")))?;
    for &index in &moved {
        if let (Some(component), Some(placed)) =
            (target.components.get_mut(index), working.get(index))
        {
            component.transform = placed.transform;
        }
    }
    Ok(moved)
}

/// The base glyph's layer for one master, falling back to its first layer.
fn base_layer<'a>(
    font: &'a Font,
    base: &GlyphName,
    master: Option<&MasterId>,
) -> Option<&'a Layer> {
    let glyph = font.glyph(base)?;
    master
        .and_then(|master| glyph.master_layer(master))
        .or_else(|| glyph.layers.first())
}

/// The mark anchor a component attaches by, as `(name without the underscore, point)`.
fn mark_anchor(base: &Layer, wanted: Option<&str>) -> Option<(String, Point)> {
    let named = wanted.map(|name| name.trim_start_matches('_').to_owned());
    base.anchors.iter().find_map(|anchor| match anchor.kind() {
        AnchorKind::Mark(name) if named.as_ref().is_none_or(|want| *want == name) => {
            Some((name, Point::new(anchor.x, anchor.y)))
        }
        _ => None,
    })
}

/// Where the mark anchor has to land, in the carrier layer's own coordinates.
fn attachment_point(
    font: &Font,
    own: &[Anchor],
    before: &[Component],
    master: Option<&MasterId>,
    name: &str,
) -> Option<Point> {
    if let Some(anchor) = own
        .iter()
        .find(|anchor| anchor.kind() == AnchorKind::Base(name.to_owned()))
    {
        return Some(Point::new(anchor.x, anchor.y));
    }
    for component in before.iter().rev() {
        let Some(base) = base_layer(font, &component.base, master) else {
            continue;
        };
        if let Some(anchor) = base
            .anchors
            .iter()
            .find(|anchor| anchor.kind() == AnchorKind::Base(name.to_owned()))
        {
            return Some(component.transform * Point::new(anchor.x, anchor.y));
        }
    }
    None
}

/// How many times a moved anchor may cascade into further composites.
///
/// Each round places the composites that reference the previous round's
/// glyphs; four covers `a` → `aacute` → `aacutegrave` → one more and bounds a
/// pathological or cyclic component graph without a visited-set walk.
const MAX_ALIGN_ROUNDS: usize = 4;

/// Every layer whose automatic alignment can depend on `root`'s anchors.
///
/// Returned in dependency order — the layers that reference `root` first, then
/// the layers that reference *those* glyphs — so aligning them in order places
/// each composite after whatever it attaches to. The set is a superset: a layer
/// whose offsets turn out to be unchanged is simply a no-op when it is aligned,
/// and including it is what lets the undo snapshot cover it.
pub(super) fn auto_align_closure(font: &Font, root: &GlyphName) -> Vec<(GlyphName, LayerId)> {
    let mut ordered: Vec<(GlyphName, LayerId)> = Vec::new();
    let mut seen: HashSet<(GlyphName, LayerId)> = HashSet::new();
    let mut roots: BTreeSet<GlyphName> = BTreeSet::from([root.clone()]);
    for _ in 0..MAX_ALIGN_ROUNDS {
        let mut next: BTreeSet<GlyphName> = BTreeSet::new();
        for (name, glyph) in &font.glyphs {
            for layer in &glyph.layers {
                // A layer is a candidate when it has something to place *and*
                // an attachment anchor it could read has moved. The anchor
                // belongs either to this layer itself or to one of the glyphs
                // its components reference — auto-aligned or not, because the
                // base component of a composite is usually the plain one.
                if !layer.components.iter().any(|c| c.auto_align && !c.locked) {
                    continue;
                }
                let reads_a_root = roots.contains(name)
                    || layer.components.iter().any(|c| roots.contains(&c.base));
                if !reads_a_root {
                    continue;
                }
                let key = (name.clone(), layer.id.clone());
                if seen.insert(key.clone()) {
                    ordered.push(key);
                    next.insert(name.clone());
                }
            }
        }
        if next.is_empty() {
            break;
        }
        roots = next;
    }
    ordered
}
