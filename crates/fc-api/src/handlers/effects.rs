//! Compute outside the document lock and commit against the captured revision.
use crate::{
    Api, ApiError, EffectMaskParams, LayerEffectsParams, LayerEffectsResult, StyleBatchExample,
};
use fc_doc::Command;
use kurbo::Shape;

fn invalid(error: impl std::fmt::Display) -> ApiError {
    ApiError::invalid_params(error.to_string())
}

pub(crate) fn mask(api: &Api, params: EffectMaskParams) -> Result<String, ApiError> {
    let (font, layer) = api
        .session
        .with_doc(params.doc, |doc| {
            let glyph = doc
                .font
                .glyph(&params.glyph)
                .ok_or_else(|| invalid("mask glyph does not exist"))?;
            let layer = match &params.layer {
                Some(id) => glyph.layer(id),
                None => glyph.master_layer(&doc.font.default_master().id),
            }
            .ok_or_else(|| invalid("mask source does not exist"))?;
            Ok::<_, ApiError>((doc.font.clone(), layer.id.clone()))
        })
        .map_err(ApiError::from)??;
    let path = fc_geom::flatten_layer_by_id(&font, &params.glyph, &layer)
        .map_err(invalid)?
        .to_svg();
    fc_model::validate_effect_mask(&path).map_err(invalid)?;
    Ok(path)
}

pub(crate) fn apply(api: &Api, params: LayerEffectsParams) -> Result<LayerEffectsResult, ApiError> {
    let (mut font, generation) = api
        .session
        .with_doc(params.doc, |doc| {
            if !params.dry_run && params.expected_generation != Some(doc.generation) {
                return Err(invalid("effect preview is stale; preview this layer again"));
            }
            let glyph = doc
                .font
                .glyph(&params.glyph)
                .ok_or_else(|| invalid("effect glyph does not exist"))?;
            if glyph.locked {
                return Err(invalid("the effect glyph is locked"));
            }
            let layer = glyph
                .layer(&params.layer)
                .ok_or_else(|| invalid("effect layer does not exist"))?;
            let mut draft = layer.clone();
            draft.effects.clone_from(&params.effects);
            draft.validate_effects().map_err(invalid)?;
            Ok((doc.font.clone(), doc.generation))
        })
        .map_err(ApiError::from)??;
    // A source-only comparison stays available even if an old stack no longer
    // evaluates after a source edit. This lets the user repair or remove it.
    let before =
        fc_model::flatten_layer_by_id(&font, &params.glyph, &params.layer).map_err(invalid)?;
    let layer = font
        .glyph_mut(&params.glyph)
        .and_then(|g| g.layers.iter_mut().find(|l| l.id == params.layer))
        .ok_or_else(|| invalid("effect source disappeared"))?;
    layer.effects.clone_from(&params.effects);
    let width = layer.width;
    let name = layer.name.clone();
    let master = super::glyphs::layer_master(layer);
    let name = if name.is_empty() {
        master
            .as_ref()
            .and_then(|id| font.master(id))
            .map_or_else(String::new, |master| master.name.clone())
    } else {
        name
    };
    let output = fc_geom::evaluate_layer(&font, &params.glyph, &params.layer).map_err(invalid)?;
    let bounds = if before.is_empty() {
        output.path.bounding_box()
    } else if output.path.is_empty() {
        before.bounding_box()
    } else {
        before.bounding_box().union(output.path.bounding_box())
    };
    let mut result = LayerEffectsResult {
        generation,
        example: StyleBatchExample {
            glyph: params.glyph.clone(),
            master: name,
            before: before.to_svg(),
            after: output.path.to_svg(),
            before_width: width,
            after_width: width,
            bounds: [bounds.x0, bounds.y0, bounds.x1, bounds.y1],
        },
        approximated_layers: output.approximated_layers,
        change: None,
    };
    if !params.dry_run {
        let command = if params.bake {
            bake_layer(&font, &params.glyph, &params.layer, output.path)?
        } else {
            Command::SetLayerEffects {
                glyph: params.glyph.clone().into(),
                layer: params.layer.clone(),
                effects: params.effects,
            }
        };
        result.change = Some(
            api.session
                .with_doc_mut(params.doc, |doc| {
                    if doc.generation != generation {
                        return Err(invalid("effect preview is stale; preview this layer again"));
                    }
                    doc.apply(
                        command,
                        if params.bake {
                            "Bake layer effects"
                        } else {
                            "Set layer effects"
                        },
                    )
                    .map_err(ApiError::from)
                })
                .map_err(ApiError::from)??,
        );
    }
    Ok(result)
}

/// Replace the evaluated layer and remove operations referring to its former source nodes.
pub(crate) fn bake_layer(
    font: &fc_model::Font,
    glyph: &str,
    layer: &fc_model::LayerId,
    path: kurbo::BezPath,
) -> Result<Command, ApiError> {
    let source = font
        .glyph(glyph)
        .and_then(|glyph| glyph.layer(layer))
        .ok_or_else(|| invalid("bake source does not exist"))?;
    let master = super::glyphs::layer_master(source);
    let contours = fc_geom::from_bezpaths(
        &[path],
        match fc_model::query::declared_curve_format(font, master.as_ref()) {
            Some(fc_model::query::CurveFormat::Quadratic) => {
                fc_geom::CurveKind::Quadratic { max_err: 0.1 }
            }
            _ => fc_geom::CurveKind::Cubic,
        },
    );
    // Clear source references before replacing the points they identify.
    Ok(Command::Batch(vec![
        Command::SetConstructions {
            glyph: glyph.into(),
            layer: layer.clone(),
            constructions: Vec::new(),
        },
        Command::SetLayerEffects {
            glyph: glyph.into(),
            layer: layer.clone(),
            effects: Vec::new(),
        },
        Command::NodeLinks(fc_doc::NodeLinkCommand::Clear {
            glyph: glyph.into(),
            layer: layer.clone(),
        }),
        Command::ReplaceLayerOutline {
            glyph: glyph.into(),
            layer: layer.clone(),
            contours,
            components: Vec::new(),
            anchors: source.anchors.clone(),
        },
    ]))
}
