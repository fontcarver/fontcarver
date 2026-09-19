//! Native paths, components, anchors and guides.

use super::{error, extras, plist::*};
use crate::Error;
use fc_model::{Anchor, Component, Contour, Guide, Point, PointKind};
use glyphs_reader::Plist;
use kurbo::Affine;

pub(super) fn contour(raw: &Plist) -> Result<Contour, Error> {
    let closed = flag(raw, "closed", false)?;
    let mut points = Vec::new();
    for raw in array(raw, "nodes")? {
        let (x, y, kind) = match raw {
            Plist::String(value) => {
                let mut pieces = value.split('{').next().unwrap_or(value).split_whitespace();
                let x = pieces
                    .next()
                    .ok_or_else(|| error("node has no x coordinate"))?
                    .parse::<f64>()
                    .map_err(|_| error("invalid node x"))?;
                let y = pieces
                    .next()
                    .ok_or_else(|| error("node has no y coordinate"))?
                    .parse::<f64>()
                    .map_err(|_| error("invalid node y"))?;
                (x, y, pieces.collect::<Vec<_>>().join(" "))
            }
            Plist::Array(values) if values.len() >= 3 => (
                as_number(&values[0])?,
                as_number(&values[1])?,
                values[2]
                    .as_str()
                    .ok_or_else(|| error("invalid node type"))?
                    .into(),
            ),
            _ => return Err(error("invalid Glyphs node")),
        };
        if !x.is_finite() || !y.is_finite() {
            return Err(error("nonfinite node coordinate"));
        }
        let (kind, smooth) = match kind.as_str() {
            "LINE" | "l" => (PointKind::Line, false),
            "LINE SMOOTH" | "ls" => (PointKind::Line, true),
            "CURVE" | "c" => (PointKind::Curve, false),
            "CURVE SMOOTH" | "cs" => (PointKind::Curve, true),
            "QCURVE" | "q" => (PointKind::QCurve, false),
            "QCURVE SMOOTH" | "qs" => (PointKind::QCurve, true),
            "OFFCURVE" | "o" => (PointKind::OffCurve, false),
            _ => return Err(error(format!("unsupported node type {kind}"))),
        };
        points.push(Point {
            x,
            y,
            kind,
            smooth,
            ..Point::default()
        });
    }
    if !closed {
        if let Some(first) = points.first_mut() {
            if first.kind == PointKind::OffCurve {
                return Err(error("open contour begins off curve"));
            }
            first.kind = PointKind::Move;
        }
    }
    let mut contour = Contour {
        closed,
        points,
        ..Contour::default()
    };
    contour
        .try_to_bezpath()
        .map_err(|err| error(format!("malformed contour: {err}")))?;
    extras::restore_contour_metadata(raw, &mut contour)?;
    Ok(contour)
}

pub(super) fn write_contour(contour: &Contour, raw: Option<&Plist>) -> Result<Plist, Error> {
    contour
        .try_to_bezpath()
        .map_err(|err| error(format!("malformed contour: {err}")))?;
    let mut raw = raw.cloned().unwrap_or_else(dict);
    let old_nodes = array(&raw, "nodes")?.to_vec();
    set(&mut raw, "closed", boolean(contour.closed));
    let mut nodes = Vec::new();
    for (index, point) in contour.points.iter().enumerate() {
        let kind = match (point.kind, point.smooth) {
            (PointKind::OffCurve, _) => "o",
            (PointKind::Curve, false) => "c",
            (PointKind::Curve, true) => "cs",
            (PointKind::QCurve, false) => "q",
            (PointKind::QCurve, true) => "qs",
            (_, false) => "l",
            (_, true) => "ls",
        };
        let mut node = vec![number(point.x)?, number(point.y)?, string(kind)];
        if old_nodes.len() == contour.points.len() {
            match old_nodes.get(index) {
                Some(Plist::Array(values)) => node.extend(values.iter().skip(3).cloned()),
                Some(Plist::String(value)) => {
                    if let Some(start) = value.find('{') {
                        node.push(
                            Plist::parse(&value[start..])
                                .map_err(|err| error(format!("invalid node metadata: {err}")))?,
                        );
                    }
                }
                _ => {}
            }
        }
        nodes.push(Plist::Array(node));
    }
    set(&mut raw, "nodes", Plist::Array(nodes));
    extras::preserve_contour_metadata(&mut raw, contour)?;
    Ok(raw)
}

