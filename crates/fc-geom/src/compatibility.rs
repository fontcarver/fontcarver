//! Conservative compatibility repair: contour order, direction and start points.
use crate::{
    canonical_order, canonical_start_points, compatible, correct_direction, reverse, Error,
};
use fc_model::{Contour, Layer};
use kurbo::Shape;

/// Canonicalize source ordering without adding, removing or moving geometry.
pub fn canonical_source(layer: &Layer) -> Layer {
    let mut result = layer.clone();
    result.contours =
        canonical_start_points(&canonical_order(&correct_direction(&layer.contours, true)));
    result.components.sort_by(|a, b| a.base.cmp(&b.base));
    clear_point_hints(&mut result);
    result
}

/// Align a source to a reference by reordering and rotating matching contours.
/// Geometry is preserved. Unmatched topology fails instead of inventing points.
pub fn repair_compatibility(reference: &Layer, source: &Layer) -> Result<Layer, Error> {
    if reference.contours.len() != source.contours.len() {
        return Err(Error::DesignSpace(
            "contour counts differ; repair topology manually".into(),
        ));
    }
    let mut result = source.clone();
    let mut remaining = source.contours.clone();
    let mut output = Vec::new();
    let mut budget = 2_000_000usize;
    let bounds = |layer: &Layer| {
        layer
            .contours
            .iter()
            .map(|c| c.to_bezpath().bounding_box())
            .reduce(|a, b| a.union(b))
            .unwrap_or(kurbo::Rect::ZERO)
    };
    let left = bounds(reference);
    let right = bounds(source);
    for contour in &reference.contours {
        let mut best: Option<(usize, Contour, f64)> = None;
        for (index, candidate) in remaining.iter().enumerate() {
            if candidate.closed != contour.closed || candidate.points.len() != contour.points.len()
            {
                continue;
            }
            for mut candidate in [candidate.clone(), reverse(candidate)] {
                let rotations = if candidate.closed {
                    candidate.points.len().max(1)
                } else {
                    1
                };
                for _ in 0..rotations {
                    budget = budget
                        .checked_sub(candidate.points.len().max(1))
                        .ok_or_else(|| {
                            Error::DesignSpace("compatibility repair exceeds work limit".into())
                        })?;
                    if candidate
                        .points
                        .iter()
                        .zip(&contour.points)
                        .all(|(a, b)| a.kind == b.kind)
                    {
                        let score = candidate
                            .points
                            .iter()
                            .zip(&contour.points)
                            .map(|(a, b)| {
                                let x = (a.x - right.x0) / right.width().max(1.0)
                                    - (b.x - left.x0) / left.width().max(1.0);
                                let y = (a.y - right.y0) / right.height().max(1.0)
                                    - (b.y - left.y0) / left.height().max(1.0);
                                x * x + y * y
                            })
                            .sum::<f64>();
                        if best.as_ref().is_none_or(|(_, _, old)| score < *old) {
                            best = Some((index, candidate.clone(), score));
                        }
                    }
                    if !candidate.points.is_empty() {
                        candidate.points.rotate_left(1);
                    }
                }
            }
        }
        let (index, contour, _) = best.ok_or_else(|| {
            Error::DesignSpace(
                "point counts or kinds cannot be aligned without changing geometry".into(),
            )
        })?;
        remaining.remove(index);
        output.push(contour);
    }
    result.contours = output;
    result.components.sort_by(|a, b| a.base.cmp(&b.base));
    compatible(reference, &result).map_err(Error::Incompatible)?;
    clear_point_hints(&mut result);
    Ok(result)
}

fn clear_point_hints(layer: &mut Layer) {
    layer.tt_instructions = None;
    layer.hints.hint_masks.clear();
    for point in layer
        .contours
        .iter_mut()
        .flat_map(|contour| &mut contour.points)
    {
        point.tt_index = fc_model::TtIndex::Unnumbered;
    }
}

