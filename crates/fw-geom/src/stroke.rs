use fw_model::{Contour, Point as ModelPoint, PointKind, TtIndex};
use kurbo::{flatten, stroke, BezPath, PathEl, Point, Stroke, StrokeOpts, Vec2};

use crate::{
    from_bezpaths, remove_overlap, to_bezpaths, Cap, CurveKind, Error, Join,
    DEFAULT_BOOLEAN_TOLERANCE,
};

/// Shape of the pen used to expand a centerline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Nib {
    /// Circular pen whose diameter is [`StrokeOptions::width`].
    Round,
    /// Rotated rectangular calligraphic pen.
    Calligraphic {
        /// Pen width before rotation.
        width: f64,
        /// Pen height before rotation.
        height: f64,
        /// Counter-clockwise rotation in radians.
        angle: f64,
    },
    /// Axis-aligned square pen whose side is [`StrokeOptions::width`].
    Rect,
}

/// Options for expanding centerline contours into filled outlines.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeOptions {
    /// Diameter of a round nib and side length of a rectangular nib.
    pub width: f64,
    /// Pen shape.
    pub nib: Nib,
    /// End-cap style.
    pub cap: Cap,
    /// Segment-join style.
    pub join: Join,
    /// Miter length limit as a multiple of stroke width.
    pub miter_limit: f64,
    /// Remove self-overlapping edges on the inside of bends.
    pub remove_internal: bool,
    /// Remove self-overlapping edges on the outside of bends.
    pub remove_external: bool,
}

impl Default for StrokeOptions {
    fn default() -> Self {
        Self {
            width: 1.0,
            nib: Nib::Round,
            cap: Cap::Round,
            join: Join::Round,
            miter_limit: 4.0,
            remove_internal: true,
            remove_external: true,
        }
    }
}

/// Expands centerline contours into closed filled outlines.
///
/// Round nibs use kurbo's offset-based stroker. `Rect` and `Calligraphic`
/// nibs use a convex-nib tracer: the source is flattened, the nib is swept over
/// each edge, and the resulting convex cells are unioned by the curve boolean
/// engine. `flo_curves::stroke_path` is circular-only, so it cannot represent
/// these two nib shapes directly.
pub fn expand_stroke(
    contours: &[Contour],
    opts: &StrokeOptions,
    kind: CurveKind,
) -> Result<Vec<Contour>, Error> {
    validate_options(opts)?;
    match opts.nib {
        Nib::Round => round_stroke(contours, opts, kind),
        Nib::Rect => {
            let half = opts.width * 0.5;
            convex_nib_stroke(contours, rectangle_corners(half, half, 0.0), kind)
        }
        Nib::Calligraphic {
            width,
            height,
            angle,
        } => convex_nib_stroke(
            contours,
            rectangle_corners(width * 0.5, height * 0.5, angle),
            kind,
        ),
    }
}

fn validate_options(opts: &StrokeOptions) -> Result<(), Error> {
    if !opts.width.is_finite() || opts.width <= 0.0 {
        return Err(Error::InvalidParameter {
            parameter: "stroke width",
        });
    }
    if !opts.miter_limit.is_finite() || opts.miter_limit <= 0.0 {
        return Err(Error::InvalidParameter {
            parameter: "miter limit",
        });
    }
    if let Nib::Calligraphic {
        width,
        height,
        angle,
    } = opts.nib
    {
        if !width.is_finite() || width <= 0.0 || !height.is_finite() || height <= 0.0 {
            return Err(Error::InvalidParameter {
                parameter: "calligraphic nib dimensions",
            });
        }
        if !angle.is_finite() {
            return Err(Error::InvalidParameter {
                parameter: "calligraphic nib angle",
            });
        }
    }
    Ok(())
}

