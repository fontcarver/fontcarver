//! Snapshot preparation and revision-checked atomic optical edits.
use crate::{Api, ApiError, OpticalBatchParams, OpticalBatchResult};
use fc_doc::{optical::OpticalOptions, Command};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn run(api: &Api, params: OpticalBatchParams) -> Result<OpticalBatchResult, ApiError> {
    if params.glyphs.is_empty() || params.glyphs.len() > 128 || params.label.len() > 512 {
        return Err(ApiError::invalid_params(
            "optical selection must contain 1–128 glyphs and label at most 512 bytes",
        ));
    }
    let (font, generation, names) = api
        .session
        .with_doc(params.doc, |doc| {
            if !params.dry_run && params.expected_generation != Some(doc.generation) {
                return Err(ApiError::invalid_params(
                    "optical preview is stale; preview the selection again",
                ));
            }
            let names = params
                .glyphs
                .iter()
                .map(|id| {
                    doc.glyph_name(*id).cloned().ok_or_else(|| {
                        ApiError::not_found("selected optical glyph no longer exists")
                    })
                })
                .collect::<Result<BTreeSet<_>, _>>()?;
            Ok((
                doc.font.clone(),
                doc.generation,
                names.into_iter().collect(),
            ))
        })
        .map_err(ApiError::from)??;
    let mut proposal = fc_doc::optical::prepare(
        font,
        &OpticalOptions {
            glyphs: names,
            master: params.master,
            operation: params.operation,
            density: params.density,
            replace_existing: params.replace_existing,
        },
    )
    .map_err(ApiError::from)?;
    // Validate source ownership once per master when applying a large generated
    // table, instead of reparsing the feature program once for every pair.
    if params.operation == fc_doc::optical::OpticalOperation::Kerning {
        let mut pairs: BTreeMap<fc_model::MasterId, Vec<fc_model::KernPair>> = BTreeMap::new();
        for command in proposal.commands.drain(..) {
            if let Command::SetHorizontalKerning {
                master,
                left,
                right,
                value: Some(value),
            } = command
            {
                pairs
                    .entry(master)
                    .or_default()
                    .push(fc_model::KernPair { left, right, value });
            }
        }
        proposal.commands = pairs
            .into_iter()
            .map(|(master, records)| Command::KerningMaintenance {
                master,
                vertical: false,
                expected_generation: generation,
                operation: fc_doc::kerning::KerningMaintenanceOperation::Paste {
                    records,
                    replace: false,
                },
            })
            .collect();
    }
    let change = if params.dry_run || proposal.commands.is_empty() {
        None
    } else {
        Some(
            api.session
                .with_doc_mut(params.doc, |doc| {
                    if doc.generation != generation {
                        return Err(ApiError::invalid_params(
                            "optical preview is stale; preview the selection again",
                        ));
                    }
                    doc.apply(Command::Batch(proposal.commands), params.label)
                        .map_err(ApiError::from)
                })
                .map_err(ApiError::from)??,
        )
    };
    Ok(OpticalBatchResult {
        generation,
        glyph_count: proposal.glyph_count,
        changed_count: proposal.changed_count,
        skipped: proposal.skipped,
        examples: proposal.examples,
        change,
    })
}
