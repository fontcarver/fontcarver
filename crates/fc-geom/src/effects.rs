//! Shared live-effect evaluation for editors, components, rasterization and export.
use std::{cell::RefCell, collections::BTreeMap};

use fc_model::{
    Contour, EffectBoolean, EffectCap, EffectJoin, EffectNib, Error, Font, Layer, LayerEffectKind,
    LayerId, MasterId,
};
use kurbo::{Affine, BezPath, ParamCurve, Rect, Shape};

use crate::{BooleanOp, Cap, CurveKind, FallbackKind, Join, Nib, StrokeOptions};

/// Evaluated drawing and the quality information needed before applying or baking it.
#[derive(Clone, Debug)]
pub struct EffectOutcome {
    /// Final path, with component effects applied before their placement transforms.
    pub path: BezPath,
    /// Source layers for which the evaluator used polygon curve refitting.
    pub approximated_layers: Vec<LayerId>,
    /// Whether any enabled effect contributed to this result.
    pub affected: bool,
}

/// Whether a source or one of its nested component sources has an enabled effect.
/// Cycles and missing sources conservatively return true so evaluation can report them.
pub(crate) fn has_effects(font: &Font, layer: &Layer) -> bool {
    fn visit(font: &Font, layer: &Layer, seen: &mut Vec<LayerId>, remaining: &mut usize) -> bool {
        if *remaining == 0 {
            return true;
        }
        *remaining -= 1;
        if layer.effects.iter().any(|e| e.enabled) || seen.len() >= 64 || seen.contains(&layer.id) {
            return true;
        }
        seen.push(layer.id.clone());
        let master = match &layer.kind {
            fc_model::LayerKind::Master(id)
            | fc_model::LayerKind::Background(id)
            | fc_model::LayerKind::Brace { master: id, .. }
            | fc_model::LayerKind::Bracket { master: id, .. }
            | fc_model::LayerKind::Smart { master: id, .. }
            | fc_model::LayerKind::Backup { of: id, .. } => id,
            fc_model::LayerKind::Custom(_) => &font.default_master().id,
        };
        let found = layer.components.iter().any(|c| {
            font.glyph(&c.base)
                .and_then(|g| g.master_layer(master))
                .is_none_or(|base| visit(font, base, seen, remaining))
        });
        seen.pop();
        found
    }
    visit(font, layer, &mut Vec::new(), &mut 1_000_000)
}

fn invalid(error: impl std::fmt::Display) -> Error {
    Error::InvalidEffects(error.to_string())
}

/// Resolve a master outline and evaluate every participating layer's effects.
pub fn flatten_layer(font: &Font, glyph: &str, master: &MasterId) -> Result<BezPath, Error> {
    fc_model::query::flatten_layer_with(font, glyph, master, &|layer, path| {
        apply(layer, path).map(|(path, _)| path)
    })
}

/// Resolve one selected layer, including its component and local effects.
pub fn flatten_layer_by_id(font: &Font, glyph: &str, layer: &LayerId) -> Result<BezPath, Error> {
    Ok(evaluate_layer(font, glyph, layer)?.path)
}

/// Interpolate local smart sources before evaluating their shared effect stack.
pub fn flatten_smart_component(
    font: &Font,
    glyph: &str,
    master: &MasterId,
    values: &BTreeMap<String, f64>,
) -> Result<BezPath, Error> {
    fc_model::query::flatten_smart_component_with(font, glyph, master, values, &|layer, path| {
        apply(layer, path).map(|(path, _)| path)
    })
}

/// Bounds of the final evaluated master drawing; empty paths have no bounds.
pub fn layer_bbox(font: &Font, glyph: &str, master: &MasterId) -> Result<Option<Rect>, Error> {
    let path = flatten_layer(font, glyph, master)?;
    Ok((!path.is_empty()).then(|| path.bounding_box()))
}

/// Evaluate a selected layer and collect component and local fallback information.
pub fn evaluate_layer(font: &Font, glyph: &str, layer: &LayerId) -> Result<EffectOutcome, Error> {
    let approximated = RefCell::new(Vec::new());
    let affected = std::cell::Cell::new(false);
    let path = fc_model::query::flatten_layer_by_id_with(font, glyph, layer, &|layer, path| {
        affected.set(affected.get() || layer.effects.iter().any(|e| e.enabled));
        let (path, fallback) = apply(layer, path)?;
        if fallback && !approximated.borrow().contains(&layer.id) {
            approximated.borrow_mut().push(layer.id.clone());
        }
        Ok(path)
    })?;
    Ok(EffectOutcome {
        path,
        approximated_layers: approximated.into_inner(),
        affected: affected.get(),
    })
}

