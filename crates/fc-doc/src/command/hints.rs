//! Undoable PostScript and TrueType hint authoring.

use fc_model::{Font, GlyphName, LayerId, MasterId, PsHints, PsPrivate, Stem, Zone};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::Error;

/// Hint authoring operations; validation completes before mutation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub enum HintCommand {
    /// Replace a program while retaining its explicit compiled-point binding.
    Tt {
        /// Persistent glyph name.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Target master layer; alternate and background layers are not supported.
        #[ts(type = "string")]
        layer: LayerId,
        /// Validated assembled program, possibly empty to remove imported hints.
        program: Vec<u8>,
        /// Compiled point identity; export validates the current outline against it.
        fingerprint: String,
    },
    /// Replace global programs, CVT and maxima at the document's current UPEM.
    TtGlobals {
        /// Target master.
        #[ts(type = "string")]
        master: MasterId,
        /// Complete global program data and interpreter resource declarations.
        #[ts(type = "TtMasterData")]
        data: fc_model::TtMasterData,
    },
    /// Replace the complete hint set of one outline layer.
    Ps {
        /// Persistent glyph name.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Persistent source layer.
        #[ts(type = "string")]
        layer: LayerId,
        /// Horizontal/vertical stems, ghosts and replacement masks.
        #[ts(type = "PsHints")]
        hints: PsHints,
    },
    /// Set one master's zones, preferred stems and private dictionary.
    Private {
        /// Target master.
        #[ts(type = "string")]
        master: MasterId,
        /// Explicit private dictionary values.
        #[ts(type = "PsPrivate")]
        private: PsPrivate,
        /// Alignment-zone source values.
        #[ts(type = "Array<Zone>")]
        zones: Vec<Zone>,
        /// Preferred named stem widths.
        #[ts(type = "Array<Stem>")]
        stems: Vec<Stem>,
    },
    /// Replace a CID private dictionary or append the next dictionary.
    CffPrivate {
        /// Target master.
        #[ts(type = "string")]
        master: MasterId,
        /// Zero selects the default private dictionary; CID indices are 1–255.
        index: u8,
        /// Complete validated dictionary values.
        #[ts(type = "PsPrivate")]
        private: PsPrivate,
    },
    /// Assign an existing private dictionary to one layer in the target master.
    CffPrivateAssignment {
        /// Target master.
        #[ts(type = "string")]
        master: MasterId,
        /// Persistent glyph name.
        #[ts(type = "string")]
        glyph: GlyphName,
        /// Persistent source layer.
        #[ts(type = "string")]
        layer: LayerId,
        /// Existing dictionary index; zero selects the default.
        index: u8,
    },
    /// Infer glyph stems and missing global values as one reversible operation.
    AutohintPs {
        /// Target master.
        #[ts(type = "string")]
        master: MasterId,
        /// Captured glyph selection.
        #[ts(type = "Array<string>")]
        glyphs: Vec<GlyphName>,
    },
}