/// One drawable shape of a source layer: a stored contour or a component.
///
/// Shape order is the interpolation contract: two sources are compatible only
/// when their shapes correspond one to one in this order. The compatibility
/// view colours and numbers shapes by it, and repair swaps positions in it.
#[derive(Clone, Debug, PartialEq)]
pub struct SourceShape {
    /// Index inside its own kind, matching the model's contour or component order.
    pub index: usize,
    /// Base glyph when this shape is a component placement.
    pub component: Option<fc_model::GlyphName>,
    /// Whether a contour shape is closed; a component is always treated as closed.
    pub closed: bool,
    /// This shape's own outline, in font units.
    pub path: kurbo::BezPath,
    /// The contour's first stored point, or a component's transformed origin.
    pub start: kurbo::Point,
    /// Centre of the shape's bounding box, used for cross-source connectors.
    pub center: kurbo::Point,
    /// Stored points of a contour shape; empty for a component.
    pub points: Vec<kurbo::Point>,
    /// `true` where the point at the same index is on-curve.
    pub on_curve: Vec<bool>,
}

/// Total points one call will describe before it reports a work limit.
const MAX_SHAPE_POINTS: usize = 20_000;

/// Describe one source layer's shapes without changing the document.
///
/// Component shapes carry the resolved outline of the glyph they place, so the
/// view can draw them in shape order beside the stored contours.
///
/// # Errors
///
/// Returns an error when a component cannot be resolved, or when the layer
/// exceeds the shape work limit.
pub fn source_shapes(font: &fc_model::Font, layer: &Layer) -> Result<Vec<SourceShape>, Error> {
    let mut shapes = Vec::with_capacity(layer.contours.len() + layer.components.len());
    let mut points = 0usize;
    for (index, contour) in layer.contours.iter().enumerate() {
        points = points.saturating_add(contour.points.len());
        if points > MAX_SHAPE_POINTS {
            return Err(Error::DesignSpace(
                "source shapes exceed the inspection work limit".into(),
            ));
        }
        let path = contour.to_bezpath();
        let bounds = path.bounding_box();
        let start = contour.points.first().map_or(kurbo::Point::ZERO, |point| {
            kurbo::Point::new(point.x, point.y)
        });
        shapes.push(SourceShape {
            index,
            component: None,
            closed: contour.closed,
            path,
            start,
            center: bounds.center(),
            points: contour
                .points
                .iter()
                .map(|point| kurbo::Point::new(point.x, point.y))
                .collect(),
            on_curve: contour
                .points
                .iter()
                .map(|point| point.kind != fc_model::PointKind::OffCurve)
                .collect(),
        });
    }
    let master = layer_master(font, layer);
    for (index, component) in layer.components.iter().enumerate() {
        let base = match &component.smart {
            Some(values) if !values.is_empty() => {
                crate::flatten_smart_component(font, component.base.as_ref(), &master, values)
            }
            _ => crate::flatten_layer(font, component.base.as_ref(), &master),
        }
        .map_err(|error| Error::DesignSpace(error.to_string()))?;
        let mut path = base;
        path.apply_affine(component.transform);
        points = points.saturating_add(path.elements().len());
        if points > MAX_SHAPE_POINTS {
            return Err(Error::DesignSpace(
                "source shapes exceed the inspection work limit".into(),
            ));
        }
        let bounds = path.bounding_box();
        shapes.push(SourceShape {
            index,
            component: Some(component.base.clone()),
            closed: true,
            path,
            start: component.transform * kurbo::Point::ZERO,
            center: bounds.center(),
            points: Vec::new(),
            on_curve: Vec::new(),
        });
    }
    Ok(shapes)
}

/// The master a source layer belongs to, falling back to the default master.
fn layer_master(font: &fc_model::Font, layer: &Layer) -> fc_model::MasterId {
    match &layer.kind {
        fc_model::LayerKind::Master(master)
        | fc_model::LayerKind::Background(master)
        | fc_model::LayerKind::Brace { master, .. }
        | fc_model::LayerKind::Bracket { master, .. }
        | fc_model::LayerKind::Smart { master, .. } => master.clone(),
        fc_model::LayerKind::Backup { of, .. } => of.clone(),
        fc_model::LayerKind::Custom(_) => font.default_master().id.clone(),
    }
}

/// How compatible one segment or shape is across the compared sources.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegmentStatus {
    /// Same structure in every source, and directions agree.
    Compatible,
    /// Same structure, but one source's direction differs by more than the
    /// warning angle: usually a missing node.
    AngleWarning,
    /// Structure differs, so the segment cannot interpolate.
    Incompatible,
}

