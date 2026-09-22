use read_fonts::tables::glyf::{Anchor, CompositeGlyphFlags, Glyph};

use super::{basis::Source, invalid, GlyphPoint};
use crate::Error;

pub(super) struct Outline {
    pub points: Vec<GlyphPoint>,
    pub component_metrics: bool,
    contours: u32,
}

pub(super) fn expand(source: &Source<'_>, id: u16) -> Result<Outline, Error> {
    expand_inner(source, id, &mut Vec::new(), &mut 1_000_000usize)
}

fn expand_inner(
    source: &Source<'_>,
    id: u16,
    active: &mut Vec<u16>,
    budget: &mut usize,
) -> Result<Outline, Error> {
    if active.len() >= 64 || active.contains(&id) {
        return Err(invalid("component cycle or depth limit"));
    }
    active.push(id);
    let mut result = Outline {
        points: Vec::new(),
        contours: 0,
        component_metrics: false,
    };
    match source.glyph(id)? {
        Some(Glyph::Simple(simple)) => {
            let ends = simple.end_pts_of_contours();
            let mut contour = 0usize;
            for (index, point) in simple.points().enumerate() {
                spend(budget, 1)?;
                if index > 65534 {
                    return Err(invalid("expanded point count exceeds maxp capacity"));
                }
                while ends
                    .get(contour)
                    .is_some_and(|end| index > usize::from(end.get()))
                {
                    contour += 1;
                }
                if contour >= ends.len() {
                    return Err(invalid("point outside declared contours"));
                }
                result.points.push(GlyphPoint {
                    position: kurbo::Point::new(f64::from(point.x), f64::from(point.y)),
                    on_curve: point.on_curve,
                    contour: contour as u32,
                });
            }
            if result.points.len() != simple.num_points() {
                return Err(invalid("truncated point stream"));
            }
            result.contours = ends.len() as u32;
        }
        Some(Glyph::Composite(composite)) => {
            let mut last = None;
            for component in composite.components() {
                spend(budget, 1)?;
                last = Some(component.flags);
                let mut child = expand_inner(source, component.glyph.to_u16(), active, budget)?;
                if result.points.len() + child.points.len() > 65535 {
                    return Err(invalid("expanded point count exceeds maxp capacity"));
                }
                let t = component.transform;
                let [xx, xy, yx, yy] = [t.xx, t.xy, t.yx, t.yy].map(|v| f64::from(v.to_f32()));
                for point in &mut child.points {
                    let (x, y) = (point.position.x, point.position.y);
                    point.position = kurbo::Point::new(xx * x + xy * y, yx * x + yy * y);
                }
                let (mut dx, mut dy) = match component.anchor {
                    Anchor::Offset { x, y } => {
                        let (x, y) = (f64::from(x), f64::from(y));
                        if component
                            .flags
                            .contains(CompositeGlyphFlags::SCALED_COMPONENT_OFFSET)
                        {
                            (xx * x + xy * y, yx * x + yy * y)
                        } else {
                            (x, y)
                        }
                    }
                    Anchor::Point { base, component } => {
                        let base = result.points.get(usize::from(base)).ok_or_else(|| {
                            invalid(
                                "component anchoring to parent phantom points is not yet supported",
                            )
                        })?;
                        let child = child.points.get(usize::from(component)).ok_or_else(|| {
                            invalid(
                                "component anchoring to child phantom points is not yet supported",
                            )
                        })?;
                        (
                            base.position.x - child.position.x,
                            base.position.y - child.position.y,
                        )
                    }
                };
                // This view is in design units. Device-grid offset rounding is
                // applied later by the interpreter at the selected ppem.
                if !dx.is_finite() || !dy.is_finite() {
                    return Err(invalid("nonfinite component offset"));
                }
                if dx == -0.0 {
                    dx = 0.0;
                }
                if dy == -0.0 {
                    dy = 0.0;
                }
                for point in &mut child.points {
                    point.position += kurbo::Vec2::new(dx, dy);
                    point.contour += result.contours;
                    if !point.position.is_finite()
                        || point.position.x.abs() > 1_000_000.0
                        || point.position.y.abs() > 1_000_000.0
                    {
                        return Err(invalid("expanded component coordinate limit"));
                    }
                }
                result.contours += child.contours;
                result.component_metrics |= component
                    .flags
                    .contains(CompositeGlyphFlags::USE_MY_METRICS);
                result.points.extend(child.points);
            }
            if last.is_none_or(|flags| flags.contains(CompositeGlyphFlags::MORE_COMPONENTS)) {
                return Err(invalid("truncated component stream"));
            }
        }
        None => {}
    }
    active.pop();
    Ok(result)
}

fn spend(budget: &mut usize, amount: usize) -> Result<(), Error> {
    *budget = budget
        .checked_sub(amount)
        .ok_or_else(|| invalid("component expansion work limit exceeded"))?;
    Ok(())
}
