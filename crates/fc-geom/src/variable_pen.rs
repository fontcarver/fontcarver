//! Bounded variable-pen expansion from sampled support lines and filled patches.

use std::panic::{catch_unwind, AssertUnwindSafe};

use fc_model::{Contour, EffectCap, EffectJoin, EffectNib, EffectProgression, PenPoint};
use i_overlay::{
    core::{fill_rule::FillRule, overlay_rule::OverlayRule},
    float::single::SingleFloatOverlay,
};
use kurbo::{Affine, BezPath, ParamCurve, PathEl, Point, Shape, Vec2};

use crate::{CurveKind, Error};

const MAX_SAMPLES: usize = 8192;
const MAX_VERTICES: usize = 200_000;

#[derive(Clone, Copy)]
struct Sample {
    point: Point,
    position: f64,
    corner: bool,
}

fn invalid(message: &str) -> Error {
    Error::BooleanFailed {
        reason: message.into(),
    }
}

pub(crate) struct Options<'a> {
    pub nib: EffectNib,
    pub cap: EffectCap,
    pub join: EffectJoin,
    pub miter_limit: f64,
    pub progression: EffectProgression,
    pub points: &'a [PenPoint],
}

impl Options<'_> {
    fn pen(&self, position: f64) -> PenPoint {
        let right = self
            .points
            .partition_point(|p| p.position < position)
            .clamp(1, self.points.len() - 1);
        PenPoint::interpolate(
            &self.points[right - 1],
            &self.points[right],
            position,
            self.progression,
        )
    }
}

fn support(pen: PenPoint, nib: EffectNib, normal: Vec2) -> Vec2 {
    let rotation = Affine::rotate(pen.angle.to_radians());
    let local = rotation.inverse() * normal.to_point();
    let (a, b) = (pen.width * 0.5, pen.height * 0.5);
    let offset = match nib {
        EffectNib::Ellipse => {
            let length = (a * local.x).hypot(b * local.y);
            Vec2::new(a * a * local.x / length, b * b * local.y / length)
        }
        EffectNib::Rectangle => {
            let sign = |v: f64| if v.abs() < 1e-12 { 0. } else { v.signum() };
            Vec2::new(a * sign(local.x), b * sign(local.y))
        }
    };
    (rotation * offset.to_point()).to_vec2()
}

fn centerline(contour: &Contour) -> Result<Vec<Sample>, Error> {
    let mut samples: Vec<Sample> = Vec::new();
    let mut exceeded = false;
    for segment in contour.to_bezpath().segments() {
        if samples.is_empty() {
            samples.push(Sample {
                point: segment.start(),
                position: 0.,
                corner: true,
            });
        }
        let mut path = BezPath::new();
        path.move_to(segment.start());
        path.push(segment.as_path_el());
        kurbo::flatten(path.iter(), 0.01, |element| {
            if let PathEl::LineTo(point) = element {
                if let Some(last) = samples.last() {
                    let distance = last.point.distance(point);
                    if distance > 1e-9 {
                        if samples.len() >= MAX_SAMPLES {
                            exceeded = true;
                        } else {
                            samples.push(Sample {
                                point,
                                position: last.position + distance,
                                corner: false,
                            });
                        }
                    }
                }
            }
        });
        if let Some(last) = samples.last_mut() {
            last.corner = true;
        }
    }
    if exceeded {
        return Err(invalid("variable pen centerline exceeds 8192 samples"));
    }
    let length = samples.last().map_or(0., |s| s.position);
    if length <= 1e-9 {
        return Ok(Vec::new());
    }
    for sample in &mut samples {
        sample.position /= length;
    }
    Ok(samples)
}

