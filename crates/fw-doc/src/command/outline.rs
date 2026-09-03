use std::collections::{BTreeMap, HashSet};

use fw_model::{Contour, Font, GlyphName, LayerId, Point, PointKind};
use kurbo::{Affine, Point as KurboPoint};

use crate::Error;

use super::{
    apply::{checked_indices, ensure_finite, layer_mut, remove_indices},
    Command, CurveKind, DeleteMode, HandleMode, PathOp, PointRef, SegmentKind,
};

pub(super) fn apply(font: &mut Font, command: &Command) -> Result<(), Error> {
    match command {
        Command::MovePoints {
            glyph,
            layer,
            points,
            delta,
            handles,
        } => move_points(font, glyph, layer, points, *delta, *handles),
        Command::SetPointPosition {
            glyph,
            layer,
            point,
            x,
            y,
        } => set_position(font, glyph, layer, *point, *x, *y),
        Command::SetSmooth {
            glyph,
            layer,
            points,
            smooth,
            level_handles,
        } => set_smooth(font, glyph, layer, points, *smooth, *level_handles),
        Command::InsertPointOnSegment {
            glyph,
            layer,
            contour,
            segment,
            t,
        } => insert(font, glyph, layer, *contour, *segment, *t),
        Command::DeletePoints {
            glyph,
            layer,
            points,
            mode,
        } => delete_points(font, glyph, layer, points, *mode),
        Command::ConvertSegment {
            glyph,
            layer,
            contour,
            segment,
            to,
        } => convert_segment(font, glyph, layer, *contour, *segment, *to),
        Command::AddContour {
            glyph,
            layer,
            contour,
        } => {
            contour.try_to_bezpath()?;
            layer_mut(font, glyph, layer)?
                .contours
                .push(contour.clone());
            Ok(())
        }
        Command::DeleteContours {
            glyph,
            layer,
            contours,
        } => {
            let target = &mut layer_mut(font, glyph, layer)?.contours;
            let indices = checked_indices(contours, target.len(), "contour")?;
            remove_indices(target, &indices);
            Ok(())
        }
        Command::ReverseContours {
            glyph,
            layer,
            contours,
        } => {
            let target = &mut layer_mut(font, glyph, layer)?.contours;
            let indices = checked_indices(contours, target.len(), "contour")?;
            for index in indices {
                target[index] = fw_geom::reverse(&target[index]);
            }
            Ok(())
        }
        Command::SetStartPoint {
            glyph,
            layer,
            point,
        } => set_start(font, glyph, layer, *point),
        Command::JoinContours { glyph, layer, a, b } => join(font, glyph, layer, *a, *b),
        Command::SplitContour {
            glyph,
            layer,
            point,
        } => split(font, glyph, layer, *point),
        Command::TransformSelection {
            glyph,
            layer,
            points,
            components,
            anchors,
            affine,
        } => transform_selection(font, glyph, layer, points, components, anchors, *affine),
        Command::PathOp { glyph, layer, op } => path_op(font, glyph, layer, op),
        _ => Err(Error::Internal(
            "non-outline command routed to outline handler".to_owned(),
        )),
    }
}

fn move_points(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    requested: &[PointRef],
    delta: (f64, f64),
    handles: HandleMode,
) -> Result<(), Error> {
    ensure_finite(&[delta.0, delta.1], "point delta")?;
    let target = layer_mut(font, glyph, layer)?;
    validate_points(&target.contours, requested)?;
    if requested.iter().any(|point_ref| {
        target.contours[point_ref.contour].points[point_ref.point].kind == PointKind::OffCurve
    }) {
        return Err(Error::InvalidCommand(
            "SetSmooth requires on-curve points".to_owned(),
        ));
    }
    let selected: HashSet<_> = requested.iter().copied().collect();
    let mut moved = selected.clone();
    if handles == HandleMode::WithHandles {
        for point_ref in &selected {
            let contour = &target.contours[point_ref.contour];
            if contour.points[point_ref.point].kind == PointKind::OffCurve {
                continue;
            }
            for adjacent in adjacent_indices(contour, point_ref.point) {
                let adjacent_ref = PointRef {
                    contour: point_ref.contour,
                    point: adjacent,
                };
                if contour.points[adjacent].kind == PointKind::OffCurve
                    && !selected.contains(&adjacent_ref)
                {
                    moved.insert(adjacent_ref);
                }
            }
        }
    }
    for point_ref in moved {
        let point = &mut target.contours[point_ref.contour].points[point_ref.point];
        point.x += delta.0;
        point.y += delta.1;
    }
    Ok(())
}

