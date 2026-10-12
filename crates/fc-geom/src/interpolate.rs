use std::collections::BTreeMap;

use fc_model::{Anchor, Layer};
use kurbo::Affine;

use crate::Error;

/// One reason that two layers cannot be interpolated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Incompatibility {
    /// Contour index for a contour-local issue, or `None` for a layer-level issue.
    pub contour: Option<usize>,
    /// Human-readable explanation of the mismatch.
    pub reason: String,
}

/// Checks whether two layers have interpolation-compatible outline structure.
///
/// Smooth flags may differ. Anchors are matched by name rather than position.
/// Diagnostics stop after the first 128 mismatches to bound pathological reports.
pub fn compatible(a: &Layer, b: &Layer) -> Result<(), Vec<Incompatibility>> {
    let mut issues = Vec::new();
    if a.contours.len() != b.contours.len() {
        issues.push(layer_issue(format!(
            "contour count differs: {} versus {}",
            a.contours.len(),
            b.contours.len()
        )));
    }
    for (index, (left, right)) in a.contours.iter().zip(&b.contours).enumerate() {
        if issues.len() >= 128 {
            return Err(issues);
        }
        if left.closed != right.closed {
            issues.push(contour_issue(index, "open/closed state differs"));
        }
        if left.points.len() != right.points.len() {
            issues.push(contour_issue(
                index,
                format!(
                    "point count differs: {} versus {}",
                    left.points.len(),
                    right.points.len()
                ),
            ));
            continue;
        }
        if let Some(point_index) = left
            .points
            .iter()
            .zip(&right.points)
            .position(|(left, right)| left.kind != right.kind)
        {
            issues.push(contour_issue(
                index,
                format!("point kind differs at point {point_index}"),
            ));
        }
    }

    if a.components.len() != b.components.len() {
        issues.push(layer_issue(format!(
            "component count differs: {} versus {}",
            a.components.len(),
            b.components.len()
        )));
    }
    for (index, (left, right)) in a.components.iter().zip(&b.components).enumerate() {
        if issues.len() >= 128 {
            return Err(issues);
        }
        if left.base != right.base {
            issues.push(layer_issue(format!(
                "component base differs at index {index}: {} versus {}",
                left.base, right.base
            )));
        }
    }

    let left_anchors = anchor_counts(&a.anchors);
    let right_anchors = anchor_counts(&b.anchors);
    for (name, count) in &left_anchors {
        if issues.len() >= 128 {
            return Err(issues);
        }
        if right_anchors.get(name) != Some(count) {
            issues.push(layer_issue(format!("anchor set differs for name '{name}'")));
        }
    }
    for name in right_anchors.keys() {
        if issues.len() >= 128 {
            return Err(issues);
        }
        if !left_anchors.contains_key(name) {
            issues.push(layer_issue(format!("anchor '{name}' is unmatched")));
        }
    }

    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}

/// Interpolates compatible layer points, component transforms, named anchors, and width.
///
/// Values of `t` outside `0..=1` extrapolate. Non-geometric layer data is cloned
/// from `a`.
pub fn interpolate(a: &Layer, b: &Layer, t: f64) -> Result<Layer, Error> {
    compatible(a, b).map_err(Error::Incompatible)?;
    let mut result = a.clone();
    result.width = lerp(a.width, b.width, t);
    result.vertical_width = interpolate_optional(a.vertical_width, b.vertical_width, t);
    result.vertical_origin = interpolate_optional(a.vertical_origin, b.vertical_origin, t);

    for (output, (left, right)) in result
        .contours
        .iter_mut()
        .zip(a.contours.iter().zip(&b.contours))
    {
        for (point, (left_point, right_point)) in output
            .points
            .iter_mut()
            .zip(left.points.iter().zip(&right.points))
        {
            point.x = lerp(left_point.x, right_point.x, t);
            point.y = lerp(left_point.y, right_point.y, t);
            point.corner_radius = lerp(left_point.corner_radius, right_point.corner_radius, t);
            point.smooth = if t < 0.5 {
                left_point.smooth
            } else {
                right_point.smooth
            };
        }
    }

    for (output, (left, right)) in result
        .components
        .iter_mut()
        .zip(a.components.iter().zip(&b.components))
    {
        let left_coefficients = left.transform.as_coeffs();
        let right_coefficients = right.transform.as_coeffs();
        output.transform = Affine::new(std::array::from_fn(|index| {
            lerp(left_coefficients[index], right_coefficients[index], t)
        }));
    }

    let mut used = vec![false; b.anchors.len()];
    for output in &mut result.anchors {
        if let Some((index, right)) = b
            .anchors
            .iter()
            .enumerate()
            .find(|(index, anchor)| !used[*index] && anchor.name == output.name)
        {
            used[index] = true;
            output.x = lerp(output.x, right.x, t);
            output.y = lerp(output.y, right.y, t);
        }
    }
    Ok(result)
}

fn anchor_counts(anchors: &[Anchor]) -> BTreeMap<&str, usize> {
    let mut counts = BTreeMap::new();
    for anchor in anchors {
        *counts.entry(anchor.name.as_str()).or_default() += 1;
    }
    counts
}

fn contour_issue(index: usize, reason: impl Into<String>) -> Incompatibility {
    Incompatibility {
        contour: Some(index),
        reason: reason.into(),
    }
}

fn layer_issue(reason: impl Into<String>) -> Incompatibility {
    Incompatibility {
        contour: None,
        reason: reason.into(),
    }
}

fn lerp(left: f64, right: f64, t: f64) -> f64 {
    left + (right - left) * t
}

fn interpolate_optional(left: Option<f64>, right: Option<f64>, t: f64) -> Option<f64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(lerp(left, right, t)),
        (left, _) if t < 0.5 => left,
        (_, right) => right,
    }
}
