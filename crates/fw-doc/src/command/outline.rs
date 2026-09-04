use std::collections::BTreeSet;

use fw_model::{
    query::{declared_curve_format, CurveFormat},
    Anchor, Component, Contour, Font, GlyphName, Layer, LayerId, LayerKind, MasterId, Point,
    PointKind,
};

use crate::Error;

use super::{apply::layer_mut, Command, CurveKind, PointRef};

mod delete;
mod points;
mod segments;

/// The contours an outline operation may have changed.
///
/// Indexes address the layer's contour list *after* the operation ran, so an
/// operation that adds or removes contours reports where its results ended up.
pub(super) enum Touched {
    /// Only these contours were written.
    Contours(BTreeSet<usize>),
    /// The whole contour list was rebuilt.
    Layer,
}

impl Touched {
    /// Nothing that could be malformed was written.
    fn none() -> Self {
        Self::Contours(BTreeSet::new())
    }

    /// One contour, addressed by index.
    fn one(index: usize) -> Self {
        Self::Contours(BTreeSet::from([index]))
    }

    /// Every contour the listed points belong to.
    fn of_points(points: &[PointRef]) -> Self {
        Self::Contours(points.iter().map(|point| point.contour).collect())
    }
}

/// The layer state an outline command may overwrite, captured before it runs.
enum Restore {
    /// Individual contours, components and anchors at stable indexes.
    ///
    /// Only commands that rewrite points in place — never adding, removing or
    /// reordering a contour, component or anchor — are captured this way.
    Parts {
        /// Captured contours and where they sit.
        contours: Vec<(usize, Contour)>,
        /// Captured components and where they sit.
        components: Vec<(usize, Component)>,
        /// Captured anchors and where they sit.
        anchors: Vec<(usize, Anchor)>,
    },
    /// The whole layer, for commands that restructure or rebuild it.
    Layer(Box<Layer>),
}

impl Restore {
    /// Captures everything `command` is able to overwrite in `target`.
    fn capture(target: &Layer, command: &Command) -> Self {
        match command {
            Command::MovePoints { points, .. } | Command::SetSmooth { points, .. } => {
                Self::contours(target, points.iter().map(|point| point.contour))
            }
            Command::SetPointPosition { point, .. } | Command::SetStartPoint { point, .. } => {
                Self::contours(target, [point.contour])
            }
            Command::InsertPointOnSegment { contour, .. }
            | Command::ConvertSegment { contour, .. } => Self::contours(target, [*contour]),
            Command::ReverseContours { contours, .. } => {
                Self::contours(target, contours.iter().copied())
            }
            Command::TransformSelection {
                points,
                components,
                anchors,
                ..
            } if !(points.is_empty() && components.is_empty() && anchors.is_empty()) => {
                Self::Parts {
                    contours: pick(&target.contours, points.iter().map(|point| point.contour)),
                    components: pick(&target.components, components.iter().copied()),
                    anchors: pick(&target.anchors, anchors.iter().copied()),
                }
            }
            _ => Self::Layer(Box::new(target.clone())),
        }
    }

    /// Captures only the listed contours.
    fn contours(target: &Layer, indices: impl IntoIterator<Item = usize>) -> Self {
        Self::Parts {
            contours: pick(&target.contours, indices),
            components: Vec::new(),
            anchors: Vec::new(),
        }
    }

    /// Puts the captured state back.
    fn apply_to(self, target: &mut Layer) {
        match self {
            Self::Layer(layer) => *target = *layer,
            Self::Parts {
                contours,
                components,
                anchors,
            } => {
                put_back(&mut target.contours, contours);
                put_back(&mut target.components, components);
                put_back(&mut target.anchors, anchors);
            }
        }
    }
}