/// Bake effects on a separate font, resolving every affected layer from the original.
///
/// Component dependents are decomposed only when an enabled effect contributes to
/// their drawing. Source anchors and metrics are retained; point-bound hints and
/// programs on changed layers are cleared. Disabled stacks are removed from the copy.
pub fn bake_effects_font(font: &Font) -> Result<(Font, Vec<LayerId>), Error> {
    font.validate_effects()?;
    let mut result = font.clone();
    let mut approximated = Vec::new();
    if !font
        .glyphs
        .values()
        .flat_map(|g| &g.layers)
        .any(|l| l.effects.iter().any(|e| e.enabled))
    {
        for layer in result.glyphs.values_mut().flat_map(|g| &mut g.layers) {
            layer.effects.clear();
        }
        return Ok((result, approximated));
    }
    for glyph in result.glyphs.values_mut() {
        for layer in &mut glyph.layers {
            let evaluated = evaluate_layer(font, &glyph.name, &layer.id)?;
            if evaluated.affected {
                layer.contours = Contour::from_bezpath(&evaluated.path);
                layer.components.clear();
                layer.hints = Default::default();
                layer.tt_instructions = None;
                layer.tt_binding = None;
            }
            layer.effects.clear();
            for id in evaluated.approximated_layers {
                if !approximated.contains(&id) {
                    approximated.push(id);
                }
            }
        }
    }
    Ok((result, approximated))
}

fn apply(layer: &Layer, source: BezPath) -> Result<(BezPath, bool), Error> {
    layer.validate_effects()?;
    if !layer.effects.iter().any(|e| e.enabled) {
        return Ok((source, false));
    }
    check_path(&source, 4096)?;
    let mut contours = Contour::from_bezpath(&source);
    let mut fallback = false;
    let mut remaining = 262_144usize;
    for entry in layer.effects.iter().filter(|e| e.enabled) {
        check_path(&joined_path(&contours), 4096)?;
        let (next, approximated) = match &entry.effect {
            LayerEffectKind::Stroke {
                nib,
                width,
                height,
                angle,
                cap,
                join,
                miter_limit,
            } => {
                let mut opts = StrokeOptions {
                    width: *width,
                    cap: match cap {
                        EffectCap::Round => Cap::Round,
                        EffectCap::Butt => Cap::Butt,
                        EffectCap::Square => Cap::Square,
                    },
                    join: match join {
                        EffectJoin::Round => Join::Round,
                        EffectJoin::Bevel => Join::Bevel,
                        EffectJoin::Miter => Join::Miter,
                    },
                    miter_limit: *miter_limit,
                    ..Default::default()
                };
                let rotation = Affine::rotate(angle.to_radians());
                // Only enlarge the working grid, keeping the stroker tolerance
                // at or below its specified size in final font coordinates.
                let diameter = width.max(*height);
                let undo =
                    rotation * Affine::scale_non_uniform(width / diameter, height / diameter);
                let input = match nib {
                    EffectNib::Ellipse => {
                        opts.width = diameter;
                        crate::transform(&contours, undo.inverse())
                    }
                    EffectNib::Rectangle => {
                        opts.nib = Nib::Calligraphic {
                            width: *width,
                            height: *height,
                            angle: angle.to_radians(),
                        };
                        contours.clone()
                    }
                };
                let expanded = effect_stroke(&input, &opts).map_err(invalid)?;
                let output = if *nib == EffectNib::Ellipse {
                    crate::transform(&expanded.contours, undo)
                } else {
                    expanded.contours
                };
                (output, expanded.fallback != FallbackKind::None)
            }
            LayerEffectKind::RemoveOverlap => {
                let output = crate::remove_overlap_with_fallback(&contours, CurveKind::Cubic, 0.01)
                    .map_err(invalid)?;
                (output.contours, output.fallback != FallbackKind::None)
            }
            LayerEffectKind::Boolean { operation, path } => {
                if contours.iter().any(|c| !c.closed) {
                    return Err(invalid("filled boolean effects require closed source contours; put a stroke before this operation"));
                }
                let mask = Contour::from_bezpath(&fc_model::validate_effect_mask(path)?);
                let op = match operation {
                    EffectBoolean::Union => BooleanOp::Union,
                    EffectBoolean::Intersect => BooleanOp::Intersect,
                    EffectBoolean::Subtract => BooleanOp::Exclude,
                    EffectBoolean::Xor => BooleanOp::Xor,
                };
                crate::filled::combine(&contours, &mask, op).map_err(invalid)?
            }
        };
        let path = joined_path(&next);
        check_path(&path, 65_535)?;
        remaining = remaining
            .checked_sub(path.elements().len())
            .ok_or_else(|| invalid("effect stack exceeds its geometry work budget"))?;
        contours = next;
        fallback |= approximated;
    }
    Ok((joined_path(&contours), fallback))
}

