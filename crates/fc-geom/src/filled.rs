//! Check the final two-input operation against a separate polygon engine.
use crate::{BooleanOp, CurveKind, Error};
use fc_model::Contour;
use i_overlay::{
    core::{fill_rule::FillRule, overlay_rule::OverlayRule},
    float::single::SingleFloatOverlay,
};
use kurbo::{flatten, PathEl, Shape};
use std::panic::{catch_unwind, AssertUnwindSafe};

type Polygons = Vec<Vec<[f64; 2]>>;
type Shapes = Vec<Polygons>;
fn invalid(message: &str) -> Error {
    Error::BooleanFailed {
        reason: message.into(),
    }
}

fn polygons(contours: &[Contour]) -> Result<Polygons, Error> {
    let mut remaining = 200_000usize;
    let mut result = Vec::new();
    for contour in contours {
        let mut polygon = Vec::new();
        let mut exceeded = false;
        flatten(contour.to_bezpath().iter(), 0.01, |element| {
            if let PathEl::MoveTo(p) | PathEl::LineTo(p) = element {
                if remaining == 0 {
                    exceeded = true;
                } else {
                    remaining -= 1;
                    polygon.push([p.x, p.y]);
                }
            }
        });
        if exceeded {
            return Err(invalid("filled polygon verification point budget exceeded"));
        }
        if polygon.len() >= 3 {
            result.push(polygon);
        }
    }
    Ok(result)
}
fn overlay(a: &Polygons, b: &Polygons, rule: OverlayRule) -> Result<Shapes, Error> {
    catch_unwind(AssertUnwindSafe(|| a.overlay(b, rule, FillRule::NonZero)))
        .map_err(|_| invalid("filled polygon verification failed"))
}
fn area(shapes: &Shapes) -> f64 {
    shapes
        .iter()
        .map(|shape| {
            shape
                .iter()
                .map(|p| {
                    p.iter()
                        .zip(p.iter().cycle().skip(1))
                        .take(p.len())
                        .map(|(a, b)| (a[0] * b[1] - a[1] * b[0]) * 0.5)
                        .sum::<f64>()
                })
                .sum::<f64>()
                .abs()
        })
        .sum()
}
fn matches(expected: &Polygons, output: &[Contour], budget: f64) -> Result<bool, Error> {
    let actual = polygons(output)?;
    Ok(area(&overlay(expected, &actual, OverlayRule::Xor)?) <= budget)
}
pub(crate) fn combine(
    a: &[Contour],
    b: &[Contour],
    op: BooleanOp,
) -> Result<(Vec<Contour>, bool), Error> {
    let left = polygons(a)?;
    let right = polygons(b)?;
    let rule = match op {
        BooleanOp::Union => OverlayRule::Union,
        BooleanOp::Exclude => OverlayRule::Difference,
        BooleanOp::Intersect => OverlayRule::Intersect,
        BooleanOp::Xor => OverlayRule::Xor,
    };
    let expected: Polygons = overlay(&left, &right, rule)?
        .into_iter()
        .flatten()
        .collect();
    // Permit at most 0.05 unit average edge displacement, measured in the
    // enlarged working grid (therefore no larger in final font coordinates).
    let perimeter: f64 = expected
        .iter()
        .flat_map(|p| p.iter().zip(p.iter().cycle().skip(1)).take(p.len()))
        .map(|(a, b)| ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt())
        .sum();
    let budget = 1. + perimeter * 0.05;
    if let Ok(result) = crate::boolean(a, b, op, CurveKind::Cubic, 0.01) {
        if matches(&expected, &result, budget)? {
            return Ok((result, false));
        }
    }
    let fitted: Vec<_> = expected
        .iter()
        .filter_map(|p| crate::boolean::fit_polygon_to_cubics(p, 0.02))
        .collect();
    let result = crate::from_bezpaths(&fitted, CurveKind::Cubic);
    let clockwise = a
        .iter()
        .chain(b)
        .map(|c| c.to_bezpath().area())
        .max_by(|x, y| x.abs().total_cmp(&y.abs()))
        .is_some_and(|area| area < 0.);
    let result = crate::correct_direction(&result, clockwise);
    if !matches(&expected, &result, budget)? {
        return Err(invalid(
            "filled curve refitting disagrees with the filled polygon result",
        ));
    }
    Ok((result, true))
}