pub(super) fn apply(font: &mut Font, command: &HintCommand) -> Result<(), Error> {
    match command {
        HintCommand::Tt {
            glyph,
            layer,
            program,
            fingerprint,
        } => {
            fc_hint::tt::decode(program).map_err(|e| Error::InvalidCommand(e.to_string()))?;
            if !fingerprint
                .strip_prefix("tt-points-v1:")
                .is_some_and(|digest| {
                    digest.len() == 64
                        && digest
                            .bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                })
            {
                return Err(Error::InvalidCommand(
                    "invalid TrueType point binding".into(),
                ));
            }
            let target = super::apply::layer_mut(font, glyph, layer)?;
            if !matches!(target.kind, fc_model::LayerKind::Master(_)) {
                return Err(Error::InvalidCommand(
                    "TrueType instructions require a master layer".into(),
                ));
            }
            target.tt_instructions = Some(program.clone());
            target.tt_binding = Some(fc_model::TtInstructionBinding {
                fingerprint: fingerprint.clone(),
                authored: true,
            });
        }
        HintCommand::TtGlobals { master, data } => {
            fc_hint::tt::validate_globals(data)
                .map_err(|e| Error::InvalidCommand(e.to_string()))?;
            let upem = font.info.units_per_em;
            let target = font
                .masters
                .iter_mut()
                .find(|m| m.id == *master)
                .ok_or_else(|| Error::MissingMaster(master.clone()))?;
            target.tt = data.clone();
            target.tt.authored_units_per_em = Some(upem);
            target.tt.max_size_of_instructions = None;
        }
        HintCommand::Ps {
            glyph,
            layer,
            hints,
        } => {
            let target = super::apply::layer_mut(font, glyph, layer)?;
            hints.validate(&target.contours)?;
            target.hints = hints.clone();
        }
        HintCommand::Private {
            master,
            private,
            zones,
            stems,
        } => {
            private.validate()?;
            if zones.len() > 12
                || zones.iter().any(|z| {
                    !z.position.is_finite()
                        || !z.size.is_finite()
                        || z.position.abs() > 32767.0
                        || z.size.abs() > 32767.0
                })
                || stems.len() > 24
                || stems.iter().any(|s| {
                    !s.width.is_finite()
                        || s.width <= 0.0
                        || s.width > 32767.0
                        || s.name.len() > 256
                })
            {
                return Err(Error::InvalidCommand(
                    "invalid PostScript zones or stems".into(),
                ));
            }
            let target = font
                .masters
                .iter_mut()
                .find(|m| m.id == *master)
                .ok_or_else(|| Error::MissingMaster(master.clone()))?;
            target.private_dict = private.clone();
            target.alignment_zones = zones.clone();
            target.stems = stems.clone();
        }
        HintCommand::CffPrivate {
            master,
            index,
            private,
        } => {
            private.validate()?;
            if *index > 0 && font.cid.is_none() {
                return Err(Error::InvalidCommand(
                    "additional private dictionaries require a CID font".into(),
                ));
            }
            let target = font
                .masters
                .iter_mut()
                .find(|m| m.id == *master)
                .ok_or_else(|| Error::MissingMaster(master.clone()))?;
            if *index == 0 {
                target.private_dict = private.clone();
            } else {
                let slot = usize::from(*index) - 1;
                if slot > target.cff_private_dicts.len() {
                    return Err(Error::InvalidCommand(
                        "private dictionary index does not exist".into(),
                    ));
                }
                if slot == target.cff_private_dicts.len() {
                    target.cff_private_dicts.push(private.clone());
                } else {
                    target.cff_private_dicts[slot] = private.clone();
                }
            }
        }
        HintCommand::CffPrivateAssignment {
            master,
            glyph,
            layer,
            index,
        } => {
            let target_master = font
                .master(master)
                .ok_or_else(|| Error::MissingMaster(master.clone()))?;
            if usize::from(*index) > target_master.cff_private_dicts.len()
                || (*index > 0 && font.cid.is_none())
            {
                return Err(Error::InvalidCommand(
                    "private dictionary index does not exist in this CID master".into(),
                ));
            }
            let kind = super::apply::layer_mut(font, glyph, layer)?.kind.clone();
            if layer_master(&kind, font) != master {
                return Err(Error::InvalidCommand(
                    "private dictionary assignment targets a different master".into(),
                ));
            }
            super::apply::layer_mut(font, glyph, layer)?.cff_private_dict_index =
                (*index > 0).then_some(*index);
        }
        HintCommand::AutohintPs { master, glyphs } => {
            let analysis = fc_hint::ps::analyze(font, master, glyphs)
                .map_err(|e| Error::InvalidCommand(e.to_string()))?;
            // Every target was validated by analysis before this loop starts,
            // and the analysis carries each layer's flex hints, dot sections and
            // unresolved-flex marker through, so replacing the set is complete.
            for (name, hints) in analysis.hints {
                if let Some(layer) = font
                    .glyph_mut(&name)
                    .and_then(|g| g.master_layer_mut(master))
                {
                    layer.hints = hints;
                }
            }
            if let Some(target) = font.masters.iter_mut().find(|m| m.id == *master) {
                target.private_dict = analysis.private;
                for (index, private) in analysis.cid_private {
                    if let Some(slot) = target
                        .cff_private_dicts
                        .get_mut(usize::from(index).saturating_sub(1))
                    {
                        *slot = private;
                    }
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn invalidate_stale_ps_hints(font: &mut Font, before: &[crate::Snapshot]) {
    use std::collections::HashSet;
    if !before.iter().any(|snapshot| {
        matches!(snapshot, crate::Snapshot::Glyph(_, Some(glyph))
            if glyph.layers.iter().any(|layer| !layer.hints.is_empty()))
    }) {
        return;
    }
    // Component resolution uses only master and smart layers in the same master.
    // Background, backup and alternate edits must not invalidate other sources.
    let mut sources = HashSet::new();
    let mut edited = HashSet::new();
    for snapshot in before {
        let crate::Snapshot::Glyph(name, Some(previous)) = snapshot else {
            continue;
        };
        let current = font.glyph(name);
        for layer in &previous.layers {
            let now = current.and_then(|g| g.layer(&layer.id));
            if now.is_none_or(|now| {
                now.contours != layer.contours
                    || now.components != layer.components
                    || now.effects != layer.effects
                    || now.kind != layer.kind
            }) || current.is_some_and(|g| g.smart_axes != previous.smart_axes)
            {
                edited.insert((name.clone(), layer.id.clone()));
                for kind in std::iter::once(&layer.kind).chain(now.map(|l| &l.kind)) {
                    if let fc_model::LayerKind::Master(master)
                    | fc_model::LayerKind::Smart { master, .. } = kind
                    {
                        sources.insert((name.clone(), master.clone()));
                    }
                }
            }
        }
        if let Some(current) = current {
            for layer in &current.layers {
                if previous.layer(&layer.id).is_none() {
                    if let fc_model::LayerKind::Master(master)
                    | fc_model::LayerKind::Smart { master, .. } = &layer.kind
                    {
                        sources.insert((name.clone(), master.clone()));
                    }
                }
            }
        }
    }
    if edited.is_empty() && sources.is_empty() {
        return;
    }
    let dependents = fc_model::dependents(font);
    let mut pending: Vec<_> = sources.iter().cloned().collect();
    while let Some((name, master)) = pending.pop() {
        for dependent in dependents.get(&name).into_iter().flatten() {
            let Some(glyph) = font.glyph(dependent) else {
                continue;
            };
            for layer in &glyph.layers {
                if layer_master(&layer.kind, font) != &master
                    || !layer.components.iter().any(|c| c.base == name)
                {
                    continue;
                }
                edited.insert((dependent.clone(), layer.id.clone()));
                if matches!(
                    &layer.kind,
                    fc_model::LayerKind::Master(_) | fc_model::LayerKind::Smart { .. }
                ) && sources.insert((dependent.clone(), master.clone()))
                {
                    pending.push((dependent.clone(), master.clone()));
                }
            }
        }
    }
    for snapshot in before {
        let crate::Snapshot::Glyph(name, Some(previous)) = snapshot else {
            continue;
        };
        let Some(current) = font.glyph_mut(name) else {
            continue;
        };
        for layer in &mut current.layers {
            if edited.contains(&(name.clone(), layer.id.clone()))
                && previous
                    .layer(&layer.id)
                    .is_some_and(|old| old.hints == layer.hints)
            {
                layer.hints = PsHints::default();
            }
        }
    }
}

fn layer_master<'a>(kind: &'a fc_model::LayerKind, font: &'a Font) -> &'a MasterId {
    use fc_model::LayerKind;
    match kind {
        LayerKind::Master(master)
        | LayerKind::Background(master)
        | LayerKind::Brace { master, .. }
        | LayerKind::Bracket { master, .. }
        | LayerKind::Smart { master, .. } => master,
        LayerKind::Backup { of, .. } => of,
        LayerKind::Custom(_) => &font.default_master().id,
    }
}