pub(super) fn component(raw: &Plist) -> Result<Component, Error> {
    if numeric(raw, "orientation", 0.)? != 0. {
        return Err(error(
            "component orientation relative to the center or right edge is not yet supported",
        ));
    }
    let alignment = numeric(raw, "alignment", 0.)?;
    if alignment != 0. && alignment != -1. {
        return Err(error("forced or partial component alignment is not yet represented by the model; use explicit transforms"));
    }
    if raw
        .get("piece")
        .is_some_and(|v| v.as_dict().is_none_or(|v| !v.is_empty()))
    {
        return Err(error("smart components are not yet supported"));
    }
    if raw.get("masterId").is_some() {
        return Err(error("cross-master components are not yet supported"));
    }
    let base = text(raw, "ref")
        .or_else(|| text(raw, "name"))
        .filter(|v| !v.is_empty())
        .ok_or_else(|| error("component has no reference"))?;
    let transform = if let Some(value) = raw.get("transform") {
        Affine::new(tuple::<6>(value)?)
    } else {
        let [x, y] = raw
            .get("pos")
            .map(tuple::<2>)
            .transpose()?
            .unwrap_or([0., 0.]);
        let [sx, sy] = raw
            .get("scale")
            .map(tuple::<2>)
            .transpose()?
            .unwrap_or([1., 1.]);
        let [kx, ky] = raw
            .get("slant")
            .map(tuple::<2>)
            .transpose()?
            .unwrap_or([0., 0.]);
        Affine::translate((x, y))
            * Affine::skew(kx.to_radians().tan(), ky.to_radians().tan())
            * Affine::rotate(numeric(raw, "angle", 0.)?.to_radians())
            * Affine::scale_non_uniform(sx, sy)
    };
    if transform.as_coeffs().iter().any(|n| !n.is_finite()) {
        return Err(error("nonfinite component transform"));
    }
    let mut result: Component =
        extras::restore(raw, &["use_my_metrics", "round_to_grid", "point_match"])?;
    result.base = base.into();
    result.transform = transform;
    result.anchor = text(raw, "anchor").map(str::to_owned);
    result.auto_align = alignment != -1.;
    Ok(result)
}

pub(super) fn write_component(component: &Component, raw: Option<&Plist>) -> Result<Plist, Error> {
    if component.smart.as_ref().is_some_and(|v| !v.is_empty()) {
        return Err(error("smart components are not yet supported"));
    }
    let mut raw = raw.cloned().unwrap_or_else(dict);
    remove(
        &mut raw,
        &[
            "name",
            "transform",
            "pos",
            "scale",
            "angle",
            "slant",
            "anchor",
            "piece",
        ],
    );
    set(&mut raw, "ref", string(component.base.to_string()));
    let [a, b, c, d, x, y] = component.transform.as_coeffs();
    if [a, b, c, d, x, y].iter().any(|v| !v.is_finite()) {
        return Err(error("nonfinite component transform"));
    }
    // Glyphs composes translate * skew * rotate * scale. Both skew axes
    // are available, so nonsingular matrices need no lossy approximation.
    let (scale, angle, slant) = if a != 0. && d != 0. {
        (
            [a, d],
            0.,
            [(c / d).atan().to_degrees(), (b / a).atan().to_degrees()],
        )
    } else if b != 0. && c != 0. {
        (
            [b, -c],
            90.,
            [(a / b).atan().to_degrees(), (d / c).atan().to_degrees()],
        )
    } else if a == 0. && b == 0. {
        ([0., c.hypot(d)], (-c).atan2(d).to_degrees(), [0., 0.])
    } else {
        ([a.hypot(b), 0.], b.atan2(a).to_degrees(), [0., 0.])
    };
    set(&mut raw, "scale", numbers(scale)?);
    set(&mut raw, "angle", number(angle)?);
    set(&mut raw, "slant", numbers(slant)?);
    set(&mut raw, "pos", numbers([x, y])?);
    set(
        &mut raw,
        "alignment",
        integer(if component.auto_align { 0 } else { -1 }),
    );
    if let Some(anchor) = &component.anchor {
        set(&mut raw, "anchor", string(anchor));
    }
    let reconstructed = self::component(&raw)?.transform.as_coeffs();
    if reconstructed
        .iter()
        .zip([a, b, c, d, x, y])
        .any(|(actual, expected)| (actual - expected).abs() > 1e-10 * expected.abs().max(1.))
    {
        return Err(error(
            "component transform cannot be represented accurately by Glyphs 3",
        ));
    }
    extras::store(
        &mut raw,
        component,
        &["use_my_metrics", "round_to_grid", "point_match"],
    )?;
    Ok(raw)
}

