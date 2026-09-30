use kurbo::{BezPath, Line, ParamCurve, ParamCurveExtrema, Rect, Shape};

use crate::Error;

pub(super) const SAMPLES: usize = 256;

pub(super) struct Profile {
    pub bounds: Rect,
    pub edges: Vec<Option<(f64, f64)>>,
}

pub(super) fn invalid(message: impl Into<String>) -> Error {
    Error::InvalidCommand(message.into())
}

impl Profile {
    pub fn sample(path: &BezPath, band: (f64, f64), budget: &mut usize) -> Result<Self, Error> {
        if !path.is_finite() || path.elements().len() > 4096 {
            return Err(invalid(
                "optical outline is non-finite or exceeds 4096 segments",
            ));
        }
        let bounds = path.bounding_box();
        if [bounds.x0, bounds.y0, bounds.x1, bounds.y1]
            .into_iter()
            .any(|v| v.abs() > 1_000_000.)
            || !band.0.is_finite()
            || !band.1.is_finite()
            || band.1 <= band.0
        {
            return Err(invalid("optical outline exceeds the sampling range"));
        }
        let segments: Vec<_> = path
            .segments()
            .map(|seg| (seg, ParamCurveExtrema::bounding_box(&seg)))
            .collect();
        *budget = budget
            .checked_sub(segments.len() * SAMPLES)
            .ok_or_else(|| invalid("optical operation exceeds 16000000 segment samples"))?;
        let mut edges = Vec::with_capacity(SAMPLES);
        for row in 0..SAMPLES {
            // Midpoint samples avoid counting a shared vertex twice.
            let y = band.0 + (row as f64 + 0.5) * (band.1 - band.0) / SAMPLES as f64;
            let line = Line::new((bounds.x0 - 1., y), (bounds.x1 + 1., y));
            let mut left = f64::INFINITY;
            let mut right = f64::NEG_INFINITY;
            for (segment, bbox) in &segments {
                if y < bbox.y0 || y > bbox.y1 {
                    continue;
                }
                for hit in segment.intersect_line(line) {
                    if (0. ..=1.).contains(&hit.segment_t) && (0. ..=1.).contains(&hit.line_t) {
                        let x = segment.eval(hit.segment_t).x;
                        left = left.min(x);
                        right = right.max(x);
                    }
                }
            }
            edges.push((left.is_finite() && right > left).then_some((left, right)));
        }
        Ok(Self { bounds, edges })
    }

    pub fn sidebearings(&self, upem: f64, density: f64) -> Option<(f64, f64)> {
        let (left, right, count) = self.edges.iter().flatten().fold(
            (0., 0., 0usize),
            |(left, right, count), &(x0, x1)| {
                (
                    left + x0 - self.bounds.x0,
                    right + self.bounds.x1 - x1,
                    count + 1,
                )
            },
        );
        if count == 0 {
            return None;
        }
        let target = upem * 0.05 / density;
        let side = |recession: f64| {
            (target - 0.5 * recession / count as f64)
                .max(target * 0.1)
                .round()
        };
        Some((side(left), side(right)))
    }

    pub fn kerning(&self, right: &Self, advance: f64, upem: f64, density: f64) -> Option<f64> {
        let (sum, smallest, count) = self
            .edges
            .iter()
            .zip(&right.edges)
            .filter_map(|(a, b)| Some(advance + b.as_ref()?.0 - a.as_ref()?.1))
            .fold((0., f64::INFINITY, 0usize), |(sum, min, count), gap| {
                (sum + gap, min.min(gap), count + 1)
            });
        if count == 0 {
            return None;
        }
        let target = upem * 0.1 / density;
        let minimum = upem * 0.005 / density;
        // Rounding upward preserves the sampled clearance floor.
        Some((target - sum / count as f64).max(minimum - smallest).ceil())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kurbo::Circle;

    #[test]
    fn curved_silhouette_gets_tighter_bearings_than_same_bounds_rectangle() {
        let mut budget = 1_000_000;
        let rect = Rect::new(0., 0., 500., 500.).to_path(0.01);
        let circle = Circle::new((250., 250.), 250.).to_path(0.01);
        let block = Profile::sample(&rect, (0., 500.), &mut budget).unwrap();
        let round = Profile::sample(&circle, (0., 500.), &mut budget).unwrap();
        assert_eq!(block.sidebearings(1000., 1.), Some((50., 50.)));
        assert!(round.sidebearings(1000., 1.).unwrap().0 < 35.);
        assert!(
            round.sidebearings(1000., 2.).unwrap().0 < round.sidebearings(1000., 1.).unwrap().0
        );
    }

    #[test]
    fn pair_uses_shape_and_preserves_sampled_clearance() {
        let mut budget = 1_000_000;
        let mut triangle = BezPath::new();
        triangle.move_to((0., 0.));
        triangle.line_to((500., 0.));
        triangle.line_to((250., 700.));
        triangle.close_path();
        let left = Profile::sample(&triangle, (0., 700.), &mut budget).unwrap();
        let block = Profile::sample(
            &Rect::new(0., 0., 500., 700.).to_path(0.01),
            (0., 700.),
            &mut budget,
        )
        .unwrap();
        let kern = left.kerning(&left, 600., 1000., 1.).unwrap();
        assert!(kern < block.kerning(&block, 600., 1000., 1.).unwrap());
        for ((_, x1), (x0, _)) in left.edges.iter().flatten().zip(left.edges.iter().flatten()) {
            assert!(600. + kern + x0 - x1 >= 5.);
        }
    }

    #[test]
    fn work_budget_and_nonfinite_are_rejected() {
        let rect = Rect::new(0., 0., 500., 500.).to_path(0.01);
        assert!(Profile::sample(&rect, (0., 500.), &mut 1).is_err());
        let mut invalid = BezPath::new();
        invalid.move_to((f64::NAN, 0.));
        assert!(Profile::sample(&invalid, (0., 500.), &mut 100).is_err());
    }
}
