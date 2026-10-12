//! Flex hints and dot sections resolved to emitted path element indices.
//!
//! The indices refer to the same path as [`super::path_hint_masks`]: the
//! concatenation of each contour's `try_to_bezpath()` output, with components
//! appended afterwards so they never shift an index.

use std::collections::{BTreeMap, BTreeSet};

use crate::{Contour, Error, PointKind, PsHints};

/// One flex hint placed on the path element that starts its first curve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathFlex {
    /// Flex height control parameter in hundredths of a device pixel.
    pub depth: f64,
    /// Index of the second curve, which the flex operator consumes as well.
    pub second: usize,
}

/// Maps each flex hint to the first of the two path elements it replaces.
///
/// Both elements must be curves and must be adjacent, which the hint validation
/// already guarantees for stored geometry; this additionally proves it for the
/// emitted path, so a writer can consume the pair as one operator.
pub fn path_flex(
    hints: &PsHints,
    contours: &[Contour],
) -> Result<BTreeMap<usize, PathFlex>, Error> {
    let mut result = BTreeMap::new();
    if hints.flex_hints.is_empty() {
        return Ok(result);
    }
    let offsets = contour_offsets(contours)?;
    for flex in &hints.flex_hints {
        let contour = contours
            .get(flex.contour)
            .ok_or_else(|| Error::InvalidHints("flex hint names a missing contour".into()))?;
        let elements = element_indices(contour)?;
        let count = contour.points.len();
        let first = *elements.get(&((flex.point + 3) % count)).ok_or_else(|| {
            Error::InvalidHints("flex hint does not end a curve at its joint".into())
        })?;
        let second = *elements.get(&((flex.point + 6) % count)).ok_or_else(|| {
            Error::InvalidHints("flex hint does not end a curve at its end point".into())
        })?;
        let base = offsets[flex.contour];
        if second != first + 1 {
            return Err(Error::InvalidHints(
                "a flex hint requires two adjacent curve segments".into(),
            ));
        }
        for index in [first, second] {
            if !matches!(
                contour
                    .try_to_bezpath()
                    .map_err(|e| Error::InvalidHints(e.to_string()))?
                    .elements()
                    .get(index),
                Some(kurbo::PathEl::CurveTo(..) | kurbo::PathEl::QuadTo(..))
            ) {
                return Err(Error::InvalidHints(
                    "a flex hint must cover two curve segments".into(),
                ));
            }
        }
        if result
            .insert(
                base + first,
                PathFlex {
                    depth: flex.depth,
                    second: base + second,
                },
            )
            .is_some()
        {
            return Err(Error::InvalidHints(
                "two flex hints resolve to the same curve".into(),
            ));
        }
    }
    Ok(result)
}

/// Path element indices after which a Type 1 `dotsection` command belongs.
///
/// Type 1 section 8.2 brackets a dot: one command immediately after the `rmoveto`
/// that opens the subpath and one immediately after its `closepath`.
pub fn path_dot_sections(hints: &PsHints, contours: &[Contour]) -> Result<BTreeSet<usize>, Error> {
    let mut result = BTreeSet::new();
    if hints.dot_sections.is_empty() {
        return Ok(result);
    }
    let offsets = contour_offsets(contours)?;
    for index in &hints.dot_sections {
        let contour = contours
            .get(*index)
            .ok_or_else(|| Error::InvalidHints("dot section names a missing contour".into()))?;
        let length = contour
            .try_to_bezpath()
            .map_err(|e| Error::InvalidHints(e.to_string()))?
            .elements()
            .len();
        if length < 2 {
            return Err(Error::InvalidHints(
                "a dot section needs a drawn closed contour".into(),
            ));
        }
        result.insert(offsets[*index]);
        result.insert(offsets[*index] + length - 1);
    }
    Ok(result)
}

fn contour_offsets(contours: &[Contour]) -> Result<Vec<usize>, Error> {
    let mut offsets = Vec::with_capacity(contours.len());
    let mut total = 0;
    for contour in contours {
        offsets.push(total);
        total += contour
            .try_to_bezpath()
            .map_err(|e| Error::InvalidHints(e.to_string()))?
            .elements()
            .len();
    }
    Ok(offsets)
}

