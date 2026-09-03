use fw_model::{Component, Contour, GlyphName, Layer, Point, PointKind, TtIndex};
use kurbo::Affine;
use read_fonts::{
    tables::{
        glyf::{Anchor, CompositeGlyphFlags, Glyf, Glyph},
        loca::Loca,
    },
    types::GlyphId,
    FontRef, TableProvider,
};

use crate::Error;

pub(crate) struct GlyfSource<'a> {
    glyf: Glyf<'a>,
    loca: Loca<'a>,
}

impl<'a> GlyfSource<'a> {
    pub(crate) fn new(face: &FontRef<'a>) -> Result<Option<Self>, Error> {
        if face
            .table_data(read_fonts::types::Tag::new(b"glyf"))
            .is_none()
        {
            return Ok(None);
        }
        let head = face.head().map_err(|error| Error::parse("head", error))?;
        let glyf = face.glyf().map_err(|error| Error::parse("glyf", error))?;
        let loca = face
            .loca(head.index_to_loc_format() != 0)
            .map_err(|error| Error::parse("loca", error))?;
        Ok(Some(Self { glyf, loca }))
    }

    pub(crate) fn apply(
        &self,
        gid: u16,
        names: &[GlyphName],
        layer: &mut Layer,
    ) -> Result<(), Error> {
        let glyph = self
            .loca
            .get_glyf(GlyphId::new(u32::from(gid)), &self.glyf)
            .map_err(|error| Error::parse("glyf", error))?;
        match glyph {
            Some(Glyph::Simple(simple)) => {
                layer.contours = simple_contours(&simple)?;
                if !simple.instructions().is_empty() {
                    layer.tt_instructions = Some(simple.instructions().to_vec());
                }
            }
            Some(Glyph::Composite(composite)) => {
                let raw_components: Vec<_> = composite.components().collect();
                let Some(last) = raw_components.last() else {
                    return Err(Error::Parse {
                        table: "glyf",
                        reason: format!("composite glyph {gid} has no readable components"),
                    });
                };
                if last.flags.contains(CompositeGlyphFlags::MORE_COMPONENTS) {
                    return Err(Error::Parse {
                        table: "glyf",
                        reason: format!("component stream for glyph {gid} is truncated"),
                    });
                }
                let has_instructions = last
                    .flags
                    .contains(CompositeGlyphFlags::WE_HAVE_INSTRUCTIONS);
                let instructions = composite.instructions();
                if has_instructions && instructions.is_none() {
                    return Err(Error::Parse {
                        table: "glyf",
                        reason: format!("instructions for composite glyph {gid} are truncated"),
                    });
                }
                layer.components = raw_components
                    .into_iter()
                    .map(|component| map_component(component, names))
                    .collect::<Result<_, _>>()?;
                if let Some(instructions) = instructions.filter(|data| !data.is_empty()) {
                    layer.tt_instructions = Some(instructions.to_vec());
                }
            }
            None => {}
        }
        Ok(())
    }
}

fn simple_contours(
    simple: &read_fonts::tables::glyf::SimpleGlyph<'_>,
) -> Result<Vec<Contour>, Error> {
    let points: Vec<_> = simple.points().collect();
    if points.len() != simple.num_points() {
        return Err(Error::Parse {
            table: "glyf",
            reason: "point stream ended before the declared point count".to_owned(),
        });
    }
    let mut start = 0usize;
    let mut contours = Vec::with_capacity(simple.end_pts_of_contours().len());
    for endpoint in simple.end_pts_of_contours() {
        let end = usize::from(endpoint.get())
            .checked_add(1)
            .ok_or_else(|| Error::Parse {
                table: "glyf",
                reason: "contour endpoint overflow".to_owned(),
            })?;
        let source = points.get(start..end).ok_or_else(|| Error::Parse {
            table: "glyf",
            reason: "contour endpoint exceeds the point stream".to_owned(),
        })?;
        let len = source.len();
        let contour_points = source
            .iter()
            .enumerate()
            .map(|(offset, point)| {
                let previous = source[(offset + len - 1) % len];
                let next = source[(offset + 1) % len];
                let kind = if !point.on_curve {
                    PointKind::OffCurve
                } else if previous.on_curve && next.on_curve {
                    PointKind::Line
                } else {
                    PointKind::QCurve
                };
                Point {
                    x: f64::from(point.x),
                    y: f64::from(point.y),
                    kind,
                    smooth: false,
                    name: None,
                    tt_index: TtIndex::Index((start + offset) as u16),
                }
            })
            .collect();
        contours.push(Contour {
            closed: true,
            points: contour_points,
            name: None,
            id: None,
        });
        start = end;
    }
    if start != points.len() {
        return Err(Error::Parse {
            table: "glyf",
            reason: "unassigned points remain after the last contour".to_owned(),
        });
    }
    Ok(contours)
}

fn map_component(
    component: read_fonts::tables::glyf::Component,
    names: &[GlyphName],
) -> Result<Component, Error> {
    let base = names
        .get(usize::from(component.glyph.to_u16()))
        .cloned()
        .ok_or_else(|| Error::Parse {
            table: "glyf",
            reason: format!(
                "component glyph {} is out of range",
                component.glyph.to_u16()
            ),
        })?;
    let xx = f64::from(component.transform.xx.to_f32());
    let yx = f64::from(component.transform.yx.to_f32());
    let xy = f64::from(component.transform.xy.to_f32());
    let yy = f64::from(component.transform.yy.to_f32());
    let (mut dx, mut dy, point_match) = match component.anchor {
        Anchor::Offset { x, y } => (f64::from(x), f64::from(y), None),
        Anchor::Point { base, component } => (0.0, 0.0, Some((base, component))),
    };
    if component
        .flags
        .contains(CompositeGlyphFlags::SCALED_COMPONENT_OFFSET)
    {
        (dx, dy) = (xx * dx + xy * dy, yx * dx + yy * dy);
    }
    Ok(Component {
        base,
        transform: Affine::new([xx, yx, xy, yy, dx, dy]),
        anchor: None,
        auto_align: false,
        use_my_metrics: component
            .flags
            .contains(CompositeGlyphFlags::USE_MY_METRICS),
        round_to_grid: component
            .flags
            .contains(CompositeGlyphFlags::ROUND_XY_TO_GRID),
        smart: None,
        point_match,
    })
}
