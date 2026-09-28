//! Prepare original sources, compute outside the session lock, then commit atomically.
use crate::{Api, ApiError, StyleBatchExample, StyleBatchParams, StyleBatchResult};
use fc_doc::Command;
use fc_geom::{CurveKind, StyleOptions};
use fc_model::{
    query::{declared_curve_format, CurveFormat},
    Anchor, Contour, GlyphName, Layer, LayerId, LayerKind, MasterId,
};
use kurbo::{Affine, BezPath, Point, Shape};
use std::{
    collections::HashSet,
    time::{SystemTime, UNIX_EPOCH},
};

struct Source {
    glyph: GlyphName,
    layer: LayerId,
    master: MasterId,
    master_name: String,
    kind: CurveKind,
    outline: Layer,
}
fn invalid(message: impl ToString) -> ApiError {
    ApiError::invalid_params(message.to_string())
}
fn path(contours: &[Contour]) -> BezPath {
    let mut path = BezPath::new();
    for contour in contours {
        path.extend(contour.to_bezpath());
    }
    path
}

pub(crate) fn run(api: &Api, params: StyleBatchParams) -> Result<StyleBatchResult, ApiError> {
    let options = StyleOptions::from(&params.options);
    options.validate().map_err(invalid)?;
    if params.glyphs.is_empty() || params.glyphs.len() > 512 {
        return Err(invalid(
            "select between one and 512 glyphs for a style batch",
        ));
    }
    if params.label.len() > 512 {
        return Err(invalid("style label is too long"));
    }
    if options == StyleOptions::default() && !params.center {
        return Err(invalid("choose a style effect before previewing"));
    }
    let (mut result, sources) = api
        .session
        .with_doc(params.doc, |doc| {
            if !params.dry_run && params.expected_generation != Some(doc.generation) {
                return Err(invalid(
                    "style preview is stale; preview the selection again",
                ));
            }
            if params
                .master
                .as_ref()
                .is_some_and(|id| doc.font.master(id).is_none())
            {
                return Err(invalid("style master does not exist"));
            }
            let mut result = StyleBatchResult {
                generation: doc.generation,
                glyph_count: 0,
                layer_count: 0,
                decomposed_layers: 0,
                approximated_layers: 0,
                approximated_sources: Vec::new(),
                examples: Vec::new(),
                change: None,
            };
            let mut sources = Vec::new();
            let mut seen = HashSet::new();
            let mut points = 0usize;
            for gid in &params.glyphs {
                if !seen.insert(*gid) {
                    continue;
                }
                let name = doc
                    .glyph_name(*gid)
                    .ok_or_else(|| invalid("selected glyph no longer exists"))?;
                let glyph = doc
                    .font
                    .glyph(name)
                    .ok_or_else(|| invalid("selected glyph is missing"))?;
                if glyph.locked {
                    return Err(invalid(format!("{name} is locked")));
                }
                result.glyph_count += 1;
                for master in doc
                    .font
                    .masters
                    .iter()
                    .filter(|m| params.master.as_ref().is_none_or(|id| *id == m.id))
                {
                    let layer = glyph.master_layer(&master.id).ok_or_else(|| {
                        invalid(format!("{name} has no source for {}", master.name))
                    })?;
                    result.layer_count += 1;
                    if result.layer_count > 1024 || layer.anchors.len() > 4096 {
                        return Err(invalid(
                            "style batch exceeds 1024 layers or 4096 anchors per layer",
                        ));
                    }
                    let kind = match declared_curve_format(&doc.font, Some(&master.id)) {
                        Some(CurveFormat::Quadratic) => CurveKind::Quadratic { max_err: 0.1 },
                        _ => CurveKind::Cubic,
                    };
                    let contours = if layer.components.is_empty() && !layer.has_live_geometry() {
                        if layer.contours.len() > 4096
                            || layer.contours.iter().map(|c| c.points.len()).sum::<usize>() > 4096
                        {
                            return Err(invalid(format!("{name} exceeds 4096 source points")));
                        }
                        layer.contours.clone()
                    } else {
                        if !layer.components.is_empty() {
                            result.decomposed_layers += 1;
                        }
                        let evaluated =
                            fc_geom::evaluate_layer(&doc.font, name, &layer.id).map_err(invalid)?;
                        if !evaluated.approximated_layers.is_empty() {
                            result.approximated_layers += 1;
                            result
                                .approximated_sources
                                .push(format!("{name} / {} (live effects)", master.name));
                        }
                        let flat = evaluated.path;
                        if flat.elements().len() > 4096 {
                            return Err(invalid(format!("{name} exceeds 4096 resolved segments")));
                        }
                        fc_geom::from_bezpaths(&[flat], kind)
                    };
                    points = points.saturating_add(
                        contours.iter().map(|c| c.points.len()).sum::<usize>()
                            + layer.anchors.len(),
                    );
                    if points > 100_000 {
                        return Err(invalid(
                            "style batch exceeds 100000 resolved points and anchors",
                        ));
                    }
                    sources.push(Source {
                        glyph: name.clone(),
                        layer: layer.id.clone(),
                        master: master.id.clone(),
                        master_name: master.name.clone(),
                        kind,
                        outline: Layer {
                            contours,
                            anchors: layer.anchors.clone(),
                            width: layer.width,
                            vertical_width: layer.vertical_width,
                            vertical_origin: layer.vertical_origin,
                            ..Default::default()
                        },
                    });
                }
            }
            Ok((result, sources))
        })
        .map_err(ApiError::from)??;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(invalid)?
        .as_secs()
        .min(i64::MAX as u64) as i64;
    let mut commands = Vec::new();
    let mut output_points = 0usize;
    for source in sources {
        let mut outcome = fc_geom::style_effect(&source.outline.contours, &options, source.kind)
            .map_err(|e| invalid(format!("{} / {}: {e}", source.glyph, source.master_name)))?;
        if outcome.fallback != fc_geom::FallbackKind::None {
            result.approximated_layers += 1;
            result
                .approximated_sources
                .push(format!("{} / {}", source.glyph, source.master_name));
        }
        let width = source.outline.width
            * if params.scale_advance {
                options.scale[0]
            } else {
                1.
            };
        if !width.is_finite() || !(0. ..=65535.).contains(&width) {
            return Err(invalid(
                "styled horizontal advance exceeds the OpenType range",
            ));
        }
        let before = path(&source.outline.contours);
        let mut after = path(&outcome.contours);
        if params.center && !after.elements().is_empty() {
            let bounds = after.bounding_box();
            let shift = Affine::translate(((width - bounds.width()) / 2. - bounds.x0, 0.));
            outcome.contours = fc_geom::transform(&outcome.contours, shift);
            outcome.transform = shift * outcome.transform;
            after = shift * after;
            if !after.is_finite()
                || after.bounding_box().x0 < -32768.
                || after.bounding_box().x1 > 32767.
                || outcome
                    .contours
                    .iter()
                    .flat_map(|c| &c.points)
                    .any(|p| p.x.abs() > 32767.)
            {
                return Err(invalid("centered outline exceeds the OpenType design grid"));
            }
        }
        let anchors = source
            .outline
            .anchors
            .iter()
            .map(|anchor| {
                let point = outcome.transform * Point::new(anchor.x, anchor.y);
                if !point.is_finite() || point.x.abs() > 32767. || point.y.abs() > 32767. {
                    return Err(invalid("styled anchor exceeds the OpenType design grid"));
                }
                Ok(Anchor {
                    name: anchor.name.clone(),
                    x: point.x,
                    y: point.y,
                })
            })
            .collect::<Result<Vec<_>, ApiError>>()?;
        output_points = output_points.saturating_add(
            outcome
                .contours
                .iter()
                .map(|c| c.points.len())
                .sum::<usize>(),
        );
        if output_points > 250_000 {
            return Err(invalid("style batch exceeds 250000 output points"));
        }
        if result.examples.len() < 12 {
            let bounds = before.bounding_box().union(after.bounding_box());
            result.examples.push(StyleBatchExample {
                glyph: source.glyph.to_string(),
                master: source.master_name,
                before: before.to_svg(),
                after: after.to_svg(),
                before_width: source.outline.width,
                after_width: width,
                bounds: [bounds.x0, bounds.y0, bounds.x1, bounds.y1],
            });
        }
        if params.backup {
            let mut backup = source.outline;
            backup.kind = LayerKind::Backup {
                of: source.master,
                time: timestamp,
            };
            backup.name = params.label.clone();
            commands.push(Command::AddLayer {
                glyph: source.glyph.clone(),
                layer: backup,
            });
        }
        commands.push(Command::ReplaceLayerOutline {
            glyph: source.glyph.clone(),
            layer: source.layer.clone(),
            contours: outcome.contours,
            components: Vec::new(),
            anchors,
        });
        commands.push(Command::SetLayerEffects {
            glyph: source.glyph.clone(),
            layer: source.layer.clone(),
            effects: Vec::new(),
        });
        commands.push(Command::SetWidth {
            glyph: source.glyph,
            layer: source.layer,
            width,
        });
    }
    if !params.dry_run {
        result.change = Some(
            api.session
                .with_doc_mut(params.doc, |doc| {
                    if doc.generation != result.generation {
                        return Err(invalid(
                            "style preview is stale; preview the selection again",
                        ));
                    }
                    doc.apply(Command::Batch(commands), params.label)
                        .map_err(ApiError::from)
                })
                .map_err(ApiError::from)??,
        );
    }
    Ok(result)
}
