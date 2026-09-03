//! Mutation commands and their application to the persistent model.

use std::collections::{BTreeSet, HashSet};

use fw_model::{
    Anchor, Component, Contour, Font, FontInfo, Glyph, GlyphName, LayerId, MasterId, MasterMetrics,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{document::rename_references, undo, Error, Snapshot};

/// One atomic, undoable document mutation.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub enum Command {
    /// Insert a new glyph at the end of glyph order.
    AddGlyph {
        /// Glyph to insert.
        #[ts(type = "Glyph")]
        glyph: Glyph,
    },
    /// Remove one or more glyphs while preserving the order of the rest.
    RemoveGlyphs {
        /// Glyph names to remove.
        #[ts(type = "Array<string>")]
        names: Vec<GlyphName>,
    },
    /// Rename a glyph and all model references to it.
    RenameGlyph {
        /// Existing glyph name.
        #[ts(type = "string")]
        from: GlyphName,
        /// Replacement glyph name.
        #[ts(type = "string")]
        to: GlyphName,
    },
    /// Replace a glyph's Unicode assignments.
    SetUnicodes {
        /// Glyph to update.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// New Unicode scalar values.
        unicodes: Vec<u32>,
    },
    /// Update selected glyph metadata fields.
    SetGlyphMeta {
        /// Glyph to update.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Optional export flag update.
        export: Option<bool>,
        /// Optional color-label update; the inner option clears the label.
        color_label: Option<Option<u8>>,
        /// Optional complete replacement tag set.
        #[ts(type = "Array<string> | null")]
        tags: Option<BTreeSet<String>>,
        /// Optional note update.
        note: Option<String>,
    },
    /// Set a layer's horizontal advance width.
    SetWidth {
        /// Glyph containing the layer.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Layer to update.
        #[ts(type = "string")]
        layer: LayerId,
        /// New width in font units.
        width: f64,
    },
    /// Replace all editable outline content in a layer.
    ReplaceLayerOutline {
        /// Glyph containing the layer.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Layer to update.
        #[ts(type = "string")]
        layer: LayerId,
        /// New point-list contours.
        #[ts(type = "Array<Contour>")]
        contours: Vec<Contour>,
        /// New components.
        #[ts(type = "Array<Component>")]
        components: Vec<Component>,
        /// New anchors.
        #[ts(type = "Array<Anchor>")]
        anchors: Vec<Anchor>,
    },
    /// Replace all font-wide metadata.
    SetFontInfo {
        /// New font information.
        #[ts(type = "FontInfo")]
        info: Box<FontInfo>,
    },
    /// Replace the metrics of one master.
    SetMasterMetrics {
        /// Master to update.
        #[ts(type = "string")]
        master: MasterId,
        /// New metrics.
        #[ts(type = "MasterMetrics")]
        metrics: MasterMetrics,
    },
    /// Apply several commands as one atomic undo entry.
    Batch(Vec<Command>),
}

/// Precise invalidation information produced by a command.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct Change {
    /// Stable handles for changed or transitively invalidated glyphs.
    #[ts(type = "Array<number>")]
    pub glyphs: Vec<fw_model::GlyphId>,
    /// Whether font-wide metadata changed.
    pub font_info: bool,
    /// Whether master data changed.
    pub masters: bool,
    /// Whether glyph order or glyph names changed.
    pub glyph_order: bool,
}

impl Change {
    pub(crate) fn merge(&mut self, other: &Self) {
        for glyph in &other.glyphs {
            if !self.glyphs.contains(glyph) {
                self.glyphs.push(*glyph);
            }
        }
        self.glyphs.sort_by_key(|glyph| glyph.0);
        self.font_info |= other.font_info;
        self.masters |= other.masters;
        self.glyph_order |= other.glyph_order;
    }
}

#[derive(Default)]
pub(crate) struct Impact {
    pub(crate) glyphs: Vec<GlyphName>,
    pub(crate) font_info: bool,
    pub(crate) masters: Vec<MasterId>,
    pub(crate) glyph_order: bool,
    pub(crate) renames: Vec<(GlyphName, GlyphName)>,
    pub(crate) affects_dependents: bool,
    pub(crate) component_graph_changed: bool,
}

