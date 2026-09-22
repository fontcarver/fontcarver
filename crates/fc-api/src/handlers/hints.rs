//! Hint reads and analysis never mutate the editable document.

use crate::{
    Api, ApiError, Change, HintAnalysis, HintAnalyzeParams, HintApplyParams, HintState,
    HintStateParams,
};

pub(crate) fn state(api: &Api, params: HintStateParams) -> Result<HintState, ApiError> {
    api.session
        .with_doc(params.doc, |doc| {
            let master = doc
                .font
                .master(&params.master)
                .ok_or_else(|| ApiError::not_found("hint master does not exist"))?;
            let mut result = HintState {
                generation: doc.generation,
                master: master.clone(),
                hints: None,
                layer: None,
                path: String::new(),
                can_autohint_ps: false,
                units_per_em: doc.font.info.units_per_em,
            };
            if let Some(name) = params.glyph {
                let glyph = doc
                    .font
                    .glyph(&name)
                    .ok_or_else(|| ApiError::not_found("hint glyph does not exist"))?;
                let layer = if let Some(id) = params.layer {
                    glyph.layers.iter().find(|l| l.id.to_string() == id)
                } else {
                    glyph.master_layer(&params.master)
                }
                .ok_or_else(|| ApiError::not_found("hint layer does not exist"))?;
                result.hints = Some(layer.hints.clone());
                result.can_autohint_ps =
                    matches!(&layer.kind, fc_model::LayerKind::Master(id) if *id == params.master);
                result.layer = Some(layer.id.to_string());
                result.path = fc_model::query::flatten_layer_by_id(&doc.font, &name, &layer.id)
                    .map_err(|e| ApiError::invalid_params(e.to_string()))?
                    .to_svg();
            }
            Ok(result)
        })
        .map_err(ApiError::from)?
}

pub(crate) fn analyze(api: &Api, params: HintAnalyzeParams) -> Result<HintAnalysis, ApiError> {
    api.session
        .with_doc(params.doc, |doc| {
            let names = params
                .glyphs
                .into_iter()
                .map(fc_model::GlyphName::from)
                .collect::<Vec<_>>();
            let (hints, private) = fc_hint::ps::analyze(&doc.font, &params.master, &names)
                .map_err(|e| ApiError::invalid_params(e.to_string()))?;
            Ok(HintAnalysis {
                generation: doc.generation,
                private,
                hints: hints
                    .into_iter()
                    .map(|(name, hints)| (name.to_string(), hints))
                    .collect(),
            })
        })
        .map_err(ApiError::from)?
}

pub(crate) fn apply(api: &Api, params: HintApplyParams) -> Result<Change, ApiError> {
    if params.commands.len() > 65_536 {
        return Err(ApiError::invalid_params(
            "hint operation count exceeds the work limit",
        ));
    }
    api.session
        .with_doc_mut(params.doc, |doc| {
            if doc.generation != params.expected_generation {
                return Err(ApiError::invalid_params(
                    "hint sources changed; reload the editor before applying",
                ));
            }
            let commands = params
                .commands
                .into_iter()
                .map(fc_doc::Command::Hints)
                .collect();
            doc.apply(fc_doc::Command::Batch(commands), params.label)
                .map_err(ApiError::from)
        })
        .map_err(ApiError::from)?
}