/// Clones the values at the listed indexes, skipping indexes out of range.
fn pick<T: Clone>(values: &[T], indices: impl IntoIterator<Item = usize>) -> Vec<(usize, T)> {
    let mut chosen: Vec<usize> = indices.into_iter().collect();
    chosen.sort_unstable();
    chosen.dedup();
    chosen
        .into_iter()
        .filter_map(|index| values.get(index).map(|value| (index, value.clone())))
        .collect()
}

/// Writes captured values back to the indexes they came from.
fn put_back<T>(values: &mut [T], captured: Vec<(usize, T)>) {
    for (index, value) in captured {
        if let Some(slot) = values.get_mut(index) {
            *slot = value;
        }
    }
}

/// Applies one outline command and rejects any malformed result.
///
/// Whatever the command can overwrite is captured before the operation and put
/// back when the operation fails or leaves one of the contours it touched in a
/// state that `Contour::try_to_bezpath` rejects, so a failed outline command
/// never changes the document. A command that only moves existing points — the
/// per-frame commands of a drag — captures just the contours it names, which
/// keeps a preview frame proportional to the selection rather than to the
/// glyph; a command that restructures the layer captures the whole layer, so a
/// rejected `TransformSelection` also restores component transforms and
/// anchors.
///
/// Only the contours an operation reports touching are checked. A layer may
/// already hold a contour the model tolerates but the path builder rejects (an
/// import can produce one), and such a contour must not block editing of the
/// contours around it.
pub(super) fn apply(font: &mut Font, command: &Command) -> Result<(), Error> {
    let Some((glyph, layer)) = target(command) else {
        return Err(Error::Internal(
            "non-outline command routed to outline handler".to_owned(),
        ));
    };
    let restore = Restore::capture(layer_mut(font, glyph, layer)?, command);
    let outcome =
        dispatch(font, command).and_then(|touched| validate(font, glyph, layer, &touched));
    if outcome.is_err() {
        if let Ok(target) = layer_mut(font, glyph, layer) {
            restore.apply_to(target);
        }
    }
    outcome
}

fn validate(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    touched: &Touched,
) -> Result<(), Error> {
    let target = layer_mut(font, glyph, layer)?;
    let malformed = match touched {
        Touched::Layer => target
            .contours
            .iter()
            .find_map(|contour| contour.try_to_bezpath().err()),
        Touched::Contours(indices) => indices
            .iter()
            .filter_map(|&index| target.contours.get(index))
            .find_map(|contour| contour.try_to_bezpath().err()),
    };
    match malformed {
        Some(error) => Err(Error::InvalidCommand(format!(
            "the command would leave a malformed contour: {error}"
        ))),
        None => Ok(()),
    }
}

fn target(command: &Command) -> Option<(&GlyphName, &LayerId)> {
    match command {
        Command::MovePoints { glyph, layer, .. }
        | Command::SetPointPosition { glyph, layer, .. }
        | Command::SetSmooth { glyph, layer, .. }
        | Command::InsertPointOnSegment { glyph, layer, .. }
        | Command::DeletePoints { glyph, layer, .. }
        | Command::ConvertSegment { glyph, layer, .. }
        | Command::AddContour { glyph, layer, .. }
        | Command::DeleteContours { glyph, layer, .. }
        | Command::ReverseContours { glyph, layer, .. }
        | Command::SetStartPoint { glyph, layer, .. }
        | Command::JoinContours { glyph, layer, .. }
        | Command::SplitContour { glyph, layer, .. }
        | Command::TransformSelection { glyph, layer, .. }
        | Command::PathOp { glyph, layer, .. } => Some((glyph, layer)),
        _ => None,
    }
}

