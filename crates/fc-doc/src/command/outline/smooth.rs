use std::collections::BTreeMap;

use fc_model::{Contour, Font, GlyphName, LayerId, PointKind};

use crate::Error;

use super::super::{apply::layer_mut, PointRef};
use super::{validate_points, Touched};

pub(super) fn set_super_smooth(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    points: &[PointRef],
    enabled: bool,
) -> Result<Touched, Error> {
    let target = layer_mut(font, glyph, layer)?;
    validate_points(&target.contours, points)?;
    for point in points {
        let node = &mut target.contours[point.contour].points[point.point];
        if node.kind == PointKind::OffCurve {
            return Err(Error::InvalidCommand(
                "super-smooth applies to on-curve nodes".into(),
            ));
        }
        node.super_smooth = enabled;
    }
    // The outline handler reapplies every persistent constraint before checking
    // the result and restores all touched contours if any projection fails.
    Ok(Touched::of_points(points))
}

pub(super) fn harmonize(
    font: &mut Font,
    glyph: &GlyphName,
    layer: &LayerId,
    points: &[PointRef],
) -> Result<Touched, Error> {
    let target = layer_mut(font, glyph, layer)?;
    validate_points(&target.contours, points)?;
    let mut selected = BTreeMap::<usize, Vec<usize>>::new();
    for point in points {
        selected.entry(point.contour).or_default().push(point.point);
    }
    for (contour_index, mut nodes) in selected {
        let contour = &mut target.contours[contour_index];
        nodes.extend(super_smooth_nodes(contour));
        fc_geom::harmonize_nodes(contour, &nodes).map_err(harmonize_error)?;
    }
    Ok(Touched::of_points(points))
}

pub(in crate::command) fn maintain_super_smooth(contour: &mut Contour) -> Result<(), Error> {
    let nodes = super_smooth_nodes(contour).collect::<Vec<_>>();
    fc_geom::harmonize_nodes(contour, &nodes).map_err(harmonize_error)
}

fn super_smooth_nodes(contour: &Contour) -> impl Iterator<Item = usize> + '_ {
    contour
        .points
        .iter()
        .enumerate()
        .filter_map(|(index, point)| point.super_smooth.then_some(index))
}

fn harmonize_error(error: fc_geom::HarmonizeError) -> Error {
    Error::InvalidCommand(format!("cannot harmonize the selected outline: {error}"))
}
