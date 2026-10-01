//! PostScript hint invariants shared by authoring and output writers.

use crate::{Contour, Error, FlexHint, PointKind, PsHints, PsPrivate};
use kurbo::Point;

fn invalid(message: &str) -> Error {
    Error::InvalidHints(message.into())
}

impl PsPrivate {
    /// Checks Type 1/CFF private dictionary limits without changing values.
    pub fn validate(&self) -> Result<(), Error> {
        for (values, limit) in [
            (&self.blue_values, 14),
            (&self.other_blues, 10),
            (&self.family_blues, 14),
            (&self.family_other_blues, 10),
        ] {
            // The first BlueValues pair is the baseline; later pairs may occur
            // in any order (Adobe Type 1 specification, section 5.3).
            if values.len() > limit
                || !values.len().is_multiple_of(2)
                || values.iter().any(|v| !v.is_finite() || v.abs() > 32767.0)
                || values.chunks_exact(2).any(|pair| pair[0] > pair[1])
                || values.chunks_exact(2).enumerate().any(|(index, pair)| {
                    values
                        .chunks_exact(2)
                        .skip(index + 1)
                        .any(|other| pair[0] <= other[1] && other[0] <= pair[1])
                })
            {
                return Err(invalid(
                    "blue zones must contain nonoverlapping low/high pairs within format limits",
                ));
            }
        }
        for values in [&self.stem_snap_h, &self.stem_snap_v] {
            if values.len() > 12
                || values
                    .iter()
                    .any(|v| !v.is_finite() || *v <= 0.0 || *v > 32767.0)
                || values.windows(2).any(|p| p[0] >= p[1])
            {
                return Err(invalid(
                    "stem snaps must contain at most 12 increasing positive widths",
                ));
            }
        }
        for value in [self.std_hw, self.std_vw].into_iter().flatten() {
            if !value.is_finite() || !(0.0..=32767.0).contains(&value) || value == 0.0 {
                return Err(invalid("standard stem widths must be finite and positive"));
            }
        }
        if self
            .blue_scale
            .is_some_and(|v| !v.is_finite() || v <= 0.0 || v > 1.0)
            || [self.blue_shift, self.blue_fuzz]
                .into_iter()
                .flatten()
                .any(|v| !v.is_finite() || !(0.0..=32767.0).contains(&v))
            || self.language_group.is_some_and(|v| v > 1)
        {
            return Err(invalid(
                "invalid BlueScale, BlueShift, BlueFuzz or LanguageGroup",
            ));
        }
        Ok(())
    }
}

impl PsHints {
    /// Checks hint geometry, ghost conventions and masks against stored points.
    pub fn validate(&self, contours: &[Contour]) -> Result<(), Error> {
        let count = self.h_stems.len() + self.v_stems.len();
        if count > 96 || self.d_stems.len() > 96 || self.hint_masks.len() > 65_535 {
            return Err(invalid("PostScript hint count exceeds format limits"));
        }
        for stems in [&self.h_stems, &self.v_stems] {
            for stem in stems {
                let ghost = stem.width == -20.0 || stem.width == -21.0;
                if !stem.start.is_finite()
                    || stem.start.abs() > 32767.0
                    || !stem.width.is_finite()
                    || (stem.width <= 0.0 && !ghost)
                    || stem.width > 32767.0
                    || (stem.start + stem.width).abs() > 32767.0
                    || stem.ranges.len() > 65_535
                    || stem
                        .ranges
                        .iter()
                        .any(|(a, b)| !a.is_finite() || !b.is_finite() || a > b)
                {
                    return Err(invalid(
                        "invalid stem coordinate, width, ghost or active range",
                    ));
                }
            }
        }
        for stem in &self.d_stems {
            if [
                stem.left.0,
                stem.left.1,
                stem.right.0,
                stem.right.1,
                stem.unit.0,
                stem.unit.1,
            ]
            .iter()
            .any(|v| !v.is_finite())
                || (stem.unit.0.hypot(stem.unit.1) - 1.0).abs() > 0.0001
            {
                return Err(invalid(
                    "diagonal hints require finite points and a unit direction",
                ));
            }
        }
        self.validate_flex(contours)?;
        for contour in &self.dot_sections {
            if contours.get(*contour).is_none_or(|c| !c.closed)
                || self
                    .dot_sections
                    .iter()
                    .filter(|other| *other == contour)
                    .count()
                    != 1
            {
                return Err(invalid(
                    "a dot section must name one distinct closed contour",
                ));
            }
        }
        let mut references = std::collections::HashSet::new();
        for mask in &self.hint_masks {
            let point = contours
                .get(mask.contour)
                .and_then(|c| c.points.get(mask.point));
            if count == 0
                || mask.mask.len() != count.div_ceil(8)
                || point.is_none_or(|p| p.kind == PointKind::OffCurve)
                || (!mask.counter && !references.insert((mask.contour, mask.point, mask.closing)))
                || (mask.closing
                    && contours.get(mask.contour).is_none_or(|c| {
                        !c.closed
                            || c.points.iter().position(|p| p.kind != PointKind::OffCurve)
                                != Some(mask.point)
                    }))
                || (!count.is_multiple_of(8)
                    && mask
                        .mask
                        .last()
                        .is_some_and(|b| b & ((1 << (8 - count % 8)) - 1) != 0))
            {
                return Err(invalid(
                    "hint masks must reference distinct on-curve points and match the stem count",
                ));
            }
        }
        Ok(())
    }
}

/// Geometry of one resolved flex hint, in stored contour order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlexGeometry {
    /// On-curve point the first curve leaves.
    pub start: Point,
    /// The six points of the two curves: two controls, the joint, two controls, the end.
    pub points: [Point; 6],
}

