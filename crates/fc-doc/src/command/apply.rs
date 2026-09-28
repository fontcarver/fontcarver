use std::collections::HashSet;

use fc_model::{
    Anchor, Component, Contour, Font, Glyph, GlyphName, Layer, LayerId, LayerKind, MasterId,
};

use crate::{document::rename_references, undo, Error, Snapshot};

use super::{features, glyphs, kerning, layers, metrics, outline, structure, Command};

#[derive(Default)]
pub(crate) struct Impact {
    pub(crate) math: bool,
    pub(crate) color: bool,
    pub(crate) svg: bool,
    pub(crate) cid: bool,
    pub(crate) design_space: bool,
    pub(crate) bitmap_strikes: bool,
    pub(crate) bitmap_glyphs: Vec<(u16, u8, GlyphName)>,
    pub(crate) feature_source: bool,
    pub(crate) feature_anchors: Vec<(GlyphName, LayerId)>,
    pub(crate) layout: bool,
    pub(crate) glyphs: Vec<GlyphName>,
    pub(crate) glyph_set: HashSet<GlyphName>,
    pub(crate) font_info: bool,
    pub(crate) masters: Vec<MasterId>,
    pub(crate) glyph_order: bool,
    pub(crate) metrics: bool,
    pub(crate) layers: bool,
    pub(crate) guides: bool,
    pub(crate) font_guides: bool,
    pub(crate) renames: Vec<(GlyphName, GlyphName)>,
    pub(crate) affects_dependents: bool,
    pub(crate) component_graph_changed: bool,
}

impl Impact {
    pub(crate) fn glyph(&mut self, name: &GlyphName) {
        if self.glyph_set.insert(name.clone()) {
            self.glyphs.push(name.clone());
        }
    }

    fn master(&mut self, id: &MasterId) {
        if !self.masters.contains(id) {
            self.masters.push(id.clone());
        }
    }
}

pub(crate) fn impact(font: &Font, command: &Command) -> Impact {
    let mut impact = Impact::default();
    collect_impact(font, command, &mut impact);
    impact
}