pub(super) fn anchors(raw: &Plist) -> Result<Vec<Anchor>, Error> {
    array(raw, "anchors")?
        .iter()
        .map(|raw| {
            let [x, y] = raw
                .get("pos")
                .or_else(|| raw.get("position"))
                .map(tuple::<2>)
                .transpose()?
                .unwrap_or([0., 0.]);
            Ok(Anchor {
                name: required(raw, "name")?.into(),
                x,
                y,
            })
        })
        .collect()
}

pub(super) fn write_anchors(anchors: &[Anchor], raw: &Plist) -> Result<Plist, Error> {
    Ok(Plist::Array(
        anchors
            .iter()
            .map(|anchor| {
                let mut raw = find(raw, "anchors", "name", &anchor.name)
                    .cloned()
                    .unwrap_or_else(dict);
                remove(&mut raw, &["position"]);
                set(&mut raw, "name", string(&anchor.name));
                set(&mut raw, "pos", numbers([anchor.x, anchor.y])?);
                Ok(raw)
            })
            .collect::<Result<_, Error>>()?,
    ))
}

pub(super) fn guides(raw: &Plist) -> Result<Vec<Guide>, Error> {
    let key = if raw.get("guides").is_some() {
        "guides"
    } else {
        "guideLines"
    };
    array(raw, key)?
        .iter()
        .map(|raw| {
            let [x, y] = raw
                .get("pos")
                .or_else(|| raw.get("position"))
                .map(tuple::<2>)
                .transpose()?
                .unwrap_or([0., 0.]);
            Ok(Guide {
                x,
                y,
                angle: numeric(raw, "angle", 0.)?,
                name: text(raw, "name").map(str::to_owned),
                locked: flag(raw, "locked", false)?,
                color: text(raw, "color").map(str::to_owned),
            })
        })
        .collect()
}

pub(super) fn write_guides(guides: &[Guide], source: Option<&Plist>) -> Result<Plist, Error> {
    let originals = source
        .and_then(|raw| raw.get("guides").or_else(|| raw.get("guideLines")))
        .and_then(Plist::as_array)
        .unwrap_or(&[]);
    Ok(Plist::Array(
        guides
            .iter()
            .enumerate()
            .map(|(index, guide)| {
                let mut raw = originals.get(index).cloned().unwrap_or_else(dict);
                remove(&mut raw, &["position", "name"]);
                set(&mut raw, "pos", numbers([guide.x, guide.y])?);
                set(&mut raw, "angle", number(guide.angle)?);
                if let Some(name) = &guide.name {
                    set(&mut raw, "name", string(name));
                }
                set(&mut raw, "locked", boolean(guide.locked));
                if let Some(color) = &guide.color {
                    set(&mut raw, "color", string(color));
                } else if raw.get("color").and_then(Plist::as_str).is_some() {
                    remove(&mut raw, &["color"]);
                }
                Ok(raw)
            })
            .collect::<Result<_, Error>>()?,
    ))
}