fn set_position(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    point_ref: PointRef,
    x: f64,
    y: f64,
) -> Result<(), Error> {
    ensure_finite(&[x, y], "point position")?;
    let target = layer_mut(font, glyph, layer)?;
    validate_points(&target.contours, &[point_ref])?;
    let point = &mut target.contours[point_ref.contour].points[point_ref.point];
    point.x = x;
    point.y = y;
    Ok(())
}

fn set_smooth(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    requested: &[PointRef],
    smooth: bool,
    level_handles: bool,
) -> Result<(), Error> {
    let target = layer_mut(font, glyph, layer)?;
    validate_points(&target.contours, requested)?;
    let mut points = requested
        .iter()
        .copied()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    points.sort_by_key(|point| (point.contour, point.point));
    for point_ref in points {
        let contour = &mut target.contours[point_ref.contour];
        contour.points[point_ref.point].smooth = smooth;
        if level_handles {
            level_node_handles(contour, point_ref.point);
        }
    }
    Ok(())
}

fn level_node_handles(contour: &mut Contour, node: usize) {
    let adjacent = adjacent_indices(contour, node);
    if adjacent.len() != 2
        || adjacent
            .iter()
            .any(|&index| contour.points[index].kind != PointKind::OffCurve)
    {
        return;
    }
    let before = adjacent[0];
    let after = adjacent[1];
    let center = KurboPoint::new(contour.points[node].x, contour.points[node].y);
    let incoming = center - KurboPoint::new(contour.points[before].x, contour.points[before].y);
    let outgoing = KurboPoint::new(contour.points[after].x, contour.points[after].y) - center;
    let incoming_length = incoming.hypot();
    let outgoing_length = outgoing.hypot();
    if incoming_length <= f64::EPSILON || outgoing_length <= f64::EPSILON {
        return;
    }
    let sum = incoming / incoming_length + outgoing / outgoing_length;
    let tangent = if sum.hypot() <= f64::EPSILON {
        incoming / incoming_length
    } else {
        sum / sum.hypot()
    };
    let leveled_before = center - tangent * incoming_length;
    let leveled_after = center + tangent * outgoing_length;
    contour.points[before].x = leveled_before.x;
    contour.points[before].y = leveled_before.y;
    contour.points[after].x = leveled_after.x;
    contour.points[after].y = leveled_after.y;
}

fn insert(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    contour_index: usize,
    segment: usize,
    t: f64,
) -> Result<(), Error> {
    ensure_finite(&[t], "split parameter")?;
    if !(0.0..1.0).contains(&t) || t == 0.0 {
        return Err(Error::InvalidCommand(
            "split parameter must be inside 0..1".to_owned(),
        ));
    }
    let contours = &mut layer_mut(font, glyph, layer)?.contours;
    let len = contours.len();
    let contour = contours
        .get_mut(contour_index)
        .ok_or_else(|| index_error("contour", contour_index, len))?;
    if segment >= contour.to_bezpath().segments().count() {
        return Err(index_error(
            "segment",
            segment,
            contour.to_bezpath().segments().count(),
        ));
    }
    *contour = fw_geom::split_segment(contour, segment, t);
    Ok(())
}

fn delete_points(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    points: &[PointRef],
    mode: DeleteMode,
) -> Result<(), Error> {
    let target = layer_mut(font, glyph, layer)?;
    validate_points(&target.contours, points)?;
    let mut contours = target.contours.clone();
    let mut grouped = BTreeMap::<usize, Vec<usize>>::new();
    for point in points {
        grouped.entry(point.contour).or_default().push(point.point);
    }
    for indices in grouped.values_mut() {
        indices.sort_unstable();
        indices.dedup();
    }
    for (contour_index, indices) in grouped.into_iter().rev() {
        match mode {
            DeleteMode::KeepShape => {
                let indices = unique_handle_segments(&contours[contour_index], &indices);
                for index in indices.into_iter().rev() {
                    delete_keep_shape(&mut contours[contour_index], index)?;
                }
                if contours[contour_index].points.is_empty() {
                    contours.remove(contour_index);
                }
            }
            DeleteMode::BreakContour => break_at_points(&mut contours, contour_index, &indices)?,
        }
    }
    target.contours = contours;
    Ok(())
}

