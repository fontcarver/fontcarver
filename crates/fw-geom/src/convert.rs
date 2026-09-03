use fw_model::{Contour, Point as ModelPoint, PointKind, TtIndex};
use kurbo::{cubics_to_quadratic_splines, BezPath, CubicBez, PathEl, Point, QuadBez};

/// Curve representation requested for model contours returned by an operation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CurveKind {
    /// Return cubic Bézier contours.
    Cubic,
    /// Approximate cubic segments with quadratic splines.
    Quadratic {
        /// Maximum permitted cubic-to-quadratic approximation error.
        max_err: f64,
    },
}

/// Converts model contours to kurbo paths, exactly degree-elevating quadratics.
pub fn to_bezpaths(contours: &[Contour]) -> Vec<BezPath> {
    contours
        .iter()
        .map(|contour| elevate_quadratics(&contour.to_bezpath()))
        .collect()
}

/// Converts kurbo paths to cubic or implied-point-free quadratic model contours.
///
/// Generated points have [`TtIndex::Unnumbered`] because these paths no longer
/// have a stable relationship to source TrueType point indices.
/// A non-finite or non-positive quadratic error is rejected with an empty result.
pub fn from_bezpaths(paths: &[BezPath], kind: CurveKind) -> Vec<Contour> {
    if let CurveKind::Quadratic { max_err } = kind {
        if !max_err.is_finite() || max_err <= 0.0 {
            tracing::warn!(
                max_err,
                error = %crate::Error::InvalidParameter { parameter: "quadratic approximation error" },
                "path conversion rejected an invalid parameter"
            );
            return Vec::new();
        }
    }
    paths
        .iter()
        .flat_map(|path| match kind {
            CurveKind::Cubic => path_to_contours(path),
            CurveKind::Quadratic { max_err } => {
                path_to_contours(&quadratic_approximation(path, max_err))
            }
        })
        .collect()
}

pub(crate) fn path_to_contours(path: &BezPath) -> Vec<Contour> {
    let mut contours = Vec::new();
    let mut points = Vec::new();
    let mut start = None;
    let mut current = None;

    for element in path.elements() {
        match *element {
            PathEl::MoveTo(point) => {
                finish(&mut contours, &mut points, false, start);
                start = Some(point);
                current = Some(point);
                points.push(model_point(point, PointKind::Move));
            }
            PathEl::LineTo(point) => {
                if current.is_some() {
                    points.push(model_point(point, PointKind::Line));
                    current = Some(point);
                }
            }
            PathEl::QuadTo(control, point) => {
                if current.is_some() {
                    points.push(model_point(control, PointKind::OffCurve));
                    points.push(model_point(point, PointKind::QCurve));
                    current = Some(point);
                }
            }
            PathEl::CurveTo(control1, control2, point) => {
                if current.is_some() {
                    points.push(model_point(control1, PointKind::OffCurve));
                    points.push(model_point(control2, PointKind::OffCurve));
                    points.push(model_point(point, PointKind::Curve));
                    current = Some(point);
                }
            }
            PathEl::ClosePath => {
                finish(&mut contours, &mut points, true, start);
                start = None;
                current = None;
            }
        }
    }
    finish(&mut contours, &mut points, false, start);
    contours
}

fn elevate_quadratics(path: &BezPath) -> BezPath {
    let mut elevated = BezPath::with_capacity(path.elements().len());
    let mut current = None;
    for element in path.elements() {
        match *element {
            PathEl::MoveTo(point) => {
                elevated.move_to(point);
                current = Some(point);
            }
            PathEl::LineTo(point) => {
                elevated.line_to(point);
                current = Some(point);
            }
            PathEl::QuadTo(control, point) => {
                if let Some(from) = current {
                    let cubic = QuadBez::new(from, control, point).raise();
                    elevated.curve_to(cubic.p1, cubic.p2, cubic.p3);
                    current = Some(point);
                }
            }
            PathEl::CurveTo(control1, control2, point) => {
                elevated.curve_to(control1, control2, point);
                current = Some(point);
            }
            PathEl::ClosePath => elevated.close_path(),
        }
    }
    elevated
}

fn quadratic_approximation(path: &BezPath, accuracy: f64) -> BezPath {
    let cubics = cubic_elements(path);
    let approximations = cubics_to_quadratic_splines(&cubics, accuracy);
    let mut result = BezPath::with_capacity(path.elements().len());
    let mut current = None;
    let mut cubic_index = 0;
    for element in path.elements() {
        match *element {
            PathEl::MoveTo(point) => {
                result.move_to(point);
                current = Some(point);
            }
            PathEl::LineTo(point) => {
                result.line_to(point);
                current = Some(point);
            }
            PathEl::QuadTo(control, point) => {
                result.quad_to(control, point);
                current = Some(point);
            }
            PathEl::CurveTo(control1, control2, point) => {
                if let Some(from) = current {
                    if let Some(spline) = approximations
                        .as_ref()
                        .and_then(|splines| splines.get(cubic_index))
                    {
                        for quad in spline.to_quads() {
                            result.quad_to(quad.p1, quad.p2);
                        }
                    } else {
                        append_cubic_as_quadratics(
                            &mut result,
                            CubicBez::new(from, control1, control2, point),
                            accuracy,
                        );
                    }
                    cubic_index += 1;
                    current = Some(point);
                }
            }
            PathEl::ClosePath => result.close_path(),
        }
    }
    result
}

fn cubic_elements(path: &BezPath) -> Vec<CubicBez> {
    let mut current = None;
    let mut cubics = Vec::new();
    for element in path.elements() {
        match *element {
            PathEl::MoveTo(point) | PathEl::LineTo(point) => current = Some(point),
            PathEl::QuadTo(_, point) => current = Some(point),
            PathEl::CurveTo(control1, control2, point) => {
                if let Some(from) = current {
                    cubics.push(CubicBez::new(from, control1, control2, point));
                }
                current = Some(point);
            }
            PathEl::ClosePath => {}
        }
    }
    cubics
}

fn append_cubic_as_quadratics(result: &mut BezPath, cubic: CubicBez, accuracy: f64) {
    if let Some(splines) = cubics_to_quadratic_splines(&[cubic], accuracy) {
        if let Some(spline) = splines.first() {
            for quad in spline.to_quads() {
                result.quad_to(quad.p1, quad.p2);
            }
            return;
        }
    }
    for (_, _, quad) in cubic.to_quads(accuracy) {
        result.quad_to(quad.p1, quad.p2);
    }
}

fn finish(
    contours: &mut Vec<Contour>,
    points: &mut Vec<ModelPoint>,
    closed: bool,
    start: Option<Point>,
) {
    if points.is_empty() {
        return;
    }
    if closed {
        let duplicate = start.is_some_and(|start_point| {
            points.last().is_some_and(|last| {
                last.kind != PointKind::OffCurve
                    && last.x == start_point.x
                    && last.y == start_point.y
                    && points.len() > 1
            })
        });
        let closing_kind = if duplicate {
            points.pop().map(|point| point.kind)
        } else {
            Some(PointKind::Line)
        };
        if let (Some(first), Some(kind)) = (points.first_mut(), closing_kind) {
            first.kind = kind;
        }
    }
    contours.push(Contour {
        closed,
        points: std::mem::take(points),
        name: None,
        id: None,
    });
}

fn model_point(point: Point, kind: PointKind) -> ModelPoint {
    ModelPoint {
        x: point.x,
        y: point.y,
        kind,
        smooth: false,
        name: None,
        tt_index: TtIndex::Unnumbered,
    }
}
