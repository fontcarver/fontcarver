//! Point-list contours and conversion to and from kurbo paths.

use kurbo::{BezPath, PathEl, Point as KurboPoint};
use serde::{Deserialize, Serialize};

use crate::Error;

/// Segment role of a contour point.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum PointKind {
    /// Start of an open contour.
    Move,
    /// On-curve endpoint of a straight segment.
    #[default]
    Line,
    /// On-curve endpoint of a cubic segment.
    Curve,
    /// On-curve endpoint of a quadratic segment.
    QCurve,
    /// Off-curve control point.
    OffCurve,
}

/// Original TrueType point identity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TtIndex {
    /// Point has no assigned TrueType index.
    #[default]
    Unnumbered,
    /// On-curve point is implied between consecutive off-curve points.
    Implied,
    /// Point index from the source `glyf` outline.
    Index(u16),
}

/// A point in a UFO-style point-list contour.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Point {
    /// Horizontal coordinate.
    #[serde(default)]
    pub x: f64,
    /// Vertical coordinate.
    #[serde(default)]
    pub y: f64,
    /// Segment role.
    #[serde(default)]
    pub kind: PointKind,
    /// Whether neighboring handles should remain tangent.
    #[serde(default)]
    pub smooth: bool,
    /// Live corner radius in font units: positive rounds, negative cuts an ink trap.
    /// Zero keeps the source join unchanged.
    #[serde(default, skip_serializing_if = "zero_radius")]
    #[cfg_attr(feature = "ts", ts(as = "Option<f64>", optional))]
    pub corner_radius: f64,
    /// Optional point name.
    #[serde(default)]
    pub name: Option<String>,
    /// Original TrueType point identity.
    #[serde(default)]
    pub tt_index: TtIndex,
}

impl Point {
    fn to_kurbo(&self) -> KurboPoint {
        KurboPoint::new(self.x, self.y)
    }

    fn from_kurbo(point: KurboPoint, kind: PointKind) -> Self {
        Self {
            x: point.x,
            y: point.y,
            kind,
            smooth: false,
            corner_radius: 0.,
            name: None,
            tt_index: TtIndex::Unnumbered,
        }
    }
}

fn zero_radius(radius: &f64) -> bool {
    *radius == 0.
}

/// One open or closed outline contour.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Contour {
    /// Whether the final point connects back to the first.
    #[serde(default)]
    pub closed: bool,
    /// Ordered on-curve and off-curve points.
    #[serde(default)]
    pub points: Vec<Point>,
    /// Optional contour name.
    #[serde(default)]
    pub name: Option<String>,
    /// Optional persistent contour identifier.
    #[serde(default)]
    pub id: Option<String>,
}

impl Contour {
    /// Returns whether any endpoint explicitly identifies a quadratic segment.
    pub fn is_quadratic(&self) -> bool {
        self.points
            .iter()
            .any(|point| point.kind == PointKind::QCurve)
    }

    /// Converts the point list into a kurbo path, rejecting malformed cubic segments.
    pub fn try_to_bezpath(&self) -> Result<BezPath, Error> {
        self.build_bezpath(true)
    }

    /// Converts the point list into a kurbo path.
    ///
    /// Malformed cubic segments use quadratic interpretation as a rendering fallback.
    pub fn to_bezpath(&self) -> BezPath {
        self.build_bezpath(false).unwrap_or_default()
    }

    /// Converts every subpath into a cubic point-list contour.
    pub fn from_bezpath(path: &BezPath) -> Vec<Contour> {
        let mut contours = Vec::new();
        let mut points = Vec::new();
        let mut current = None;

        for element in path.elements() {
            match *element {
                PathEl::MoveTo(point) => {
                    finish_contour(&mut contours, &mut points, false);
                    points.push(Point::from_kurbo(point, PointKind::Move));
                    current = Some(point);
                }
                PathEl::LineTo(point) => {
                    points.push(Point::from_kurbo(point, PointKind::Line));
                    current = Some(point);
                }
                PathEl::QuadTo(control, point) => {
                    if let Some(start) = current {
                        let control1 = start + (control - start) * (2.0 / 3.0);
                        let control2 = point + (control - point) * (2.0 / 3.0);
                        points.push(Point::from_kurbo(control1, PointKind::OffCurve));
                        points.push(Point::from_kurbo(control2, PointKind::OffCurve));
                        points.push(Point::from_kurbo(point, PointKind::Curve));
                        current = Some(point);
                    }
                }
                PathEl::CurveTo(control1, control2, point) => {
                    points.push(Point::from_kurbo(control1, PointKind::OffCurve));
                    points.push(Point::from_kurbo(control2, PointKind::OffCurve));
                    points.push(Point::from_kurbo(point, PointKind::Curve));
                    current = Some(point);
                }
                PathEl::ClosePath => {
                    finish_contour(&mut contours, &mut points, true);
                    current = None;
                }
            }
        }
        finish_contour(&mut contours, &mut points, false);
        contours
    }