// Cover a closed centerline with open spans between smooth segment midpoints.
// Each span retains a source join, and butt caps meet without adding new joins.
// This prevents inverted inward offsets from becoming spurious counters when
// a pen is wider than the enclosed area.
fn effect_stroke(
    contours: &[Contour],
    opts: &StrokeOptions,
) -> Result<crate::StrokeOutcome, crate::Error> {
    let mut expanded = Vec::new();
    let mut fallback = false;
    for contour in contours {
        let path = contour.to_bezpath();
        let mut segments: Vec<_> = path.segments().collect();
        if !contour.closed || segments.is_empty() {
            let result = expand_effect_span(contour, opts)?;
            expanded.extend(result.contours);
            fallback |= result.fallback != FallbackKind::None;
            continue;
        }
        if segments.len() == 1 {
            let only = segments[0];
            segments = (0..4)
                .map(|i| only.subsegment(f64::from(i) * 0.25..f64::from(i + 1) * 0.25))
                .collect();
        }
        let patch_options = StrokeOptions {
            cap: Cap::Butt,
            ..*opts
        };
        for (left, right) in segments
            .iter()
            .zip(segments.iter().cycle().skip(1))
            .take(segments.len())
        {
            let left = left.subsegment(0.5..1.);
            let right = right.subsegment(0. ..0.5);
            let mut patch = BezPath::new();
            patch.move_to(left.start());
            patch.push(left.as_path_el());
            patch.push(right.as_path_el());
            let mut patch_contours = Contour::from_bezpath(&patch);
            let Some(patch) = patch_contours.pop() else {
                continue;
            };
            let result = expand_effect_span(&patch, &patch_options)?;
            expanded.extend(result.contours);
            fallback |= result.fallback != FallbackKind::None;
        }
    }
    let (contours, refitted) = crate::filled::combine(&expanded, &[], BooleanOp::Union)?;
    Ok(crate::StrokeOutcome {
        contours,
        fallback: if fallback || refitted {
            FallbackKind::Flattened
        } else {
            FallbackKind::None
        },
    })
}

// Keep the stroker's signed winding until its nonzero fill is resolved. Splitting a
// self-intersecting inside join into nested rings and correcting their direction
// can incorrectly turn its doubly covered triangle into an empty counter.
fn expand_effect_span(
    contour: &Contour,
    opts: &StrokeOptions,
) -> Result<crate::StrokeOutcome, crate::Error> {
    if !matches!(opts.nib, Nib::Round) {
        return crate::expand_stroke_with_fallback(
            std::slice::from_ref(contour),
            opts,
            CurveKind::Cubic,
        );
    }
    let style = kurbo::Stroke::new(opts.width)
        .with_join(opts.join)
        .with_miter_limit(opts.miter_limit)
        .with_caps(opts.cap);
    let mut path = kurbo::stroke(
        contour.to_bezpath().iter(),
        &style,
        &kurbo::StrokeOpts::default(),
        crate::DEFAULT_BOOLEAN_TOLERANCE,
    );
    if !path.is_empty() && !matches!(path.elements().last(), Some(kurbo::PathEl::ClosePath)) {
        path.close_path();
    }
    let (contours, refitted) =
        crate::filled::combine(&Contour::from_bezpath(&path), &[], BooleanOp::Union)?;
    Ok(crate::StrokeOutcome {
        // Separate spans describe a union of filled sets. Normalize only after
        // each span's nonzero fill has been resolved, so their signs cannot
        // cancel where a wide pen crosses another span.
        contours: crate::correct_direction(&contours, false),
        fallback: if refitted {
            FallbackKind::Flattened
        } else {
            FallbackKind::None
        },
    })
}

fn joined_path(contours: &[Contour]) -> BezPath {
    contours
        .iter()
        .flat_map(|c| c.to_bezpath().into_iter())
        .collect()
}

fn check_path(path: &BezPath, limit: usize) -> Result<(), Error> {
    let bounds = path.control_box();
    if !path.is_finite()
        || path.elements().len() > limit
        || [bounds.x0, bounds.y0, bounds.x1, bounds.y1]
            .iter()
            .any(|v| v.abs() > 32767.)
    {
        return Err(invalid(format!(
            "effect geometry exceeds {limit} elements or the signed font coordinate grid"
        )));
    }
    Ok(())
}