fn refine(
    a: Sample,
    b: Sample,
    opts: &Options<'_>,
    output: &mut Vec<Sample>,
    depth: usize,
) -> Result<(), Error> {
    if output.len() >= MAX_SAMPLES {
        return Err(invalid("variable pen interpolation exceeds 8192 samples"));
    }
    let delta = b.point - a.point;
    if delta.hypot() <= 1e-12 {
        return Err(invalid(
            "variable pen positions are too close to resolve on this centerline",
        ));
    }
    let normal = Vec2::new(-delta.y, delta.x).normalize();
    let left = opts.pen(a.position);
    let right = opts.pen(b.position);
    let before = support(left, opts.nib, normal);
    let after = support(right, opts.nib, normal);
    let inaccurate = [0.25, 0.5, 0.75].iter().any(|&t| {
        let actual = support(
            opts.pen(a.position + (b.position - a.position) * t),
            opts.nib,
            normal,
        );
        (actual - before.lerp(after, t)).hypot() > 0.025
    }) || (left.angle - right.angle).abs() > 5.;
    if inaccurate && depth < 16 {
        let middle = Sample {
            point: a.point.lerp(b.point, 0.5),
            position: (a.position + b.position) * 0.5,
            corner: false,
        };
        refine(a, middle, opts, output, depth + 1)?;
        refine(middle, b, opts, output, depth + 1)?;
    } else {
        output.push(b);
    }
    Ok(())
}

fn samples(contour: &Contour, opts: &Options<'_>) -> Result<Vec<Sample>, Error> {
    let raw = centerline(contour)?;
    let Some(first) = raw.first() else {
        return Ok(Vec::new());
    };
    let mut result = vec![*first];
    for pair in raw.windows(2) {
        let [a, b] = [pair[0], pair[1]];
        let mut previous = a;
        for pen in opts
            .points
            .iter()
            .filter(|p| p.position > a.position && p.position < b.position)
        {
            let t = (pen.position - a.position) / (b.position - a.position);
            let next = Sample {
                point: a.point.lerp(b.point, t),
                position: pen.position,
                corner: false,
            };
            refine(previous, next, opts, &mut result, 0)?;
            previous = next;
        }
        refine(previous, b, opts, &mut result, 0)?;
    }
    Ok(result)
}

struct Patches {
    polygons: Vec<Vec<[f64; 2]>>,
    vertices: usize,
}

impl Patches {
    fn add(&mut self, points: &[Point]) -> Result<(), Error> {
        if points.iter().any(|point| !point.is_finite()) {
            return Err(invalid("variable pen produced nonfinite patch coordinates"));
        }
        self.vertices += points.len();
        if self.vertices > MAX_VERTICES {
            return Err(invalid("variable pen exceeds 200000 patch vertices"));
        }
        if points.len() < 3 {
            return Ok(());
        }
        let area: f64 = points
            .iter()
            .zip(points.iter().cycle().skip(1))
            .take(points.len())
            .map(|(a, b)| a.x * b.y - b.x * a.y)
            .sum();
        if area.abs() <= 1e-12 {
            return Ok(());
        }
        let mut polygon: Vec<_> = points.iter().map(|p| [p.x, p.y]).collect();
        if area < 0. {
            polygon.reverse();
        }
        self.polygons.push(polygon);
        Ok(())
    }
}

fn nib_arc(pen: PenPoint, nib: EffectNib, from_normal: Vec2, to_normal: Vec2) -> Vec<Vec2> {
    let rotation = Affine::rotate(pen.angle.to_radians());
    let parameter = |normal: Vec2| {
        if nib == EffectNib::Ellipse {
            let offset = rotation.inverse() * support(pen, nib, normal).to_point();
            (offset.y / pen.height).atan2(offset.x / pen.width)
        } else {
            normal.atan2()
        }
    };
    let start = parameter(from_normal);
    let sweep = (parameter(to_normal) - start).rem_euclid(std::f64::consts::TAU);
    let sweep = if sweep < 1e-9 {
        std::f64::consts::TAU
    } else {
        sweep
    };
    let radius = pen.width.max(pen.height) * 0.5;
    let step = (2. * (1. - (0.01 / radius).min(1.)).acos()).clamp(0.002, 0.2);
    let steps = (sweep / step).ceil() as usize;
    (0..=steps)
        .map(|i| {
            let angle = start + sweep * i as f64 / steps as f64;
            if nib == EffectNib::Ellipse {
                (rotation
                    * Point::new(
                        pen.width * 0.5 * angle.cos(),
                        pen.height * 0.5 * angle.sin(),
                    ))
                .to_vec2()
            } else {
                support(pen, nib, Vec2::new(angle.cos(), angle.sin()))
            }
        })
        .collect()
}