fn delete_keep_shape(contour: &mut Contour, index: usize) -> Result<(), Error> {
    if contour.points[index].kind == PointKind::OffCurve {
        return delete_handle_as_line(contour, index);
    }
    let on_curves = on_curve_indices(contour);
    let Some(position) = on_curves.iter().position(|&candidate| candidate == index) else {
        return Ok(());
    };
    if on_curves.len() < 3
        || (!contour.closed && (position == 0 || position + 1 == on_curves.len()))
    {
        remove_endpoint(contour, index);
        return Ok(());
    }
    let previous = on_curves[(position + on_curves.len() - 1) % on_curves.len()];
    let next = on_curves[(position + 1) % on_curves.len()];
    if contour.closed {
        contour.points.rotate_left(previous);
        let target = (index + contour.points.len() - previous) % contour.points.len();
        let next = (next + contour.points.len() - previous) % contour.points.len();
        fit_joined_span(contour, 0, target, next)
    } else {
        fit_joined_span(contour, previous, index, next)
    }
}

fn fit_joined_span(
    contour: &mut Contour,
    previous: usize,
    target: usize,
    next: usize,
) -> Result<(), Error> {
    let mut temporary = Contour {
        closed: false,
        points: contour.points[previous..=next].to_vec(),
        name: None,
        id: None,
    };
    temporary.points[0].kind = PointKind::Move;
    let quadratic = temporary.is_quadratic();
    let mut error = 1.0;
    let fitted = loop {
        let options = fw_geom::SimplifyOptions {
            error,
            ignore_slopes: false,
            ignore_extrema: true,
            smooth_curves: false,
            force_lines: false,
            nearly_hv_lines: false,
            start_at_extrema: false,
        };
        let candidate = fw_geom::simplify(std::slice::from_ref(&temporary), &options)
            .into_iter()
            .next()
            .unwrap_or_else(|| temporary.clone());
        if candidate.to_bezpath().segments().count() == 1 || error >= 4096.0 {
            break candidate;
        }
        error *= 2.0;
    };
    if fitted.to_bezpath().segments().count() != 1 {
        return Err(Error::Geometry(
            "fw-geom could not fit adjacent segments into one segment".to_owned(),
        ));
    }
    let kind = if quadratic {
        CurveKind::Quadratic { max_err: 0.1 }
    } else {
        CurveKind::Cubic
    };
    let paths = fw_geom::to_bezpaths(std::slice::from_ref(&fitted));
    let converted = fw_geom::from_bezpaths(&paths, kind.geometry())
        .into_iter()
        .next()
        .unwrap_or(fitted);
    contour
        .points
        .splice(previous + 1..=next, converted.points.into_iter().skip(1));
    let _ = target;
    Ok(())
}

fn delete_handle_as_line(contour: &mut Contour, index: usize) -> Result<(), Error> {
    let previous = previous_on_curve(contour, index)
        .ok_or_else(|| Error::InvalidCommand("handle has no preceding node".to_owned()))?;
    let next = next_on_curve(contour, index)
        .ok_or_else(|| Error::InvalidCommand("handle has no following node".to_owned()))?;
    let mut remove = cyclic_between(contour.points.len(), previous, next);
    remove.retain(|&candidate| contour.points[candidate].kind == PointKind::OffCurve);
    remove.sort_unstable();
    remove_indices(&mut contour.points, &remove);
    let adjusted_next = next - remove.iter().filter(|&&removed| removed < next).count();
    if let Some(endpoint) = contour.points.get_mut(adjusted_next) {
        endpoint.kind = PointKind::Line;
    }
    Ok(())
}

fn remove_endpoint(contour: &mut Contour, index: usize) {
    let mut remove = vec![index];
    for adjacent in adjacent_indices(contour, index) {
        if contour.points[adjacent].kind == PointKind::OffCurve {
            remove.push(adjacent);
        }
    }
    remove.sort_unstable();
    remove.dedup();
    remove_indices(&mut contour.points, &remove);
    if !contour.closed {
        if let Some(first) = contour.points.first_mut() {
            first.kind = PointKind::Move;
        }
    }
}