fn round_stroke(
    contours: &[Contour],
    opts: &StrokeOptions,
    kind: CurveKind,
) -> Result<Vec<Contour>, Error> {
    let style = Stroke::new(opts.width)
        .with_join(opts.join)
        .with_miter_limit(opts.miter_limit)
        .with_caps(opts.cap);
    let paths: Vec<BezPath> = to_bezpaths(contours)
        .into_iter()
        .map(|path| {
            stroke(
                path.iter(),
                &style,
                &StrokeOpts::default(),
                DEFAULT_BOOLEAN_TOLERANCE,
            )
        })
        .collect();
    let expanded = from_bezpaths(&paths, CurveKind::Cubic);
    if opts.remove_internal || opts.remove_external {
        remove_overlap(&expanded, kind, DEFAULT_BOOLEAN_TOLERANCE)
    } else {
        Ok(from_bezpaths(&paths, kind))
    }
}

fn convex_nib_stroke(
    contours: &[Contour],
    corners: [Vec2; 4],
    kind: CurveKind,
) -> Result<Vec<Contour>, Error> {
    let mut cells = Vec::new();
    for path in to_bezpaths(contours) {
        let mut flattened = Vec::new();
        let closed = path
            .elements()
            .iter()
            .any(|element| matches!(element, PathEl::ClosePath));
        flatten(path.iter(), 0.05, |element| {
            if let PathEl::MoveTo(point) | PathEl::LineTo(point) = element {
                flattened.push(point);
            }
        });
        if closed {
            if let (Some(first), Some(last)) =
                (flattened.first().copied(), flattened.last().copied())
            {
                if first != last {
                    flattened.push(first);
                }
            }
        }
        for edge in flattened.windows(2) {
            if let [start, end] = edge {
                if (*end - *start).hypot() > 1e-9 {
                    cells.push(swept_cell(*start, *end, &corners));
                }
            }
        }
    }
    remove_overlap(&cells, kind, DEFAULT_BOOLEAN_TOLERANCE)
}

fn rectangle_corners(half_width: f64, half_height: f64, angle: f64) -> [Vec2; 4] {
    let (sin, cos) = angle.sin_cos();
    let rotate = |x: f64, y: f64| Vec2::new(x * cos - y * sin, x * sin + y * cos);
    [
        rotate(-half_width, -half_height),
        rotate(half_width, -half_height),
        rotate(half_width, half_height),
        rotate(-half_width, half_height),
    ]
}

fn swept_cell(start: Point, end: Point, corners: &[Vec2; 4]) -> Contour {
    let mut candidates = Vec::with_capacity(8);
    for &corner in corners {
        candidates.push(start + corner);
        candidates.push(end + corner);
    }
    let hull = convex_hull(candidates);
    Contour {
        closed: true,
        points: hull
            .into_iter()
            .map(|point| ModelPoint {
                x: point.x,
                y: point.y,
                kind: PointKind::Line,
                smooth: false,
                name: None,
                tt_index: TtIndex::Unnumbered,
            })
            .collect(),
        name: None,
        id: None,
    }
}

fn convex_hull(mut points: Vec<Point>) -> Vec<Point> {
    points.sort_by(|left, right| left.x.total_cmp(&right.x).then(left.y.total_cmp(&right.y)));
    points.dedup_by(|left, right| left.x == right.x && left.y == right.y);
    if points.len() <= 2 {
        return points;
    }
    let mut lower = Vec::new();
    for &point in &points {
        while lower.len() >= 2
            && cross(lower[lower.len() - 2], lower[lower.len() - 1], point) <= 0.0
        {
            lower.pop();
        }
        lower.push(point);
    }
    let mut upper = Vec::new();
    for &point in points.iter().rev() {
        while upper.len() >= 2
            && cross(upper[upper.len() - 2], upper[upper.len() - 1], point) <= 0.0
        {
            upper.pop();
        }
        upper.push(point);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

fn cross(a: Point, b: Point, c: Point) -> f64 {
    (b - a).cross(c - a)
}