fn cap(
    patches: &mut Patches,
    sample: Sample,
    direction: Vec2,
    opts: &Options<'_>,
) -> Result<(), Error> {
    let pen = opts.pen(sample.position);
    let normal = Vec2::new(-direction.y, direction.x);
    let left = support(pen, opts.nib, normal);
    let right = -left;
    match opts.cap {
        EffectCap::Butt => Ok(()),
        EffectCap::Square => {
            let mut extension = support(pen, opts.nib, direction);
            if opts.nib == EffectNib::Rectangle {
                extension = direction * extension.dot(direction);
            }
            patches.add(&[
                sample.point + left,
                sample.point + left + extension,
                sample.point + right + extension,
                sample.point + right,
            ])
        }
        EffectCap::Round => {
            let arc = nib_arc(pen, opts.nib, -normal, normal);
            let points: Vec<_> = arc.into_iter().map(|v| sample.point + v).collect();
            patches.add(&points)
        }
    }
}

fn join(
    patches: &mut Patches,
    sample: Sample,
    incoming: Vec2,
    outgoing: Vec2,
    opts: &Options<'_>,
) -> Result<(), Error> {
    if incoming.dot(outgoing) > 1. - 1e-12 {
        return Ok(());
    }
    let pen = opts.pen(sample.position);
    let effective = if sample.corner {
        opts.join
    } else {
        EffectJoin::Bevel
    };
    if incoming.dot(outgoing) < -1. + 1e-10 && effective == EffectJoin::Round {
        let normal = Vec2::new(-incoming.y, incoming.x);
        let points: Vec<_> = nib_arc(pen, opts.nib, normal, normal)
            .into_iter()
            .map(|offset| sample.point + offset)
            .collect();
        return patches.add(&points);
    }
    for side in [-1., 1.] {
        let a = support(pen, opts.nib, Vec2::new(-incoming.y, incoming.x) * side);
        let b = support(pen, opts.nib, Vec2::new(-outgoing.y, outgoing.x) * side);
        let outer = incoming.cross(outgoing) * side < 0.;
        let mut patch = vec![sample.point, sample.point + a];
        if outer && effective == EffectJoin::Round {
            let na = Vec2::new(-incoming.y, incoming.x) * side;
            let nb = Vec2::new(-outgoing.y, outgoing.x) * side;
            let arc = if incoming.cross(outgoing) > 0. {
                nib_arc(pen, opts.nib, na, nb)
            } else {
                let mut arc = nib_arc(pen, opts.nib, nb, na);
                arc.reverse();
                arc
            };
            patch.extend(arc.into_iter().map(|v| sample.point + v));
        } else if outer && effective == EffectJoin::Miter {
            let divisor = incoming.cross(outgoing);
            if divisor.abs() > 1e-10 {
                let intersection = a + incoming * ((b - a).cross(outgoing) / divisor);
                if intersection.hypot() <= opts.miter_limit * pen.width.max(pen.height) * 0.5 {
                    patch.push(sample.point + intersection);
                }
            }
        }
        patch.push(sample.point + b);
        patches.add(&patch)?;
    }
    Ok(())
}