/// Maps a contour's stored on-curve point indices to its path element indices.
///
/// This mirrors the traversal in [`super::path_hint_masks`]: the path starts at
/// the first explicit on-curve point, and a point preceded by control points
/// advances by one element for a curve and by the number of implied segments for
/// a line, so quadratic implied points land on their own segments.
pub fn element_indices(contour: &Contour) -> Result<BTreeMap<usize, usize>, Error> {
    let start = contour
        .points
        .iter()
        .position(|point| point.kind != PointKind::OffCurve)
        .ok_or_else(|| Error::InvalidHints("contour has no explicit on-curve point".into()))?;
    let mut result = BTreeMap::from([(start, 0usize)]);
    let mut path_index = 0;
    let mut controls = 0;
    for offset in 1..contour.points.len() {
        let index = (start + offset) % contour.points.len();
        let point = &contour.points[index];
        if point.kind == PointKind::OffCurve {
            controls += 1;
            continue;
        }
        path_index += if point.kind == PointKind::Curve || controls == 0 {
            1
        } else {
            controls
        };
        result.insert(index, path_index);
        controls = 0;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FlexHint, HintMask, Point as ModelPoint, StemHint};

    fn flex_contour() -> Contour {
        // One on-curve start, two cubic segments, then a closing line.
        let point = |x: f64, y: f64, kind| ModelPoint {
            x,
            y,
            kind,
            ..ModelPoint::default()
        };
        Contour {
            closed: true,
            points: vec![
                point(100.0, -10.0, PointKind::Line),
                point(115.0, -10.0, PointKind::OffCurve),
                point(125.0, 0.0, PointKind::OffCurve),
                point(150.0, 0.0, PointKind::Curve),
                point(175.0, 0.0, PointKind::OffCurve),
                point(185.0, -10.0, PointKind::OffCurve),
                point(200.0, -10.0, PointKind::Curve),
                point(200.0, -200.0, PointKind::Line),
                point(100.0, -200.0, PointKind::Line),
            ],
            name: None,
            id: None,
        }
    }

    #[test]
    fn element_indices_agree_with_the_hint_mask_traversal() {
        let contour = flex_contour();
        let hints = PsHints {
            h_stems: vec![StemHint {
                start: 0.0,
                width: 10.0,
                ranges: Vec::new(),
            }],
            hint_masks: (0..contour.points.len())
                .filter(|index| contour.points[*index].kind != PointKind::OffCurve)
                .map(|point| HintMask {
                    counter: false,
                    closing: false,
                    contour: 0,
                    point,
                    mask: vec![0x80],
                })
                .collect(),
            ..PsHints::default()
        };
        let masks = super::super::path_hint_masks(&hints, std::slice::from_ref(&contour)).unwrap();
        let mut expected: Vec<_> = element_indices(&contour).unwrap().into_values().collect();
        expected.sort_unstable();
        let mut actual: Vec<_> = masks.into_keys().collect();
        actual.sort_unstable();
        assert_eq!(actual, expected);
    }

    #[test]
    fn a_flex_pair_resolves_to_two_adjacent_curve_elements() {
        let contour = flex_contour();
        let hints = PsHints {
            flex_hints: vec![FlexHint {
                contour: 0,
                point: 0,
                depth: 50.0,
            }],
            dot_sections: vec![0],
            ..PsHints::default()
        };
        let contours = std::slice::from_ref(&contour);
        hints.validate(contours).unwrap();
        assert_eq!(
            path_flex(&hints, contours).unwrap(),
            BTreeMap::from([(
                1,
                PathFlex {
                    depth: 50.0,
                    second: 2
                }
            )])
        );
        // The contour emits MoveTo, two curves, two lines and ClosePath.
        assert_eq!(
            path_dot_sections(&hints, contours).unwrap(),
            BTreeSet::from([0, 5])
        );
    }

    #[test]
    fn a_mask_may_bind_to_a_flex_joint_but_not_from_there_to_its_end() {
        let contour = flex_contour();
        let contours = std::slice::from_ref(&contour);
        let hints = |point: usize| PsHints {
            h_stems: vec![StemHint {
                start: 0.0,
                width: 10.0,
                ranges: Vec::new(),
            }],
            flex_hints: vec![FlexHint {
                contour: 0,
                point: 0,
                depth: 50.0,
            }],
            hint_masks: vec![HintMask {
                counter: false,
                closing: false,
                contour: 0,
                point,
                mask: vec![0x80],
            }],
            ..PsHints::default()
        };
        // The joint's mask is emitted before the operator that draws the pair.
        hints(3).validate(contours).unwrap();
        assert_eq!(
            path_flex(&hints(3), contours).unwrap(),
            BTreeMap::from([(
                1,
                PathFlex {
                    depth: 50.0,
                    second: 2
                }
            )])
        );
        // The end point's mask would have to take effect inside that operator.
        let error = hints(6).validate(contours).unwrap_err().to_string();
        assert!(error.contains("inside a flex hint's curve pair"), "{error}");
    }

    #[test]
    fn a_flex_hint_on_a_line_segment_is_rejected() {
        let contour = flex_contour();
        let hints = PsHints {
            flex_hints: vec![FlexHint {
                contour: 0,
                point: 6,
                depth: 50.0,
            }],
            ..PsHints::default()
        };
        let contours = std::slice::from_ref(&contour);
        assert!(hints.validate(contours).is_err());
        assert!(path_flex(&hints, contours).is_err());
    }
}
