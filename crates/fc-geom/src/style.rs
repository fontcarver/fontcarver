//! Bounded affine and filled-outline weight effects.
use crate::filled;
use crate::{
    expand_stroke_with_fallback, transform, BooleanOp, CurveKind, Error, FallbackKind,
    StrokeOptions,
};
use fc_model::Contour;
use kurbo::{Affine, ParamCurve, PathEl};

/// A style operation applies scale, then oblique shear, translation and weight.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StyleOptions {
    /// Horizontal and vertical scale factors, each in [0.05, 4].
    pub scale: [f64; 2],
    /// Font-unit translation after scaling/shearing.
    pub translate: [f64; 2],
    /// Shear angle in degrees, in [-60, 60].
    pub oblique: f64,
    /// Horizontal/vertical boundary offsets. Both zero, or same-sign nonzero values.
    pub weight: [f64; 2],
}
impl Default for StyleOptions {
    fn default() -> Self {
        Self {
            scale: [1., 1.],
            translate: [0., 0.],
            oblique: 0.,
            weight: [0., 0.],
        }
    }
}
/// Geometry and the affine portion to apply to anchors.
#[derive(Clone, Debug, PartialEq)]
pub struct StyleOutcome {
    /// Rebuilt filled outlines, or transformed open curves when weight is zero.
    pub contours: Vec<Contour>,
    /// Exact scale/shear/translation; a weight offset does not move anchors.
    pub transform: Affine,
    /// Whether a failed strict boolean required polygon union and curve refitting.
    /// Callers must surface this in the preview before applying.
    pub fallback: FallbackKind,
}
fn invalid(message: &str) -> Error {
    Error::InvalidStyle(message.into())
}
impl StyleOptions {
    /// Validate ranges before geometry work or a document mutation.
    pub fn validate(&self) -> Result<(), Error> {
        if self
            .scale
            .iter()
            .any(|v| !v.is_finite() || !(0.05..=4.).contains(v))
        {
            return Err(invalid("scale factors must be between 0.05 and 4"));
        }
        if self
            .translate
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 32768.)
            || !self.oblique.is_finite()
            || self.oblique.abs() > 60.
        {
            return Err(invalid("translation must fit font coordinates and oblique angle must be within ±60 degrees"));
        }
        if self.weight != [0., 0.]
            && (self
                .weight
                .iter()
                .any(|v| !v.is_finite() || !(0.1..=500.).contains(&v.abs()))
                || self.weight[0].signum() != self.weight[1].signum())
        {
            return Err(invalid("weight offsets must both be zero or have the same sign and magnitudes between 0.1 and 500"));
        }
        Ok(())
    }
}
fn check(contours: &[Contour], limit: usize) -> Result<(), Error> {
    if contours.len() > limit || contours.iter().map(|c| c.points.len()).sum::<usize>() > limit {
        return Err(invalid("style effect outline point budget exceeded"));
    }
    if contours
        .iter()
        .flat_map(|c| &c.points)
        .any(|p| !p.x.is_finite() || !p.y.is_finite() || p.x.abs() > 32767. || p.y.abs() > 32767.)
    {
        return Err(invalid(
            "style effect coordinates must be finite and fit the OpenType design grid",
        ));
    }
    for contour in contours {
        contour
            .try_to_bezpath()
            .map_err(|e| invalid(&e.to_string()))?;
    }
    Ok(())
}
/// Apply a bounded style effect without changing its source.
///
/// Weight uses an elliptical boundary stroke and filled union/subtraction, so
/// counters shrink when emboldened and grow when lightened. Open contours with
/// nonzero weight are rejected instead of being silently discarded by booleans.
/// A failed strict boundary union may use the existing curve-refitting fallback,
/// reported in the outcome. Unrecoverable failures return no partial result.
pub fn style_effect(
    contours: &[Contour],
    options: &StyleOptions,
    kind: CurveKind,
) -> Result<StyleOutcome, Error> {
    options.validate()?;
    if let CurveKind::Quadratic { max_err } = kind {
        if !max_err.is_finite() || max_err <= 0. {
            return Err(invalid(
                "quadratic approximation error must be positive and finite",
            ));
        }
    }
    check(contours, 4096)?;
    let affine = Affine::translate((options.translate[0], options.translate[1]))
        * Affine::new([1., 0., options.oblique.to_radians().tan(), 1., 0., 0.])
        * Affine::scale_non_uniform(options.scale[0], options.scale[1]);
    let mut result = transform(contours, affine);
    let mut fallback = FallbackKind::None;
    check(&result, 4096)?;
    if options.weight != [0., 0.] && !result.is_empty() {
        if result.iter().any(|c| !c.closed) {
            return Err(invalid("weight effects require closed contours"));
        }
        let [x, y] = options.weight.map(f64::abs);
        // Only enlarge the working coordinates, keeping all geometric tolerances
        // at least as accurate as the requested tolerance in final font units.
        let radius = x.max(y);
        let normalise = Affine::scale_non_uniform(radius / x, radius / y);
        let normalized = crate::remove_overlap_with_fallback(
            &transform(&result, normalise),
            CurveKind::Cubic,
            0.01,
        )?;
        fallback = normalized.fallback;
        let normalized = normalized.contours;
        if normalized.iter().map(|c| c.points.len()).sum::<usize>() > 16384 {
            return Err(invalid("normalized style boundary point budget exceeded"));
        }
        // Stroke each boundary segment as an open capsule. A closed offset ring
        // can invert its inner loop when the weight exceeds a thin stem, leaving
        // a false hole. A filled union of capsules correctly covers that stem.
        let segments = normalized
            .iter()
            .flat_map(|contour| {
                contour
                    .to_bezpath()
                    .segments()
                    .flat_map(|segment| {
                        let path = kurbo::BezPath::from_vec(vec![
                            PathEl::MoveTo(segment.start()),
                            segment.as_path_el(),
                        ]);
                        crate::from_bezpaths(&[path], CurveKind::Cubic)
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let ring = expand_stroke_with_fallback(
            &segments,
            &StrokeOptions {
                width: 2. * radius,
                ..Default::default()
            },
            CurveKind::Cubic,
        )?;
        if ring.fallback != FallbackKind::None {
            fallback = ring.fallback;
        }
        let operation = if options.weight[0] > 0. {
            BooleanOp::Union
        } else {
            BooleanOp::Exclude
        };
        let (expanded, refitted) = filled::combine(&normalized, &ring.contours, operation)?;
        if refitted {
            fallback = FallbackKind::Flattened;
        }
        result = transform(&expanded, Affine::scale_non_uniform(x / radius, y / radius));
        if let CurveKind::Quadratic { .. } = kind {
            result = crate::from_bezpaths(&crate::to_bezpaths(&result), kind);
        }
    }
    check(&result, 65535)?;
    Ok(StyleOutcome {
        contours: result,
        transform: affine,
        fallback,
    })
}