fn collect_impact(font: &Font, command: &Command, impact: &mut Impact) {
    match command {
        Command::Hints(super::HintCommand::Ps { glyph, .. })
        | Command::Hints(super::HintCommand::Tt { glyph, .. }) => impact.glyph(glyph),
        Command::Hints(super::HintCommand::Private { master, .. })
        | Command::Hints(super::HintCommand::TtGlobals { master, .. }) => {
            impact.master(master);
            impact.font_info = true;
        }
        Command::Hints(super::HintCommand::AutohintPs { master, glyphs }) => {
            impact.master(master);
            impact.font_info = true;
            for name in glyphs {
                impact.glyph(name);
            }
        }
        Command::SetCidInfo { .. } => {
            impact.cid = true;
            impact.font_info = true;
        }
        Command::DesignSpace(super::DesignSpaceCommand::Layer { glyph, .. })
        | Command::DesignSpace(super::DesignSpaceCommand::SmartAxes { glyph, .. }) => {
            impact.glyph(glyph);
            impact.layers = true;
            impact.metrics = true;
            impact.affects_dependents = true;
            impact.component_graph_changed = true;
        }
        Command::DesignSpace(super::DesignSpaceCommand::Repair { glyphs }) => {
            for name in glyphs {
                impact.glyph(name);
            }
            impact.affects_dependents = true;
            impact.component_graph_changed = true;
        }
        Command::DesignSpace(super::DesignSpaceCommand::Set {
            masters,
            axis_renames,
            ..
        }) => {
            impact.design_space = true;
            impact.layout = true;
            impact.metrics = true;
            impact.layers = true;
            impact.font_info = true;
            let changed = !axis_renames.is_empty()
                || masters.len() != font.masters.len()
                || masters.iter().any(|m| font.master(&m.id).is_none());
            if changed {
                for name in font.glyphs.keys() {
                    impact.glyph(name);
                }
                impact.affects_dependents = true;
                impact.component_graph_changed = true;
            }
        }
        Command::Bitmap(super::BitmapCommand::SetGlyph {
            ppem, depth, glyph, ..
        }) => {
            let key = (*ppem, *depth, glyph.clone());
            if !impact.bitmap_glyphs.contains(&key) {
                impact.bitmap_glyphs.push(key);
            }
        }
        Command::Bitmap(_) => impact.bitmap_strikes = true,
        Command::Math(_) => impact.math = true,
        Command::Svg(super::SvgCommand::Replace { svg }) => {
            impact.svg = true;
            for table in font.svg.iter().chain(svg.iter()) {
                for document in &table.documents {
                    for name in document.glyphs.keys() {
                        impact.glyph(name);
                    }
                }
            }
        }
        Command::Color(super::ColorCommand::Replace { color }) => {
            impact.color = true;
            impact.component_graph_changed = true;
            for table in font.color.iter().chain(color.iter()) {
                if let Some(graph) = &table.graph {
                    for name in graph.glyphs.keys() {
                        impact.glyph(name);
                    }
                }
                for name in table.glyphs.keys() {
                    impact.glyph(name);
                }
            }
        }
        Command::SetFeatureDraft { .. } | Command::DiscardFeatureDraft { .. } => {
            impact.feature_source = true
        }
        Command::SetHorizontalKerning { .. }
        | Command::SetVerticalKerning { .. }
        | Command::CreateKerningGroup { .. }
        | Command::SetKerningGroup { .. }
        | Command::AssignKerningGroup { .. }
        | Command::RenameKerningGroup { .. }
        | Command::RemoveKerningGroup { .. } => impact.layout = true,
        Command::AddGlyph { glyph } => {
            impact.glyph(&glyph.name);
            impact.glyph_order = true;
            impact.layers = true;
            impact.metrics = true;
            impact.affects_dependents = true;
            impact.component_graph_changed = true;
        }
        Command::RemoveGlyphs { names } => {
            impact.math = font.math.is_some();
            impact.color = font.color.is_some();
            impact.svg = font.svg.is_some();
            impact.cid = font.cid.is_some();
            impact.bitmap_strikes = !font.bitmap_strikes.is_empty();
            impact.layout = true;
            for name in names {
                impact.glyph(name);
            }
            impact.glyph_order = true;
            impact.layers = true;
            impact.metrics = true;
            impact.affects_dependents = true;
            impact.component_graph_changed = true;
        }
        Command::RenameGlyph { from, to } => {
            impact.math = font.math.is_some();
            impact.color = font.color.is_some();
            impact.svg = font.svg.is_some();
            impact.cid = font.cid.is_some();
            impact.bitmap_strikes = !font.bitmap_strikes.is_empty();
            impact.layout = true;
            impact.glyph(from);
            impact.glyph(to);
            if from != to {
                impact.renames.push((from.clone(), to.clone()));
            }
            impact.glyph_order = true;
            impact.affects_dependents = true;
            impact.component_graph_changed |= from != to;
        }
        Command::SetUnicodes { glyph, .. } | Command::SetGlyphMeta { glyph, .. } => {
            impact.glyph(glyph)
        }
        Command::SetWidth { glyph, .. }
        | Command::SetVerticalMetrics { glyph, .. }
        | Command::SetMetricsKeys { glyph, .. } => {
            impact.glyph(glyph);
            impact.metrics = true;
        }
        Command::SetMetrics { glyph, .. } | Command::CenterMetrics { glyph, .. } => {
            impact.glyph(glyph);
            impact.metrics = true;
            impact.affects_dependents = true;
        }
        Command::ReplaceLayerOutline {
            glyph,
            layer,
            components,
            ..
        } => {
            impact.glyph(glyph);
            impact.affects_dependents = true;
            impact.component_graph_changed |= font
                .glyph(glyph)
                .and_then(|glyph| glyph.layers.iter().find(|candidate| candidate.id == *layer))
                .is_none_or(|current| current.components != *components);
        }
        Command::MovePoints { glyph, layer, .. }
        | Command::SetPointPosition { glyph, layer, .. }
        | Command::SetSmooth { glyph, layer, .. }
        | Command::InsertPointOnSegment { glyph, layer, .. }
        | Command::DeletePoints { glyph, layer, .. }
        | Command::ConvertSegment { glyph, layer, .. }
        | Command::AddContour { glyph, layer, .. }
        | Command::DeleteContours { glyph, layer, .. }
        | Command::ReverseContours { glyph, layer, .. }
        | Command::SetStartPoint { glyph, layer, .. }
        | Command::JoinContours { glyph, layer, .. }
        | Command::SplitContour { glyph, layer, .. }
        | Command::PathOp { glyph, layer, .. } => {
            impact.glyph(glyph);
            impact.affects_dependents = true;
            impact.metrics |= is_master_layer(font, glyph, layer);
        }
        Command::TransformSelection {
            glyph,
            layer,
            components,
            ..
        } => {
            impact.glyph(glyph);
            impact.affects_dependents = true;
            impact.component_graph_changed |= !components.is_empty();
            impact.metrics |= is_master_layer(font, glyph, layer);
        }
        Command::AddComponent { glyph, .. }
        | Command::SetComponent { glyph, .. }
        | Command::RemoveComponents { glyph, .. }
        | Command::DecomposeComponents { glyph, .. } => {
            impact.glyph(glyph);
            impact.affects_dependents = true;
            impact.component_graph_changed = true;
        }
        Command::AddAnchor { glyph, .. }
        | Command::SetAnchor { glyph, .. }
        | Command::RemoveAnchors { glyph, .. } => impact.glyph(glyph),
        Command::AddGuide { glyph, master, .. }
        | Command::SetGuide { glyph, master, .. }
        | Command::RemoveGuide { glyph, master, .. } => {
            impact.guides = true;
            if let Some(glyph) = glyph {
                impact.glyph(glyph);
            } else if let Some(master) = master {
                impact.master(master);
            } else {
                impact.font_guides = true;
            }
        }
        Command::SetLayerEffects { glyph, layer, .. } => {
            impact.glyph(glyph);
            impact.layers = true;
            impact.metrics |= is_master_layer(font, glyph, layer);
            impact.affects_dependents = true;
        }
        Command::AddLayer { glyph, .. }
        | Command::RemoveLayer { glyph, .. }
        | Command::CopyLayer { glyph, .. }
        | Command::SwapWithBackground { glyph, .. } => {
            impact.glyph(glyph);
            impact.layers = true;
            impact.affects_dependents = true;
            impact.component_graph_changed = true;
        }
        Command::DuplicateGlyph { name, .. } => {
            impact.math = font.math.is_some();
            impact.color = font.color.is_some();
            impact.svg = font.svg.is_some();
            impact.bitmap_strikes = !font.bitmap_strikes.is_empty();
            impact.glyph(name);
            impact.glyph_order = true;
            impact.layers = true;
            impact.metrics = true;
            impact.affects_dependents = true;
            impact.component_graph_changed = true;
        }
        Command::SetGlyphOrder { .. } => impact.glyph_order = true,
        Command::SetFontInfo { .. } => impact.font_info = true,
        Command::SetMasterMetrics { master, .. } => {
            impact.master(master);
            impact.metrics = true;
        }
        Command::SetMaster { master } => {
            impact.layout |= font.master(&master.id).is_some_and(|current| {
                !crate::kerning::same_records(&current.kerning_h, &master.kerning_h)
            });
            impact.master(&master.id);
            impact.metrics = true;
            impact.guides = true;
        }
        Command::SetFontGuides { .. } => {
            impact.guides = true;
            impact.font_guides = true;
        }
        Command::Batch(commands) => {
            for command in commands {
                collect_impact(font, command, impact);
            }
        }
    }
}

