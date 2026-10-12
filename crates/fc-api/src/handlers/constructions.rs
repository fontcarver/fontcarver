//! Evaluate attachments outside the session lock and commit one reviewed revision.

use crate::{
    Api, ApiError, ConstructionCreate, ConstructionCreateKind, LayerConstructionsParams,
    LayerEffectsResult, StyleBatchExample,
};
use fc_doc::Command;
use fc_model::{ConstructionKind, GlyphConstruction, Layer};
use kurbo::Shape;

fn invalid(error: impl std::fmt::Display) -> ApiError {
    ApiError::invalid_params(error.to_string())
}

/// Evaluate a construction draft and optionally commit it against its preview revision.
pub(crate) fn apply(
    api: &Api,
    params: LayerConstructionsParams,
) -> Result<LayerEffectsResult, ApiError> {
    let (mut font, generation) = api
        .session
        .with_doc(params.doc, |doc| {
            if !params.dry_run && params.expected_generation != Some(doc.generation) {
                return Err(invalid(
                    "construction preview is stale; preview this layer again",
                ));
            }
            let glyph = doc
                .font
                .glyph(&params.glyph)
                .ok_or_else(|| invalid("construction glyph does not exist"))?;
            if glyph.locked {
                return Err(invalid("the construction glyph is locked"));
            }
            glyph
                .layer(&params.layer)
                .ok_or_else(|| invalid("construction layer does not exist"))?;
            Ok((doc.font.clone(), doc.generation))
        })
        .map_err(ApiError::from)??;
    // Keep a raw-source comparison available when removing a broken attachment.
    let before =
        fc_model::flatten_layer_by_id(&font, &params.glyph, &params.layer).map_err(invalid)?;
    let layer = font
        .glyph_mut(&params.glyph)
        .and_then(|glyph| {
            glyph
                .layers
                .iter_mut()
                .find(|layer| layer.id == params.layer)
        })
        .ok_or_else(|| invalid("construction source disappeared"))?;
    layer.constructions.clone_from(&params.constructions);
    if let Some(add) = &params.add {
        let construction = create(layer, add)?;
        layer.constructions.push(construction);
    }
    layer.validate_constructions().map_err(invalid)?;
    let draft = layer.clone();
    let master = super::glyphs::layer_master(&draft);
    let name = if draft.name.is_empty() {
        master
            .as_ref()
            .and_then(|id| font.master(id))
            .map_or_else(String::new, |master| master.name.clone())
    } else {
        draft.name.clone()
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
            before_width: draft.width,
            after_width: draft.width,
            bounds: [bounds.x0, bounds.y0, bounds.x1, bounds.y1],
        },
        approximated_layers: output.approximated_layers,
        change: None,
    };
    if !params.dry_run {
        let command = if params.bake {
            super::effects::bake_layer(&font, &params.glyph, &params.layer, output.path)?
        } else {
            let mut commands = Vec::new();
            if params.add.is_some() {
                commands.push(Command::ReplaceLayerOutline {
                    glyph: params.glyph.clone().into(),
                    layer: params.layer.clone(),
                    contours: draft.contours,
                    components: draft.components,
                    anchors: draft.anchors,
                });
            }
            commands.push(Command::SetConstructions {
                glyph: params.glyph.clone().into(),
                layer: params.layer.clone(),
                constructions: draft.constructions,
            });
            Command::Batch(commands)
        };
        result.change = Some(
            api.session
                .with_doc_mut(params.doc, |doc| {
                    if doc.generation != generation {
                        return Err(invalid(
                            "construction preview is stale; preview this layer again",
                        ));
                    }
                    doc.apply(
                        command,
                        if params.bake {
                            "Bake glyph constructions"
                        } else {
                            "Set glyph constructions"
                        },
                    )
                    .map_err(ApiError::from)
                })
                .map_err(ApiError::from)??,
        );
    }
    Ok(result)
}

fn create(layer: &mut Layer, params: &ConstructionCreate) -> Result<GlyphConstruction, ApiError> {
    let mut node = |point: &fc_doc::PointRef| {
        layer
            .ensure_node_id(point.contour, point.point)
            .map_err(invalid)
    };
    let kind = match &params.kind {
        ConstructionCreateKind::Corner { at, scale } => ConstructionKind::Corner {
            at: node(at)?,
            scale: *scale,
        },
        ConstructionCreateKind::Cap { at, scale } => ConstructionKind::Cap {
            at: node(at)?,
            scale: *scale,
        },
        ConstructionCreateKind::Segment {
            start,
            end,
            source_length,
        } => ConstructionKind::Segment {
            start: node(start)?,
            end: node(end)?,
            source_length: *source_length,
        },
        ConstructionCreateKind::Brush { at, spacing, scale } => ConstructionKind::Brush {
            at: node(at)?,
            spacing: *spacing,
            scale: *scale,
        },
    };
    Ok(GlyphConstruction {
        id: fc_model::LayerId::new().to_string(),
        source: params.source.as_str().into(),
        kind,
        keep_carrier: params.keep_carrier,
    })
}