pub(crate) fn expand(contours: &[Contour], opts: &Options<'_>) -> Result<Vec<Contour>, Error> {
    let mut patches = Patches {
        polygons: Vec::new(),
        vertices: 0,
    };
    for contour in contours {
        if contour.closed {
            let a = opts.points[0];
            let b = opts.points[opts.points.len() - 1];
            if a.width != b.width
                || a.height != b.height
                || (a.angle - b.angle)
                    .rem_euclid(360.)
                    .min((b.angle - a.angle).rem_euclid(360.))
                    > 1e-9
            {
                return Err(invalid("closed variable pen contours require matching first and last pen dimensions and angle"));
            }
        }
        let samples = samples(contour, opts)?;
        if samples.len() < 2 {
            continue;
        }
        let directions: Vec<_> = samples
            .windows(2)
            .map(|s| (s[1].point - s[0].point).normalize())
            .collect();
        for (pair, direction) in samples.windows(2).zip(&directions) {
            let normal = Vec2::new(-direction.y, direction.x);
            let a = support(opts.pen(pair[0].position), opts.nib, normal);
            let b = support(opts.pen(pair[1].position), opts.nib, normal);
            patches.add(&[
                pair[0].point + a,
                pair[1].point + b,
                pair[1].point - b,
                pair[0].point - a,
            ])?;
        }
        for index in 1..samples.len() - 1 {
            join(
                &mut patches,
                samples[index],
                directions[index - 1],
                directions[index],
                opts,
            )?;
        }
        let last = samples.len() - 1;
        if contour.closed {
            join(
                &mut patches,
                samples[0],
                directions[last - 1],
                directions[0],
                opts,
            )?;
        } else {
            cap(&mut patches, samples[0], -directions[0], opts)?;
            cap(&mut patches, samples[last], directions[last - 1], opts)?;
        }
    }
    let empty: Vec<Vec<[f64; 2]>> = Vec::new();
    let shapes = catch_unwind(AssertUnwindSafe(|| {
        patches
            .polygons
            .overlay(&empty, OverlayRule::Union, FillRule::NonZero)
    }))
    .map_err(|_| invalid("variable pen patch union failed"))?;
    let mut paths = Vec::new();
    for polygon in shapes.into_iter().flatten() {
        paths.push(checked_fit(&polygon)?);
    }
    Ok(crate::from_bezpaths(&paths, CurveKind::Cubic))
}

// A curve fit can overshoot substantially at a shallow transition from a
// tapered edge into a round cap. Keep the sampled boundary as the oracle and
// retain that boundary if the fitter cannot preserve its bounds and fill.
fn checked_fit(polygon: &[[f64; 2]]) -> Result<BezPath, Error> {
    let Some(first) = polygon.first() else {
        return Err(invalid("variable pen union produced an empty contour"));
    };
    let mut boundary = BezPath::new();
    boundary.move_to((first[0], first[1]));
    for point in &polygon[1..] {
        boundary.line_to((point[0], point[1]));
    }
    boundary.close_path();
    let Some(fitted) = crate::boolean::fit_polygon_to_cubics(polygon, 0.02) else {
        return Ok(boundary);
    };
    if !fitted.is_finite() {
        return Ok(boundary);
    }
    let expected = boundary.bounding_box();
    let actual = fitted.bounding_box();
    if [
        (expected.x0 - actual.x0).abs(),
        (expected.y0 - actual.y0).abs(),
        (expected.x1 - actual.x1).abs(),
        (expected.y1 - actual.y1).abs(),
    ]
    .into_iter()
    .any(|difference| difference > 0.05)
    {
        return Ok(boundary);
    }
    let mut flattened = Vec::new();
    let mut exceeded = false;
    kurbo::flatten(fitted.iter(), 0.01, |element| {
        if let PathEl::MoveTo(point) | PathEl::LineTo(point) = element {
            if flattened.len() == MAX_VERTICES {
                exceeded = true;
            } else {
                flattened.push([point.x, point.y]);
            }
        }
    });
    if exceeded {
        return Ok(boundary);
    }
    let original = vec![polygon.to_vec()];
    let candidate = vec![flattened];
    let difference = catch_unwind(AssertUnwindSafe(|| {
        original.overlay(&candidate, OverlayRule::Xor, FillRule::NonZero)
    }))
    .map_err(|_| invalid("variable pen curve-fit verification failed"))?;
    let area: f64 = difference
        .iter()
        .map(|shape| {
            shape
                .iter()
                .map(|ring| {
                    ring.iter()
                        .zip(ring.iter().cycle().skip(1))
                        .take(ring.len())
                        .map(|(a, b)| (a[0] * b[1] - a[1] * b[0]) * 0.5)
                        .sum::<f64>()
                })
                .sum::<f64>()
                .abs()
        })
        .sum();
    let perimeter: f64 = polygon
        .iter()
        .zip(polygon.iter().cycle().skip(1))
        .take(polygon.len())
        .map(|(a, b)| (a[0] - b[0]).hypot(a[1] - b[1]))
        .sum();
    Ok(if area <= perimeter * 0.05 {
        fitted
    } else {
        boundary
    })
}
