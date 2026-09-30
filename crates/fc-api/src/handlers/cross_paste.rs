//! Transactional cross-document transfer and reference repair.

use std::collections::BTreeSet;

use fc_doc::{Change, Command, Session};
use fc_model::{GlyphName, LayerKind, MasterId};

use crate::{Api, ApiError, PasteSpecialParams, PasteSpecialResult, RelinkComponentsParams};

mod plan;

pub(crate) fn paste(api: &Api, params: PasteSpecialParams) -> Result<PasteSpecialResult, ApiError> {
    if params.mappings.is_empty() || params.mappings.len() > 512 {
        return Err(ApiError::invalid_params(
            "paste requires 1 to 512 glyph mappings",
        ));
    }
    let (source, source_names, source_generation) =
        api.session.with_doc(params.source_doc, |doc| {
            check_generation(doc.generation, params.expected_source_generation, "source")?;
            let names = params
                .mappings
                .iter()
                .map(|mapping| {
                    doc.glyph_name(mapping.source)
                        .cloned()
                        .ok_or_else(|| ApiError::not_found("source glyph handle does not exist"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<_, ApiError>((doc.font.clone(), names, doc.generation))
        })??;
    let (plan, target, generation) = api.session.with_doc(params.doc, |doc| {
        check_generation(doc.generation, params.expected_generation, "destination")?;
        // A same-document operation must also guard the source snapshot taken
        // before acquiring the destination lock.
        if params.source_doc == params.doc && doc.generation != source_generation {
            return Err(ApiError::invalid_params("source snapshot is stale"));
        }
        let plan = plan::build(&source, &source_names, doc, &params)?;
        Ok::<_, ApiError>((plan, doc.font.clone(), doc.generation))
    })??;
    let batch = Command::Batch(plan.commands);
    // Preview uses the real command/undo validators in an isolated session.
    // Run outside session callbacks, which forbid reentrancy even across sessions.
    let scratch = Session::new();
    let id = scratch.open_font(target, None);
    scratch.with_doc_mut(id, |candidate| {
        candidate.apply(batch.clone(), "Validate paste")
    })??;
    let (change, generation) = if params.dry_run {
        (None, generation)
    } else {
        api.session.with_doc_mut(params.doc, |doc| {
            check_generation(doc.generation, Some(generation), "destination")?;
            let change = doc.apply(batch, "Paste special")?;
            Ok::<_, ApiError>((Some(change), doc.generation))
        })??
    };
    Ok(PasteSpecialResult {
        change,
        generation,
        source_generation,
        pasted: plan.pasted,
        added_bases: plan.added_bases,
        warnings: plan.warnings,
    })
}

pub(crate) fn relink(api: &Api, params: RelinkComponentsParams) -> Result<Change, ApiError> {
    if params.gids.is_empty() || params.gids.len() > 100_000 || params.from.is_empty() {
        return Err(ApiError::invalid_params(
            "relink requires a base name and 1 to 100000 glyphs",
        ));
    }
    api.session.with_doc_mut(params.doc, |doc| {
        check_generation(doc.generation, params.expected_generation, "destination")?;
        let target = doc
            .glyph_name(params.to)
            .cloned()
            .ok_or_else(|| ApiError::not_found("replacement base does not exist"))?;
        if let Some(master) = &params.master {
            if doc.font.master(master).is_none() {
                return Err(ApiError::not_found("relink master does not exist"));
            }
        }
        let mut seen = BTreeSet::new();
        let mut commands = Vec::new();
        for gid in params.gids {
            let name = doc
                .glyph_name(gid)
                .ok_or_else(|| ApiError::not_found("relink glyph handle does not exist"))?;
            if !seen.insert(name.clone()) {
                continue;
            }
            let mut glyph = doc
                .font
                .glyph(name)
                .cloned()
                .ok_or_else(|| ApiError::not_found("relink glyph does not exist"))?;
            let mut changed = false;
            for layer in &mut glyph.layers {
                if params
                    .master
                    .as_ref()
                    .is_some_and(|master| layer_master(&layer.kind) != Some(master))
                {
                    continue;
                }
                for component in layer.components.iter_mut().chain(
                    layer
                        .color_layers
                        .iter_mut()
                        .flatten()
                        .flat_map(|color| &mut color.components),
                ) {
                    if component.base.as_ref() == params.from && component.base != target {
                        component.base = target.clone();
                        changed = true;
                    }
                }
                for construction in &mut layer.constructions {
                    if construction.source.as_ref() == params.from && construction.source != target
                    {
                        construction.source = target.clone();
                        changed = true;
                    }
                }
            }
            if changed {
                if glyph.locked {
                    return Err(ApiError::invalid_params(format!(
                        "glyph `{name}` is locked"
                    )));
                }
                commands.push(Command::ReplaceGlyph { glyph });
            }
        }
        if commands.is_empty() {
            return Ok(Change::default());
        }
        doc.apply(Command::Batch(commands), "Relink components")
            .map_err(ApiError::from)
    })?
}

fn check_generation(actual: u64, expected: Option<u64>, label: &str) -> Result<(), ApiError> {
    if expected.is_some_and(|expected| expected != actual) {
        Err(ApiError::invalid_params(format!(
            "{label} preview is stale; preview again"
        )))
    } else {
        Ok(())
    }
}

fn layer_master(kind: &LayerKind) -> Option<&MasterId> {
    match kind {
        LayerKind::Master(master)
        | LayerKind::Background(master)
        | LayerKind::Smart { master, .. }
        | LayerKind::Brace { master, .. }
        | LayerKind::Bracket { master, .. } => Some(master),
        LayerKind::Backup { of, .. } => Some(of),
        LayerKind::Custom(_) => None,
    }
}

fn glyph_name(name: &GlyphName) -> String {
    name.to_string()
}
