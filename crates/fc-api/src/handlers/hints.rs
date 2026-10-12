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
                is_cid: doc.font.cid.is_some(),
                cff_private_dict_index: None,
                path: String::new(),
                can_autohint_ps: false,
                units_per_em: doc.font.info.units_per_em,
                contour_count: 0,
                flex_paths: Vec::new(),
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
                result.cff_private_dict_index = Some(layer.cff_private_dict_index.unwrap_or(0));
                // Automatic analysis now covers additional CID dictionaries too;
                // it still needs the selected master's own layer.
                result.can_autohint_ps =
                    matches!(&layer.kind, fc_model::LayerKind::Master(id) if *id == params.master);
                result.layer = Some(layer.id.to_string());
                result.contour_count = layer.contours.len() as u32;
                result.flex_paths = layer
                    .hints
                    .flex_hints
                    .iter()
                    .map(|flex| {
                        layer
                            .hints
                            .flex_geometry(&layer.contours, flex)
                            .map(|geometry| {
                                let mut path = kurbo::BezPath::new();
                                path.move_to(geometry.start);
                                path.curve_to(
                                    geometry.points[0],
                                    geometry.points[1],
                                    geometry.points[2],
                                );
                                path.curve_to(
                                    geometry.points[3],
                                    geometry.points[4],
                                    geometry.points[5],
                                );
                                path.to_svg()
                            })
                            .unwrap_or_default()
                    })
                    .collect();
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
            let analysis = fc_hint::ps::analyze(&doc.font, &params.master, &names)
                .map_err(|e| ApiError::invalid_params(e.to_string()))?;
            Ok(HintAnalysis {
                generation: doc.generation,
                private: analysis.private,
                cid_private: analysis.cid_private,
                hints: analysis
                    .hints
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

/// Reports whether the optional external TrueType hinter is installed.
///
/// This touches no document: it only inspects `PATH` and asks the executable for
/// its version, so the export dialog can disclose the tool's absence before an
/// export rather than only afterwards.
pub(crate) fn tt_autohinter(
    _api: &Api,
    params: crate::TtAutohinterParams,
) -> Result<crate::TtAutohinterStatus, ApiError> {
    let problem = params
        .options
        .resolve()
        .err()
        .map(|error| error.message)
        .unwrap_or_default();
    Ok(match fc_hint::external::availability() {
        Some(found) => crate::TtAutohinterStatus {
            installed: true,
            path: found.path.to_string_lossy().into_owned(),
            version: found.version,
            problem,
        },
        None => crate::TtAutohinterStatus {
            problem,
            ..crate::TtAutohinterStatus::default()
        },
    })
}