/// Direction difference, in degrees, above which a segment is warned about.
///
/// Matches the 20-degree threshold the Glyphs master-compatibility view uses.
pub const ANGLE_WARNING_DEGREES: f64 = 20.0;

/// Per-shape correspondence across an ordered set of sources.
#[derive(Clone, Debug, PartialEq)]
pub struct ShapeComparison {
    /// Shape position in the reference source.
    pub index: usize,
    /// Base glyph when the shape is a component placement.
    pub component: Option<fc_model::GlyphName>,
    /// Worst status of the shape.
    pub status: SegmentStatus,
    /// Status of each segment of the reference shape.
    pub segments: Vec<SegmentStatus>,
    /// Largest direction difference seen on any segment, in degrees.
    pub max_angle: f64,
}

/// Compare the shapes of several sources in shape order.
///
/// The first entry of `sources` is the reference. A shape that is missing from
/// a source, or whose point structure differs, is reported as incompatible
/// rather than compared segment by segment.
#[must_use]
pub fn compare_shapes(sources: &[Vec<SourceShape>]) -> Vec<ShapeComparison> {
    let Some(reference) = sources.first() else {
        return Vec::new();
    };
    reference
        .iter()
        .enumerate()
        .map(|(position, shape)| {
            let others: Vec<&SourceShape> = sources
                .iter()
                .skip(1)
                .filter_map(|source| source.get(position))
                .collect();
            let structural = others.len() + 1 != sources.len()
                || others.iter().any(|other| {
                    other.component != shape.component
                        || other.closed != shape.closed
                        || other.points.len() != shape.points.len()
                        || other.on_curve != shape.on_curve
                });
            if structural {
                return ShapeComparison {
                    index: shape.index,
                    component: shape.component.clone(),
                    status: SegmentStatus::Incompatible,
                    segments: vec![SegmentStatus::Incompatible; shape.points.len()],
                    max_angle: 0.0,
                };
            }
            let count = shape.points.len();
            let mut segments = Vec::with_capacity(count);
            let mut max_angle = 0.0f64;
            let limit = if shape.closed {
                count
            } else {
                count.saturating_sub(1)
            };
            for start in 0..limit {
                let end = (start + 1) % count.max(1);
                let angles: Vec<f64> = std::iter::once(shape)
                    .chain(others.iter().copied())
                    .filter_map(|source| {
                        let from = source.points.get(start)?;
                        let to = source.points.get(end)?;
                        let delta = *to - *from;
                        (delta.hypot() > f64::EPSILON).then(|| delta.atan2().to_degrees())
                    })
                    .collect();
                let spread = angle_spread(&angles);
                max_angle = max_angle.max(spread);
                segments.push(if spread > ANGLE_WARNING_DEGREES {
                    SegmentStatus::AngleWarning
                } else {
                    SegmentStatus::Compatible
                });
            }
            let status = if segments.contains(&SegmentStatus::AngleWarning) {
                SegmentStatus::AngleWarning
            } else {
                SegmentStatus::Compatible
            };
            ShapeComparison {
                index: shape.index,
                component: shape.component.clone(),
                status,
                segments,
                max_angle,
            }
        })
        .collect()
}

/// Largest pairwise difference between directions, wrapped into `[0, 180]`.
fn angle_spread(angles: &[f64]) -> f64 {
    let mut spread = 0.0f64;
    for (index, left) in angles.iter().enumerate() {
        for right in &angles[index + 1..] {
            let mut difference = (left - right).abs() % 360.0;
            if difference > 180.0 {
                difference = 360.0 - difference;
            }
            spread = spread.max(difference);
        }
    }
    spread
}

impl SegmentStatus {
    /// Stable identifier used by the API and the compatibility view.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Compatible => "compatible",
            Self::AngleWarning => "angle",
            Self::Incompatible => "incompatible",
        }
    }
}

/// Stable identifier of one status, for API payloads.
#[must_use]
pub const fn segment_status_name(status: SegmentStatus) -> &'static str {
    status.as_str()
}
