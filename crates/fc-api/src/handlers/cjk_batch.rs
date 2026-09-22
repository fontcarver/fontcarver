//! Selection-based CJK spacing with bounded preparation and atomic history.
use crate::{Api, ApiError, CjkBatchExample, CjkBatchParams, CjkBatchResult};
use fc_doc::Command;
use kurbo::Shape;
use std::collections::HashSet;

pub(crate) fn run(api: &Api, params: CjkBatchParams) -> Result<CjkBatchResult, ApiError> {
    validate(&params)?;
    api.session
        .with_doc_mut(params.doc, |doc| {
            if !params.dry_run && params.expected_generation != Some(doc.generation) {
                return Err(ApiError::invalid_params(
                    "spacing preview is stale; preview the selection again",
                ));
            }
            if let Some(master) = &params.master {
                if doc.font.master(master).is_none() {
                    return Err(ApiError::not_found("spacing master does not exist"));
                }
            }
            let mut result = CjkBatchResult {
                generation: doc.generation,
                glyph_count: 0,
                layer_count: 0,
                examples: Vec::new(),
                change: None,
            };
            let mut commands = Vec::new();
            let mut seen = HashSet::new();
            let mut points = 0usize;
            let mut names = Vec::new();
            for gid in &params.glyphs {
                if seen.insert(*gid) {
                    names.push(
                        doc.glyph_name(*gid).cloned().ok_or_else(|| {
                            ApiError::not_found("selected glyph no longer exists")
                        })?,
                    );
                }
            }
            if params.center {
                names = dependency_order(&doc.font, &names, params.master.as_ref())?;
            }
            for name in &names {
                let glyph = doc
                    .font
                    .glyph(name)
                    .ok_or_else(|| ApiError::not_found("selected glyph is missing"))?;
                result.glyph_count += 1;
                for master in doc
                    .font
                    .masters
                    .iter()
                    .filter(|master| params.master.as_ref().is_none_or(|id| master.id == *id))
                {
                    let layer = glyph.master_layer(&master.id).ok_or_else(|| {
                        ApiError::invalid_params(format!(
                            "{} has no source for {}",
                            name, master.name
                        ))
                    })?;
                    result.layer_count += 1;
                    if result.layer_count > 131_072 {
                        return Err(ApiError::invalid_params(
                            "spacing operation exceeds 131072 source layers",
                        ));
                    }
                    points = points.saturating_add(
                        layer.contours.iter().map(|c| c.points.len()).sum::<usize>(),
                    );
                    if points > 8_000_000 {
                        return Err(ApiError::invalid_params(
                            "spacing operation exceeds eight million source points",
                        ));
                    }
                    let width = params.width.unwrap_or(layer.width);
                    if !width.is_finite() || !(0.0..=65535.0).contains(&width) {
                        return Err(ApiError::invalid_params(
                            "horizontal advance exceeds the OpenType range",
                        ));
                    }
                    if result.examples.len() < 12 {
                        result.examples.push(CjkBatchExample {
                            glyph: name.to_string(),
                            master: master.name.clone(),
                            before: layer.width,
                            after: width,
                        });
                    }
                    if params.center {
                        let path = fc_model::flatten_layer(&doc.font, name, &master.id)
                            .map_err(|e| ApiError::invalid_params(e.to_string()))?;
                        points = points.saturating_add(path.elements().len());
                        if points > 8_000_000 {
                            return Err(ApiError::invalid_params(
                                "spacing operation exceeds eight million resolved points",
                            ));
                        }
                        if !path.elements().is_empty() {
                            let bounds = path.bounding_box();
                            let side = (width - bounds.width()) / 2.0;
                            if !side.is_finite() {
                                return Err(ApiError::invalid_params(
                                    "outline bounds are not finite",
                                ));
                            }
                            commands.push(Command::CenterMetrics {
                                glyph: name.clone(),
                                layer: layer.id.clone(),
                                width,
                            });
                        } else if params.width.is_some() {
                            commands.push(Command::SetWidth {
                                glyph: name.clone(),
                                layer: layer.id.clone(),
                                width,
                            });
                        }
                    } else if params.width.is_some() {
                        commands.push(Command::SetWidth {
                            glyph: name.clone(),
                            layer: layer.id.clone(),
                            width,
                        });
                    }
                    if params.vertical {
                        commands.push(Command::SetVerticalMetrics {
                            glyph: name.clone(),
                            layer: layer.id.clone(),
                            advance: params.advance,
                            origin: params.origin,
                        });
                    }
                }
            }
            if !params.dry_run {
                result.change = Some(
                    doc.apply(Command::Batch(commands), params.label)
                        .map_err(ApiError::from)?,
                );
            }
            Ok(result)
        })
        .map_err(ApiError::from)?
}