    fn build_bezpath(&self, checked: bool) -> Result<BezPath, Error> {
        if self.closed {
            self.build_closed(checked)
        } else {
            self.build_open(checked)
        }
    }

    fn build_open(&self, checked: bool) -> Result<BezPath, Error> {
        let mut path = BezPath::new();
        let Some(first) = self.points.first() else {
            return Ok(path);
        };
        if first.kind == PointKind::OffCurve && checked {
            return Err(Error::MalformedContour);
        }
        path.move_to(first.to_kurbo());
        let mut controls = Vec::new();
        for point in &self.points[1..] {
            if point.kind == PointKind::OffCurve {
                controls.push(point.to_kurbo());
            } else {
                emit_segment(&mut path, &controls, point, checked)?;
                controls.clear();
            }
        }
        if !controls.is_empty() {
            if checked {
                return Err(Error::MalformedContour);
            }
            for control in controls {
                path.line_to(control);
            }
        }
        Ok(path)
    }

    fn build_closed(&self, checked: bool) -> Result<BezPath, Error> {
        let mut path = BezPath::new();
        let Some(start_index) = self
            .points
            .iter()
            .position(|point| point.kind != PointKind::OffCurve)
        else {
            self.build_all_off_curve(&mut path);
            return Ok(path);
        };
        let start = &self.points[start_index];
        path.move_to(start.to_kurbo());

        let mut controls = Vec::new();
        for offset in 1..self.points.len() {
            let point = &self.points[(start_index + offset) % self.points.len()];
            if point.kind == PointKind::OffCurve {
                controls.push(point.to_kurbo());
            } else {
                emit_segment(&mut path, &controls, point, checked)?;
                controls.clear();
            }
        }
        if !controls.is_empty() {
            emit_segment(&mut path, &controls, start, checked)?;
        } else if checked && start.kind == PointKind::Curve {
            return Err(Error::MalformedContour);
        }
        path.close_path();
        Ok(path)
    }

    fn build_all_off_curve(&self, path: &mut BezPath) {
        let Some(first) = self.points.first() else {
            return;
        };
        let Some(last) = self.points.last() else {
            return;
        };
        let start = midpoint(last.to_kurbo(), first.to_kurbo());
        path.move_to(start);
        for (index, control) in self.points.iter().enumerate() {
            let endpoint = if index + 1 == self.points.len() {
                start
            } else {
                midpoint(control.to_kurbo(), self.points[index + 1].to_kurbo())
            };
            path.quad_to(control.to_kurbo(), endpoint);
        }
        path.close_path();
    }
}

fn emit_segment(
    path: &mut BezPath,
    controls: &[KurboPoint],
    endpoint: &Point,
    checked: bool,
) -> Result<(), Error> {
    if endpoint.kind == PointKind::Curve {
        if controls.len() == 2 {
            path.curve_to(controls[0], controls[1], endpoint.to_kurbo());
            return Ok(());
        }
        if checked {
            return Err(Error::MalformedContour);
        }
    }
    if controls.is_empty() {
        path.line_to(endpoint.to_kurbo());
        return Ok(());
    }
    emit_quadratics(path, controls, endpoint.to_kurbo());
    Ok(())
}

fn emit_quadratics(path: &mut BezPath, controls: &[KurboPoint], endpoint: KurboPoint) {
    for (index, control) in controls.iter().enumerate() {
        let segment_end = controls
            .get(index + 1)
            .map_or(endpoint, |next| midpoint(*control, *next));
        path.quad_to(*control, segment_end);
    }
}

fn midpoint(left: KurboPoint, right: KurboPoint) -> KurboPoint {
    KurboPoint::new((left.x + right.x) * 0.5, (left.y + right.y) * 0.5)
}

fn finish_contour(contours: &mut Vec<Contour>, points: &mut Vec<Point>, closed: bool) {
    if points.is_empty() {
        return;
    }
    if closed {
        let duplicate_start = points.len() > 1
            && points.last().is_some_and(|last| {
                last.kind != PointKind::OffCurve
                    && points
                        .first()
                        .is_some_and(|first| last.x == first.x && last.y == first.y)
            });
        let closing_kind = duplicate_start
            .then(|| points.pop().map(|point| point.kind))
            .flatten();
        if let Some(first) = points.first_mut() {
            first.kind = closing_kind.unwrap_or(PointKind::Line);
        }
    }
    contours.push(Contour {
        closed,
        points: std::mem::take(points),
        name: None,
        id: None,
    });
}