/// Reports whether a layer is the main layer of a design-space master.
///
/// Outline edits on a master layer change the derived sidebearings, so they
/// invalidate metrics as well.
fn is_master_layer(font: &Font, glyph: &GlyphName, layer: &LayerId) -> bool {
    font.glyph(glyph)
        .and_then(|glyph| glyph.layer(layer))
        .is_some_and(|layer| matches!(layer.kind, LayerKind::Master(_)))
}

pub(crate) fn apply_to_font(
    font: &mut Font,
    command: &Command,
    rollback: &[Snapshot],
) -> Result<(), Error> {
    match command {
        Command::Batch(commands) => {
            let mut applied_renames = Vec::new();
            if let Err(error) = apply_batch(font, commands, &mut applied_renames) {
                for (from, to) in applied_renames.iter().rev() {
                    rename_references(font, to, from);
                }
                undo::restore(font, rollback);
                return Err(error);
            }
            Ok(())
        }
        command => apply_inner(font, command),
    }
}

fn apply_batch(
    font: &mut Font,
    commands: &[Command],
    applied_renames: &mut Vec<(GlyphName, GlyphName)>,
) -> Result<(), Error> {
    for command in commands {
        match command {
            Command::Batch(nested) => apply_batch(font, nested, applied_renames)?,
            Command::RenameGlyph { from, to } => {
                apply_inner(font, command)?;
                if from != to {
                    applied_renames.push((from.clone(), to.clone()));
                }
            }
            command => apply_inner(font, command)?,
        }
    }
    Ok(())
}