fn break_at_points(
    contours: &mut Vec<Contour>,
    contour_index: usize,
    indices: &[usize],
) -> Result<(), Error> {
    if indices.is_empty() {
        return Ok(());
    }
    let mut contour = contours.remove(contour_index);
    let selected_on_curves = indices
        .iter()
        .copied()
        .filter(|&index| contour.points[index].kind != PointKind::OffCurve)
        .collect::<Vec<_>>();
    if selected_on_curves.is_empty() {
        for &index in indices.iter().rev() {
            delete_handle_as_line(&mut contour, index)?;
        }
        contours.insert(contour_index, contour);
        return Ok(());
    }

    let len = contour.points.len();
    let mut keep = vec![true; len];
    let mut force_line = HashSet::new();
    for &index in indices {
        if contour.points[index].kind == PointKind::OffCurve {
            let previous = previous_on_curve(&contour, index)
                .ok_or_else(|| Error::InvalidCommand("handle has no preceding node".to_owned()))?;
            let next = next_on_curve(&contour, index)
                .ok_or_else(|| Error::InvalidCommand("handle has no following node".to_owned()))?;
            for candidate in cyclic_between(len, previous, next) {
                if contour.points[candidate].kind == PointKind::OffCurve {
                    keep[candidate] = false;
                }
            }
            force_line.insert(next);
        }
    }
    for &index in &selected_on_curves {
        keep[index] = false;
        if contour.closed || index > 0 {
            let mut before = if index == 0 { len - 1 } else { index - 1 };
            while contour.points[before].kind == PointKind::OffCurve {
                keep[before] = false;
                if before == 0 {
                    if !contour.closed {
                        break;
                    }
                    before = len - 1;
                } else {
                    before -= 1;
                }
            }
        }
        if contour.closed || index + 1 < len {
            let mut after = (index + 1) % len;
            while contour.points[after].kind == PointKind::OffCurve {
                keep[after] = false;
                after = (after + 1) % len;
                if !contour.closed && after == 0 {
                    break;
                }
            }
        }
    }

    let order = if contour.closed {
        let start = keep
            .iter()
            .position(|kept| !kept)
            .map_or(0, |index| (index + 1) % len);
        (0..len)
            .map(|offset| (start + offset) % len)
            .collect::<Vec<_>>()
    } else {
        (0..len).collect()
    };
    let mut groups = Vec::<Vec<(usize, Point)>>::new();
    let mut current = Vec::new();
    for index in order {
        if keep[index] {
            current.push((index, contour.points[index].clone()));
        } else if !current.is_empty() {
            groups.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        groups.push(current);
    }

    let mut insert_at = contour_index;
    let mut metadata_available = true;
    for mut group in groups {
        while group
            .first()
            .is_some_and(|(_, point)| point.kind == PointKind::OffCurve)
        {
            group.remove(0);
        }
        while group
            .last()
            .is_some_and(|(_, point)| point.kind == PointKind::OffCurve)
        {
            group.pop();
        }
        if group.is_empty() {
            continue;
        }
        for (source_index, point) in &mut group {
            if force_line.contains(source_index) {
                point.kind = PointKind::Line;
            }
        }
        let mut points = group
            .into_iter()
            .map(|(_, point)| point)
            .collect::<Vec<_>>();
        points[0].kind = PointKind::Move;
        contours.insert(
            insert_at,
            Contour {
                closed: false,
                points,
                name: if metadata_available {
                    contour.name.clone()
                } else {
                    None
                },
                id: if metadata_available {
                    contour.id.clone()
                } else {
                    None
                },
            },
        );
        metadata_available = false;
        insert_at += 1;
    }
    Ok(())
}
fn convert_segment(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    contour_index: usize,
    segment: usize,
    to: SegmentKind,
) -> Result<(), Error> {
    let contours = &mut layer_mut(font, glyph, layer)?.contours;
    let len = contours.len();
    let contour = contours
        .get_mut(contour_index)
        .ok_or_else(|| index_error("contour", contour_index, len))?;
    let (start, end) = fw_geom::segment_to_points(contour, segment);
    if start >= contour.points.len() || end >= contour.points.len() {
        return Err(index_error(
            "segment",
            segment,
            contour.to_bezpath().segments().count(),
        ));
    }
    if contour.closed {
        contour.points.rotate_left(start);
    }
    let end = if contour.closed {
        (end + contour.points.len() - start) % contour.points.len()
    } else {
        end
    };
    let start = if contour.closed { 0 } else { start };
    let control_count = end.saturating_sub(start + 1);
    if (to == SegmentKind::Line && control_count == 0)
        || (to == SegmentKind::Curve
            && contour.points[end].kind == PointKind::Curve
            && control_count == 2)
    {
        return Ok(());
    }
    let from = contour.points[start].clone();
    let endpoint = contour.points[end].clone();
    contour.points.drain(start + 1..end);
    let endpoint_index = start + 1;
    match to {
        SegmentKind::Line => contour.points[endpoint_index].kind = PointKind::Line,
        SegmentKind::Curve => {
            let one_third = point_between(&from, &endpoint, 1.0 / 3.0);
            let two_thirds = point_between(&from, &endpoint, 2.0 / 3.0);
            contour.points[endpoint_index].kind = PointKind::Curve;
            contour
                .points
                .splice(endpoint_index..endpoint_index, [one_third, two_thirds]);
        }
    }
    Ok(())
}

fn point_between(a: &Point, b: &Point, t: f64) -> Point {
    Point {
        x: (b.x - a.x).mul_add(t, a.x),
        y: (b.y - a.y).mul_add(t, a.y),
        kind: PointKind::OffCurve,
        ..Point::default()
    }
}

fn set_start(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    point: PointRef,
) -> Result<(), Error> {
    let contours = &mut layer_mut(font, glyph, layer)?.contours;
    validate_points(contours, &[point])?;
    let contour = &mut contours[point.contour];
    if !contour.closed || contour.points[point.point].kind == PointKind::OffCurve {
        return Err(Error::InvalidCommand(
            "start point must be an on-curve point of a closed contour".to_owned(),
        ));
    }
    contour.points.rotate_left(point.point);
    Ok(())
}

fn join(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    a: PointRef,
    b: PointRef,
) -> Result<(), Error> {
    let contours = &mut layer_mut(font, glyph, layer)?.contours;
    validate_points(contours, &[a, b])?;
    if a.contour == b.contour {
        return Err(Error::InvalidCommand(
            "JoinContours requires two contours".to_owned(),
        ));
    }
    let first = &contours[a.contour];
    let second = &contours[b.contour];
    if first.closed || second.closed || a.point + 1 != first.points.len() {
        return Err(Error::InvalidCommand(
            "a must be the end of an open contour".to_owned(),
        ));
    }
    let mut second = second.clone();
    if b.point + 1 == second.points.len() {
        second = fw_geom::reverse(&second);
    } else if b.point != 0 {
        return Err(Error::InvalidCommand(
            "b must be an endpoint of an open contour".to_owned(),
        ));
    }
    if let Some(start) = second.points.first_mut() {
        start.kind = PointKind::Line;
    }
    let second_index = b.contour;
    let mut first = contours[a.contour].clone();
    if same_position(first.points.last(), second.points.first()) {
        second.points.remove(0);
    }
    first.points.extend(second.points);
    let keep_index = if second_index < a.contour {
        a.contour - 1
    } else {
        a.contour
    };
    contours.remove(second_index);
    contours[keep_index] = first;
    Ok(())
}

fn split(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    point: PointRef,
) -> Result<(), Error> {
    let contours = &mut layer_mut(font, glyph, layer)?.contours;
    validate_points(contours, &[point])?;
    let contour = &mut contours[point.contour];
    if contour.points[point.point].kind == PointKind::OffCurve {
        return Err(Error::InvalidCommand(
            "split point must be on-curve".to_owned(),
        ));
    }
    if contour.closed {
        contour.points.rotate_left(point.point);
        contour.closed = false;
        while contour
            .points
            .last()
            .is_some_and(|point| point.kind == PointKind::OffCurve)
        {
            contour.points.pop();
        }
        contour.points[0].kind = PointKind::Move;
    } else {
        if point.point == 0 || point.point + 1 == contour.points.len() {
            return Err(Error::InvalidCommand(
                "open contour split must be an interior node".to_owned(),
            ));
        }
        let mut right = contour.clone();
        right.points = right.points[point.point..].to_vec();
        right.points[0].kind = PointKind::Move;
        contour.points.truncate(point.point + 1);
        contours.insert(point.contour + 1, right);
    }
    Ok(())
}

fn transform_selection(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    points: &[PointRef],
    components: &[usize],
    anchors: &[usize],
    coefficients: [f64; 6],
) -> Result<(), Error> {
    ensure_finite(&coefficients, "affine transform")?;
    let affine = Affine::new(coefficients);
    let target = layer_mut(font, glyph, layer)?;
    validate_points(&target.contours, points)?;
    let components = checked_indices(components, target.components.len(), "component")?;
    let anchors = checked_indices(anchors, target.anchors.len(), "anchor")?;
    if points.is_empty() && components.is_empty() && anchors.is_empty() {
        target.contours = fw_geom::transform(&target.contours, affine);
        for component in &mut target.components {
            component.transform = affine * component.transform;
        }
        for anchor in &mut target.anchors {
            let transformed = affine * KurboPoint::new(anchor.x, anchor.y);
            anchor.x = transformed.x;
            anchor.y = transformed.y;
        }
        return Ok(());
    }
    let transformed = fw_geom::transform(&target.contours, affine);
    for point in points.iter().copied().collect::<HashSet<_>>() {
        target.contours[point.contour].points[point.point].x =
            transformed[point.contour].points[point.point].x;
        target.contours[point.contour].points[point.point].y =
            transformed[point.contour].points[point.point].y;
    }
    for index in components {
        target.components[index].transform = affine * target.components[index].transform;
    }
    for index in anchors {
        let transformed =
            affine * KurboPoint::new(target.anchors[index].x, target.anchors[index].y);
        target.anchors[index].x = transformed.x;
        target.anchors[index].y = transformed.y;
    }
    Ok(())
}

fn path_op(font: &mut Font, glyph: &GlyphName, layer: &LayerId, op: &PathOp) -> Result<(), Error> {
    let target = layer_mut(font, glyph, layer)?;
    let contours = &target.contours;
    let result = match op {
        PathOp::RemoveOverlap => fw_geom::remove_overlap(
            contours,
            inferred_kind(contours).geometry(),
            fw_geom::DEFAULT_BOOLEAN_TOLERANCE,
        )
        .map_err(|error| Error::Geometry(error.to_string()))?,
        PathOp::CorrectDirection => fw_geom::correct_direction(contours, true),
        PathOp::AddExtrema => fw_geom::add_extrema(contours),
        PathOp::AddInflections => fw_geom::add_inflections(contours),
        PathOp::Simplify(options) => fw_geom::simplify(contours, &options.geometry()),
        PathOp::RoundToGrid(step) => {
            if !step.is_finite() || *step <= 0.0 {
                return Err(Error::InvalidCommand(
                    "grid step must be positive and finite".to_owned(),
                ));
            }
            fw_geom::round_to_grid(contours, *step)
        }
        PathOp::CanonicalStart => fw_geom::canonical_start_points(contours),
        PathOp::CanonicalOrder => fw_geom::canonical_order(contours),
        PathOp::ExpandStroke(options) => fw_geom::expand_stroke(
            contours,
            &options.geometry(),
            inferred_kind(contours).geometry(),
        )
        .map_err(|error| Error::Geometry(error.to_string()))?,
        PathOp::ConvertCurves(kind) => {
            if matches!(kind, CurveKind::Quadratic { max_err } if !max_err.is_finite() || *max_err <= 0.0)
            {
                return Err(Error::InvalidCommand(
                    "quadratic conversion error must be positive and finite".to_owned(),
                ));
            }
            fw_geom::from_bezpaths(&fw_geom::to_bezpaths(contours), kind.geometry())
        }
    };
    target.contours = result;
    Ok(())
}

fn inferred_kind(contours: &[Contour]) -> CurveKind {
    if contours.iter().any(Contour::is_quadratic) {
        CurveKind::Quadratic { max_err: 0.1 }
    } else {
        CurveKind::Cubic
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

fn unique_handle_segments(contour: &Contour, indices: &[usize]) -> Vec<usize> {
    let mut segments = HashSet::new();
    indices
        .iter()
        .copied()
        .filter(|&index| {
            if contour.points[index].kind != PointKind::OffCurve {
                return true;
            }
            previous_on_curve(contour, index)
                .zip(next_on_curve(contour, index))
                .is_none_or(|segment| segments.insert(segment))
        })
        .collect()
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

fn cyclic_between(len: usize, start: usize, end: usize) -> Vec<usize> {
    let mut result = Vec::new();
    let mut cursor = (start + 1) % len;
    while cursor != end {
        result.push(cursor);
        cursor = (cursor + 1) % len;
    }
    result
}

fn same_position(a: Option<&Point>, b: Option<&Point>) -> bool {
    a.zip(b).is_some_and(|(a, b)| a.x == b.x && a.y == b.y)
}

fn index_error(what: &str, index: usize, len: usize) -> Error {
    Error::InvalidCommand(format!(
        "{what} index {index} is out of bounds for length {len}"
    ))
}