fn dispatch(font: &mut Font, command: &Command) -> Result<Touched, Error> {
    match command {
        Command::MovePoints {
            glyph,
            layer,
            points,
            delta,
            handles,
        } => points::move_points(font, glyph, layer, points, *delta, *handles),
        Command::SetPointPosition {
            glyph,
            layer,
            point,
            x,
            y,
        } => points::set_position(font, glyph, layer, *point, *x, *y),
        Command::SetSmooth {
            glyph,
            layer,
            points,
            smooth,
            level_handles,
        } => points::set_smooth(font, glyph, layer, points, *smooth, *level_handles),
        Command::TransformSelection {
            glyph,
            layer,
            points,
            components,
            anchors,
            affine,
        } => points::transform_selection(font, glyph, layer, points, components, anchors, *affine),
        Command::InsertPointOnSegment {
            glyph,
            layer,
            contour,
            segment,
            t,
        } => segments::insert(font, glyph, layer, *contour, *segment, *t),
        Command::ConvertSegment {
            glyph,
            layer,
            contour,
            segment,
            to,
        } => segments::convert(font, glyph, layer, *contour, *segment, *to),
        Command::AddContour {
            glyph,
            layer,
            contour,
        } => segments::add_contour(font, glyph, layer, contour),
        Command::DeleteContours {
            glyph,
            layer,
            contours,
        } => segments::delete_contours(font, glyph, layer, contours),
        Command::ReverseContours {
            glyph,
            layer,
            contours,
        } => segments::reverse_contours(font, glyph, layer, contours),
        Command::SetStartPoint {
            glyph,
            layer,
            point,
        } => segments::set_start(font, glyph, layer, *point),
        Command::JoinContours { glyph, layer, a, b } => segments::join(font, glyph, layer, *a, *b),
        Command::SplitContour {
            glyph,
            layer,
            point,
        } => segments::split(font, glyph, layer, *point),
        Command::PathOp { glyph, layer, op } => segments::path_op(font, glyph, layer, op),
        Command::DeletePoints {
            glyph,
            layer,
            points,
            mode,
        } => delete::delete_points(font, glyph, layer, points, *mode),
        _ => Err(Error::Internal(
            "non-outline command routed to outline handler".to_owned(),
        )),
    }
}

fn validate_points(contours: &[Contour], points: &[PointRef]) -> Result<(), Error> {
    for point in points {
        let contour = contours
            .get(point.contour)
            .ok_or_else(|| index_error("contour", point.contour, contours.len()))?;
        if point.point >= contour.points.len() {
            return Err(index_error("point", point.point, contour.points.len()));
        }
    }
    Ok(())
}

/// Maximum cubic-to-quadratic approximation error used for generated handles.
const QUADRATIC_MAX_ERR: f64 = 0.1;

/// The curve kind an operation on one layer should produce.
///
/// The font's own declaration wins: a TrueType (`glyf`) source never receives
/// cubic handles, and a PostScript source never receives quadratic ones, however
/// the layer at hand happens to be drawn. Only a document that declares nothing
/// — one drawn from scratch — falls back to [`inferred_kind`], which reads the
/// layer's existing outlines.
fn layer_curve_kind(font: &Font, glyph: &GlyphName, layer: &LayerId) -> CurveKind {
    let target = font.glyph(glyph).and_then(|glyph| glyph.layer(layer));
    let master = target.and_then(layer_master);
    match declared_curve_format(font, master.as_ref()) {
        Some(CurveFormat::Quadratic) => CurveKind::Quadratic {
            max_err: QUADRATIC_MAX_ERR,
        },
        Some(CurveFormat::Cubic) => CurveKind::Cubic,
        None => inferred_kind(target.map_or(&[], |layer| layer.contours.as_slice())),
    }
}

/// The master a layer belongs to, when it belongs to one.
fn layer_master(layer: &Layer) -> Option<MasterId> {
    match &layer.kind {
        LayerKind::Master(master)
        | LayerKind::Background(master)
        | LayerKind::Brace { master, .. }
        | LayerKind::Bracket { master, .. } => Some(master.clone()),
        LayerKind::Backup { of, .. } => Some(of.clone()),
        LayerKind::Custom(_) => None,
    }
}