fn apply_inner(font: &mut Font, command: &Command) -> Result<(), Error> {
    match command {
        Command::Hints(command) => super::hints::apply(font, command),
        Command::SetCidInfo { cid } => super::cid::set(font, cid),
        Command::DesignSpace(command) => super::designspace::apply(font, command),
        Command::Bitmap(command) => super::bitmap::apply(font, command),
        Command::Math(command) => super::math::apply(font, command),
        Command::Color(command) => super::color::apply(font, command),
        Command::Svg(command) => super::svg::apply(font, command),
        Command::SetFeatureDraft {
            expected_revision,
            source,
            settings,
        } => features::set_draft(font, *expected_revision, source, settings),
        Command::DiscardFeatureDraft { expected_revision } => {
            features::discard_draft(font, *expected_revision)
        }
        Command::SetHorizontalKerning { .. }
        | Command::SetVerticalKerning { .. }
        | Command::CreateKerningGroup { .. }
        | Command::SetKerningGroup { .. }
        | Command::AssignKerningGroup { .. }
        | Command::RenameKerningGroup { .. }
        | Command::RemoveKerningGroup { .. } => kerning::apply(font, command),
        Command::AddGlyph { glyph } => glyphs::add(font, glyph),
        Command::RemoveGlyphs { names } => glyphs::remove(font, names),
        Command::RenameGlyph { from, to } => glyphs::rename(font, from, to),
        Command::SetUnicodes { glyph, unicodes } => {
            glyph_mut(font, glyph)?.unicodes.clone_from(unicodes);
            Ok(())
        }
        Command::SetGlyphMeta {
            glyph,
            export,
            color_label,
            tags,
            note,
        } => glyphs::set_meta(font, glyph, export, color_label, tags, note),
        Command::SetWidth {
            glyph,
            layer,
            width,
        } => metrics::set(font, glyph, layer, Some(*width), None, None),
        Command::ReplaceLayerOutline {
            glyph,
            layer,
            contours,
            components,
            anchors,
        } => replace_layer_outline(font, glyph, layer, contours, components, anchors),
        Command::SetFontInfo { info } => {
            font.info = (**info).clone();
            Ok(())
        }
        Command::SetMasterMetrics { master, metrics } => {
            metrics.validate()?;
            let target = font
                .masters
                .iter_mut()
                .find(|candidate| candidate.id == *master)
                .ok_or_else(|| Error::MissingMaster(master.clone()))?;
            target.metrics = metrics.clone();
            Ok(())
        }
        Command::MovePoints { .. }
        | Command::SetPointPosition { .. }
        | Command::SetSmooth { .. }
        | Command::InsertPointOnSegment { .. }
        | Command::DeletePoints { .. }
        | Command::ConvertSegment { .. }
        | Command::AddContour { .. }
        | Command::DeleteContours { .. }
        | Command::ReverseContours { .. }
        | Command::SetStartPoint { .. }
        | Command::JoinContours { .. }
        | Command::SplitContour { .. }
        | Command::TransformSelection { .. }
        | Command::PathOp { .. } => outline::apply(font, command),
        Command::AddComponent { .. }
        | Command::SetComponent { .. }
        | Command::RemoveComponents { .. }
        | Command::DecomposeComponents { .. }
        | Command::AddAnchor { .. }
        | Command::SetAnchor { .. }
        | Command::RemoveAnchors { .. }
        | Command::AddGuide { .. }
        | Command::SetGuide { .. }
        | Command::RemoveGuide { .. } => structure::apply(font, command),
        Command::CenterMetrics {
            glyph,
            layer,
            width,
        } => metrics::center(font, glyph, layer, *width),
        Command::SetMetrics {
            glyph,
            layer,
            width,
            lsb,
            rsb,
        } => metrics::set(font, glyph, layer, *width, *lsb, *rsb),
        Command::SetVerticalMetrics {
            glyph,
            layer,
            advance,
            origin,
        } => metrics::set_vertical(font, glyph, layer, *advance, *origin),
        Command::SetMetricsKeys { glyph, keys } => {
            glyph_mut(font, glyph)?.metrics_keys = keys.clone();
            Ok(())
        }
        Command::SetLayerEffects { .. }
        | Command::AddLayer { .. }
        | Command::RemoveLayer { .. }
        | Command::CopyLayer { .. }
        | Command::SwapWithBackground { .. }
        | Command::SetMaster { .. }
        | Command::SetFontGuides { .. } => layers::apply(font, command),
        Command::DuplicateGlyph { source, name } => glyphs::duplicate(font, source, name),
        Command::SetGlyphOrder { order } => glyphs::set_order(font, order),
        Command::Batch(commands) => {
            for command in commands {
                apply_inner(font, command)?;
            }
            Ok(())
        }
    }
}