impl Impact {
    pub(crate) fn glyph(&mut self, name: &GlyphName) {
        if !self.glyphs.contains(name) {
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
        Command::AddGlyph { glyph } => {
            impact.glyph(&glyph.name);
            impact.glyph_order = true;
            impact.affects_dependents = true;
            impact.component_graph_changed = true;
        }
        Command::RemoveGlyphs { names } => {
            for name in names {
                impact.glyph(name);
            }
            impact.glyph_order = true;
            impact.affects_dependents = true;
            impact.component_graph_changed = true;
        }
        Command::RenameGlyph { from, to } => {
            impact.glyph(from);
            impact.glyph(to);
            if from != to {
                impact.renames.push((from.clone(), to.clone()));
            }
            impact.glyph_order = true;
            impact.affects_dependents = true;
            impact.component_graph_changed |= from != to;
        }
        Command::SetUnicodes { glyph, .. }
        | Command::SetGlyphMeta { glyph, .. }
        | Command::SetWidth { glyph, .. } => impact.glyph(glyph),
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
        Command::SetFontInfo { .. } => impact.font_info = true,
        Command::SetMasterMetrics { master, .. } => impact.master(master),
        Command::Batch(commands) => {
            for command in commands {
                collect_impact(font, command, impact);
            }
        }
    }
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
        Command::AddGlyph { glyph } => {
            validate_glyph_outlines(glyph)?;
            font.insert_glyph(glyph.clone()).map_err(Error::from)
        }
        Command::RemoveGlyphs { names } => remove_glyphs(font, names),
        Command::RenameGlyph { from, to } => font.rename_glyph(from, to).map_err(Error::from),
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
        } => {
            let glyph = glyph_mut(font, glyph)?;
            if let Some(export) = export {
                glyph.export = *export;
            }
            if let Some(color_label) = color_label {
                glyph.color_label = *color_label;
            }
            if let Some(tags) = tags {
                glyph.tags.clone_from(tags);
            }
            if let Some(note) = note {
                glyph.note.clone_from(note);
            }
            Ok(())
        }
        Command::SetWidth {
            glyph,
            layer,
            width,
        } => {
            let glyph_data = glyph_mut(font, glyph)?;
            let Some(layer_data) = glyph_data
                .layers
                .iter_mut()
                .find(|candidate| candidate.id == *layer)
            else {
                return Err(Error::MissingLayer {
                    glyph: glyph.clone(),
                    layer: layer.clone(),
                });
            };
            layer_data.width = *width;
            Ok(())
        }
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
            let Some(master_data) = font
                .masters
                .iter_mut()
                .find(|candidate| candidate.id == *master)
            else {
                return Err(Error::MissingMaster(master.clone()));
            };
            master_data.metrics = metrics.clone();
            Ok(())
        }
        Command::Batch(commands) => {
            for command in commands {
                apply_inner(font, command)?;
            }
            Ok(())
        }
    }
}

fn glyph_mut<'a>(font: &'a mut Font, name: &GlyphName) -> Result<&'a mut Glyph, Error> {
    font.glyph_mut(name)
        .ok_or_else(|| Error::MissingGlyph(name.clone()))
}

fn remove_glyphs(font: &mut Font, names: &[GlyphName]) -> Result<(), Error> {
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

fn replace_layer_outline(
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
    let Some(position) = font.glyphs.get_index_of(glyph_name) else {
        return Err(Error::MissingGlyph(glyph_name.clone()));
    };
    let Some(original) = font.glyph(glyph_name).cloned() else {
        return Err(Error::MissingGlyph(glyph_name.clone()));
    };
    let mut replacement = original.clone();
    let Some(layer) = replacement
        .layers
        .iter_mut()
        .find(|candidate| candidate.id == *layer_id)
    else {
        return Err(Error::MissingLayer {
            glyph: glyph_name.clone(),
            layer: layer_id.clone(),
        });
    };
    layer.contours = contours.to_vec();
    layer.components = components.to_vec();
    layer.anchors = anchors.to_vec();

    font.remove_glyph(glyph_name);
    if let Err(error) = font.insert_glyph(replacement) {
        // The original came from a valid document and therefore always restores.
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

fn validate_glyph_outlines(glyph: &Glyph) -> Result<(), Error> {
    for contour in glyph.layers.iter().flat_map(|layer| &layer.contours).chain(
        glyph
            .layers
            .iter()
            .flat_map(|layer| layer.color_layers.iter().flatten())
            .flat_map(|color_layer| &color_layer.contours),
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