impl FlexGeometry {
    /// The shared on-curve point between the two curves.
    pub fn joint(&self) -> Point {
        self.points[2]
    }

    /// The on-curve point the second curve ends at.
    pub fn end(&self) -> Point {
        self.points[5]
    }

    /// Whether the outer endpoints share a coordinate, and on which axis.
    ///
    /// Type 1 section 8.3 requires the two outer endpoints to be exactly
    /// horizontal or vertical to each other. `Some(true)` is a horizontally
    /// oriented flex (equal `y`), `Some(false)` a vertical one.
    pub fn orientation(&self) -> Option<bool> {
        if self.start.y == self.end().y {
            Some(true)
        } else if self.start.x == self.end().x {
            Some(false)
        } else {
            None
        }
    }

    /// Perpendicular depth of the joint from the line between the outer endpoints.
    pub fn height(&self) -> f64 {
        match self.orientation() {
            Some(true) => (self.joint().y - self.start.y).abs(),
            Some(false) => (self.joint().x - self.start.x).abs(),
            None => {
                let chord = self.end() - self.start;
                let length = chord.hypot();
                if length == 0.0 {
                    0.0
                } else {
                    ((self.joint() - self.start).cross(chord) / length).abs()
                }
            }
        }
    }

    /// The reference point Type 1 section 8.3 requires before the six curve points.
    ///
    /// For a horizontally oriented flex it shares the joint's `x` and the start's
    /// `y`; for a vertical one the joint's `y` and the start's `x`. A flex with no
    /// exact orientation has no specified reference point.
    pub fn reference(&self) -> Option<Point> {
        match self.orientation()? {
            true => Some(Point::new(self.joint().x, self.start.y)),
            false => Some(Point::new(self.start.x, self.joint().y)),
        }
    }
}

impl PsHints {
    /// Resolves one flex hint's seven points against stored contour geometry.
    ///
    /// Fails when the bound point is missing, is not on the outline, or is not
    /// followed by exactly two curve segments.
    pub fn flex_geometry(
        &self,
        contours: &[Contour],
        flex: &FlexHint,
    ) -> Result<FlexGeometry, Error> {
        let contour = contours
            .get(flex.contour)
            .ok_or_else(|| invalid("flex hint names a missing contour"))?;
        let count = contour.points.len();
        if count == 0 || contour.points.get(flex.point).is_none() {
            return Err(invalid("flex hint names a missing point"));
        }
        if contour.points[flex.point].kind == PointKind::OffCurve {
            return Err(invalid("a flex hint must start at an on-curve point"));
        }
        if !contour.closed && flex.point + 6 >= count {
            return Err(invalid("flex hint runs past the end of an open contour"));
        }
        let mut points = [Point::ZERO; 6];
        for (offset, slot) in points.iter_mut().enumerate() {
            let point = &contour.points[(flex.point + offset + 1) % count];
            let expected = if offset == 2 || offset == 5 {
                point.kind == PointKind::Curve
            } else {
                point.kind == PointKind::OffCurve
            };
            if !expected {
                return Err(invalid(
                    "a flex hint requires exactly two cubic curve segments",
                ));
            }
            *slot = Point::new(point.x, point.y);
        }
        let start = &contour.points[flex.point];
        Ok(FlexGeometry {
            start: Point::new(start.x, start.y),
            points,
        })
    }

    fn validate_flex(&self, contours: &[Contour]) -> Result<(), Error> {
        if self.flex_hints.len() > 64 {
            return Err(invalid("a layer may carry at most 64 flex hints"));
        }
        let mut claimed = std::collections::HashSet::new();
        for flex in &self.flex_hints {
            if !flex.depth.is_finite() || !(0.0..=1000.0).contains(&flex.depth) {
                return Err(invalid(
                    "a flex height control parameter must be between 0 and 1000",
                ));
            }
            let geometry = self.flex_geometry(contours, flex)?;
            let count = contours[flex.contour].points.len();
            // Two flex hints may not share a segment, and a mask may not fall on
            // the joint or inside the pair: the emitted operator is one unit.
            for offset in 0..6 {
                if !claimed.insert((flex.contour, (flex.point + offset) % count)) {
                    return Err(invalid("flex hints may not share a curve segment"));
                }
            }
            // A mask at the joint is emitted before the flex operator, which is
            // legal and useful. A mask from the joint onwards would have to take
            // effect inside the single operator that draws both curves, and the
            // writers consume the second segment with the first, so such a mask
            // would be declared and never activated.
            if self.hint_masks.iter().any(|mask| {
                !mask.counter
                    && mask.contour == flex.contour
                    && (4..=6).contains(&((mask.point + count - flex.point) % count))
            }) {
                return Err(invalid(
                    "a hint mask cannot fall inside a flex hint's curve pair; bind it to the joint or to a point outside the pair",
                ));
            }
            if geometry.height() > 20.0 {
                return Err(invalid(
                    "a flex hint's height must be 20 units or less (Adobe Type 1 section 8.3)",
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::PsPrivate;

    #[test]
    fn blue_zone_pair_order_is_preserved_but_each_pair_must_be_valid() {
        let mut private = PsPrivate {
            blue_values: vec![-15.0, 0.0, 700.0, 715.0, 547.0, 559.0],
            ..Default::default()
        };
        assert!(private.validate().is_ok());
        private.blue_values[4..].copy_from_slice(&[-12.0, -2.0]);
        assert!(private.validate().is_err());
        private.blue_values[4..].copy_from_slice(&[559.0, 547.0]);
        assert!(private.validate().is_err());
    }
}