fn validate(params: &CjkBatchParams) -> Result<(), ApiError> {
    if params.glyphs.is_empty() || params.glyphs.len() > 65_535 {
        return Err(ApiError::invalid_params(
            "select between one and 65535 glyphs",
        ));
    }
    if params.width.is_none() && !params.center && !params.vertical {
        return Err(ApiError::invalid_params("choose a spacing operation"));
    }
    for advance in [params.width, params.advance].into_iter().flatten() {
        if !advance.is_finite() || !(0.0..=65535.0).contains(&advance) {
            return Err(ApiError::invalid_params(
                "advance exceeds the OpenType range",
            ));
        }
    }
    if params
        .origin
        .is_some_and(|origin| !origin.is_finite() || !(-32768.0..=32767.0).contains(&origin))
    {
        return Err(ApiError::invalid_params(
            "vertical origin exceeds the OpenType range",
        ));
    }
    Ok(())
}

/// Visit through unselected composites too, since they can connect two selected glyphs.
fn dependency_order(
    font: &fc_model::Font,
    names: &[fc_model::GlyphName],
    master: Option<&fc_model::MasterId>,
) -> Result<Vec<fc_model::GlyphName>, ApiError> {
    struct Walk<'a> {
        font: &'a fc_model::Font,
        master: Option<&'a fc_model::MasterId>,
        selected: HashSet<fc_model::GlyphName>,
        visiting: HashSet<fc_model::GlyphName>,
        done: HashSet<fc_model::GlyphName>,
        ordered: Vec<fc_model::GlyphName>,
        remaining: usize,
    }
    impl Walk<'_> {
        fn visit(&mut self, name: &fc_model::GlyphName) -> Result<(), ApiError> {
            if self.done.contains(name) {
                return Ok(());
            }
            if self.visiting.len() >= 64 || !self.visiting.insert(name.clone()) {
                return Err(ApiError::invalid_params(
                    "cyclic or excessively deep spacing components",
                ));
            }
            let glyph = self
                .font
                .glyph(name)
                .ok_or_else(|| ApiError::not_found("spacing component is missing"))?;
            for layer in &glyph.layers {
                let owner = match &layer.kind {
                    fc_model::LayerKind::Master(id)
                    | fc_model::LayerKind::Smart { master: id, .. } => id,
                    _ => continue,
                };
                if self.master.is_some_and(|id| id != owner) {
                    continue;
                }
                for component in &layer.components {
                    self.remaining = self
                        .remaining
                        .checked_sub(1)
                        .ok_or_else(|| ApiError::invalid_params("spacing component work limit"))?;
                    self.visit(&component.base)?;
                }
            }
            self.visiting.remove(name);
            self.done.insert(name.clone());
            if self.selected.contains(name) {
                self.ordered.push(name.clone());
            }
            Ok(())
        }
    }
    let mut walk = Walk {
        font,
        master,
        selected: names.iter().cloned().collect(),
        visiting: HashSet::new(),
        done: HashSet::new(),
        ordered: Vec::new(),
        remaining: 8_000_000,
    };
    for name in names {
        walk.visit(name)?;
    }
    Ok(walk.ordered)
}