pub(super) fn glyph_mut<'a>(font: &'a mut Font, name: &GlyphName) -> Result<&'a mut Glyph, Error> {
    font.glyph_mut(name)
        .ok_or_else(|| Error::MissingGlyph(name.clone()))
}

pub(super) fn layer_mut<'a>(
    font: &'a mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
) -> Result<&'a mut Layer, Error> {
    glyph_mut(font, glyph)?
        .layers
        .iter_mut()
        .find(|candidate| candidate.id == *layer)
        .ok_or_else(|| Error::MissingLayer {
            glyph: glyph.clone(),
            layer: layer.clone(),
        })
}

pub(super) fn replace_layer_outline(
    font: &mut Font,
    glyph_name: &GlyphName,
    layer_id: &LayerId,
    contours: &[Contour],
    components: &[Component],
    anchors: &[Anchor],
) -> Result<(), Error> {
    for contour in contours {
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
    let mut replacement = original.clone();
    let layer = replacement
        .layers
        .iter_mut()
        .find(|candidate| candidate.id == *layer_id)
        .ok_or_else(|| Error::MissingLayer {
            glyph: glyph_name.clone(),
            layer: layer_id.clone(),
        })?;
    layer.contours = contours.to_vec();
    layer.components = components.to_vec();
    layer.anchors = anchors.to_vec();
    font.remove_glyph(glyph_name);
    if let Err(error) = font.insert_glyph(replacement) {
        if let Err(restore_error) = font.insert_glyph(original) {
            return Err(Error::Internal(format!(
                "failed to restore glyph after `{error}`: {restore_error}"
            )));
        }
        move_glyph_to(font, glyph_name, position);
        return Err(error.into());
    }
    move_glyph_to(font, glyph_name, position);
    Ok(())
}

pub(super) fn validate_glyph_outlines(glyph: &Glyph) -> Result<(), Error> {
    for contour in glyph.layers.iter().flat_map(|layer| &layer.contours).chain(
        glyph
            .layers
            .iter()
            .flat_map(|layer| layer.color_layers.iter().flatten())
            .flat_map(|layer| &layer.contours),
    ) {
        contour.try_to_bezpath()?;
    }
    Ok(())
}

fn move_glyph_to(font: &mut Font, glyph: &GlyphName, position: usize) {
    if let Some(current) = font.glyphs.get_index_of(glyph) {
        font.glyphs.move_index(current, position);
    }
}

pub(super) fn checked_indices(
    indices: &[usize],
    len: usize,
    what: &str,
) -> Result<Vec<usize>, Error> {
    let mut result = indices.to_vec();
    result.sort_unstable();
    result.dedup();
    if let Some(index) = result.iter().find(|&&index| index >= len) {
        return Err(Error::InvalidCommand(format!(
            "{what} index {index} is out of bounds for length {len}"
        )));
    }
    Ok(result)
}

pub(super) fn remove_indices<T>(values: &mut Vec<T>, indices: &[usize]) {
    for &index in indices.iter().rev() {
        values.remove(index);
    }
}

pub(super) fn ensure_finite(values: &[f64], what: &str) -> Result<(), Error> {
    if values.iter().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(Error::InvalidCommand(format!("{what} must be finite")))
    }
}

pub(super) fn unique_names(names: &[GlyphName]) -> bool {
    let mut seen = HashSet::new();
    names.iter().all(|name| seen.insert(name))
}