/// The curve kind a layer's own outlines suggest.
fn inferred_kind(contours: &[Contour]) -> CurveKind {
    if contours.iter().any(Contour::is_quadratic) {
        CurveKind::Quadratic {
            max_err: QUADRATIC_MAX_ERR,
        }
    } else {
        CurveKind::Cubic
    }
}

fn adjacent_indices(contour: &Contour, index: usize) -> Vec<usize> {
    let len = contour.points.len();
    if len < 2 {
        return Vec::new();
    }
    let mut result = Vec::new();
    if index > 0 {
        result.push(index - 1);
    } else if contour.closed {
        result.push(len - 1);
    }
    if index + 1 < len {
        result.push(index + 1);
    } else if contour.closed {
        result.push(0);
    }
    result
}

fn on_curve_indices(contour: &Contour) -> Vec<usize> {
    contour
        .points
        .iter()
        .enumerate()
        .filter(|(_, point)| point.kind != PointKind::OffCurve)
        .map(|(index, _)| index)
        .collect()
}

fn previous_on_curve(contour: &Contour, index: usize) -> Option<usize> {
    (1..contour.points.len())
        .map(|offset| (index + contour.points.len() - offset) % contour.points.len())
        .take_while(|&candidate| contour.closed || candidate < index)
        .find(|&candidate| contour.points[candidate].kind != PointKind::OffCurve)
}

fn next_on_curve(contour: &Contour, index: usize) -> Option<usize> {
    (1..contour.points.len())
        .map(|offset| (index + offset) % contour.points.len())
        .take_while(|&candidate| contour.closed || candidate > index)
        .find(|&candidate| contour.points[candidate].kind != PointKind::OffCurve)
}

/// Lists the stored indexes strictly between two points in cyclic order.
fn cyclic_between(len: usize, start: usize, end: usize) -> Vec<usize> {
    let mut result = Vec::new();
    if len == 0 {
        return result;
    }
    let mut cursor = (start + 1) % len;
    while cursor != end && result.len() < len {
        result.push(cursor);
        cursor = (cursor + 1) % len;
    }
    result
}

/// Counts the off-curve points immediately preceding a stored index.
fn preceding_controls(contour: &Contour, index: usize) -> usize {
    let len = contour.points.len();
    (1..len)
        .map(|offset| (index + len - offset) % len)
        .take_while(|&candidate| {
            (contour.closed || candidate < index)
                && contour
                    .points
                    .get(candidate)
                    .is_some_and(|point| point.kind == PointKind::OffCurve)
        })
        .count()
}

/// Restores the point kinds a contour needs to stay convertible to a path.
///
/// Off-curve points may not open or close an open contour, and a cubic endpoint
/// requires exactly two preceding handles; anything else becomes a quadratic or
/// straight endpoint.
fn repair_kinds(contour: &mut Contour) {
    if !contour.closed {
        while contour
            .points
            .first()
            .is_some_and(|point| point.kind == PointKind::OffCurve)
        {
            contour.points.remove(0);
        }
        while contour
            .points
            .last()
            .is_some_and(|point| point.kind == PointKind::OffCurve)
        {
            contour.points.pop();
        }
        if let Some(first) = contour.points.first_mut() {
            first.kind = PointKind::Move;
        }
    }
    for index in 0..contour.points.len() {
        if contour.points[index].kind != PointKind::Curve {
            continue;
        }
        if !contour.closed && index == 0 {
            continue;
        }
        let controls = preceding_controls(contour, index);
        if controls != 2 {
            contour.points[index].kind = if controls == 0 {
                PointKind::Line
            } else {
                PointKind::QCurve
            };
        }
    }
}

fn off_curve(point: kurbo::Point) -> Point {
    Point {
        x: point.x,
        y: point.y,
        kind: PointKind::OffCurve,
        ..Point::default()
    }
}

fn index_error(what: &str, index: usize, len: usize) -> Error {
    Error::InvalidCommand(format!(
        "{what} index {index} is out of bounds for length {len}"
    ))
}
