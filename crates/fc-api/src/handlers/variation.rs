use crate::{
    Api, ApiError, Change, DocParams, VariationApplyParams, VariationPreview,
    VariationPreviewParams, VariationSourcePreview, VariationState,
};

pub(crate) fn state(api: &Api, params: DocParams) -> Result<VariationState, ApiError> {
    api.session
        .with_doc(params.doc, |doc| VariationState {
            generation: doc.generation,
            axes: doc.font.axes.clone(),
            masters: doc.font.masters.clone(),
            instances: doc.font.instances.clone(),
            first_glyph: doc
                .font
                .glyph_order()
                .find(|name| name.as_ref() != ".notdef")
                .map(ToString::to_string),
        })
        .map_err(ApiError::from)
}

pub(crate) fn apply(api: &Api, params: VariationApplyParams) -> Result<Change, ApiError> {
    api.session
        .with_doc_mut(params.doc, |doc| {
            if doc.generation != params.expected_generation {
                return Err(ApiError::invalid_params(
                    "design space changed; reload before applying your edits",
                ));
            }
            doc.apply(fc_doc::Command::DesignSpace(params.command), params.label)
                .map_err(ApiError::from)
        })
        .map_err(ApiError::from)?
}

pub(crate) fn preview(
    api: &Api,
    params: VariationPreviewParams,
) -> Result<VariationPreview, ApiError> {
    api.session
        .with_doc(params.doc, |doc| {
            let font = &doc.font;
            let glyph = font
                .glyph(&params.glyph)
                .ok_or_else(|| ApiError::not_found("preview glyph does not exist"))?;
            if params.location.len() > 16
                || params
                    .location
                    .keys()
                    .any(|tag| !font.axes.iter().any(|a| a.tag.to_string() == *tag))
            {
                return Err(ApiError::invalid_params("preview contains an unknown axis"));
            }
            let mut location = fc_model::DesignLocation::new();
            for axis in &font.axes {
                let user = params
                    .location
                    .get(&axis.tag.to_string())
                    .copied()
                    .unwrap_or(axis.default);
                if !user.is_finite() || user < axis.min || user > axis.max {
                    return Err(ApiError::invalid_params(
                        "preview value is outside the axis range",
                    ));
                }
                location.insert(axis.tag, axis.user_to_design(user));
            }
            let mut result = VariationPreview {
                generation: doc.generation,
                path: String::new(),
                width: 0.0,
                vertical_width: None,
                vertical_origin: None,
                units_per_em: font.info.units_per_em,
                sources: Vec::new(),
                issues: Vec::new(),
            };
            let sources = fc_geom::variation::glyph_sources(font, glyph)
                .map_err(|e| ApiError::invalid_params(e.to_string()))?;
            if sources.len() > 1024 {
                return Err(ApiError::invalid_params("preview exceeds 1024 sources"));
            }
            let reference = font
                .design_default_master()
                .ok()
                .and_then(|master| glyph.master_layer(&master.id));
            let mut preview_elements = 0usize;
            for (_, layer) in sources {
                let name = match &layer.kind {
                    fc_model::LayerKind::Master(id) => font
                        .master(id)
                        .map_or_else(|| id.to_string(), |m| m.name.clone()),
                    _ => {
                        if layer.name.is_empty() {
                            layer.id.to_string()
                        } else {
                            layer.name.clone()
                        }
                    }
                };
                if let Some(reference) = reference {
                    if let Err(issues) = fc_geom::compatible(reference, layer) {
                        for issue in issues
                            .into_iter()
                            .take(64usize.saturating_sub(result.issues.len()))
                        {
                            result.issues.push(format!("{name}: {}", issue.reason));
                        }
                    }
                }
                if result.sources.len() < 16 {
                    match fc_geom::flatten_layer_by_id(font, &params.glyph, &layer.id) {
                        Ok(path) => {
                            preview_elements =
                                preview_elements.saturating_add(path.elements().len());
                            if preview_elements <= 100_000 {
                                result.sources.push(VariationSourcePreview {
                                    name,
                                    path: path.to_svg(),
                                });
                            } else if result.issues.len() < 64 {
                                result
                                    .issues
                                    .push("source overlays exceed the preview work limit".into());
                            }
                        }
                        Err(error) if result.issues.len() < 64 => {
                            result.issues.push(format!("{name}: {error}"))
                        }
                        Err(_) => {}
                    }
                }
            }
            match fc_geom::variation::interpolate_glyph(font, glyph, &location) {
                Ok(layer) => {
                    result.width = layer.width;
                    result.vertical_width = layer.vertical_width;
                    result.vertical_origin = layer.vertical_origin;
                    match fc_geom::variation::interpolated_path(font, &params.glyph, &location) {
                        Ok(path) => result.path = path.to_svg(),
                        Err(error) => result.issues.push(error.to_string()),
                    }
                }
                Err(error) => result.issues.push(error.to_string()),
            }
            Ok(result)
        })
        .map_err(ApiError::from)?
}
