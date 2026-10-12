//! Point-bound PostScript hint masks shared by native output formats.

use crate::{Contour, Error, PointKind, PsHints};
use std::collections::BTreeMap;

/// Maps stored on-curve indices to emitted path operators without coordinate matching.
pub type PathHintMasks<'a> = BTreeMap<usize, Vec<(bool, &'a [u8])>>;

/// Maps validated point masks to path element indices and checks active stem overlap.
///
/// The indices refer to the concatenation of each contour's `try_to_bezpath()`
/// output, including closing edges. Components appended to that path do not
/// change the returned indices.
pub fn path_hint_masks<'a>(
    hints: &'a PsHints,
    contours: &[Contour],
) -> Result<PathHintMasks<'a>, Error> {
    hints.validate(contours)?;
    let mut result: BTreeMap<usize, Vec<(bool, &[u8])>> = BTreeMap::new();
    let mut by_contour: BTreeMap<usize, Vec<_>> = BTreeMap::new();
    for mask in &hints.hint_masks {
        by_contour.entry(mask.contour).or_default().push(mask);
    }
    let mut path_offset = 0;
    for (ci, contour) in contours.iter().enumerate() {
        let path = contour
            .try_to_bezpath()
            .map_err(|e| Error::InvalidHints(e.to_string()))?;
        let mut requested: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for mask in by_contour.get(&ci).into_iter().flatten() {
            if mask.closing {
                let first = contour
                    .points
                    .get(mask.point)
                    .ok_or_else(|| Error::InvalidHints("missing closing hint point".into()))?;
                let last = path.elements().get(path.elements().len().saturating_sub(2));
                let endpoint = match last {
                    Some(
                        kurbo::PathEl::LineTo(p)
                        | kurbo::PathEl::QuadTo(_, p)
                        | kurbo::PathEl::CurveTo(_, _, p),
                    ) => Some(*p),
                    _ => None,
                };
                let index = path.elements().len().saturating_sub(
                    if endpoint == Some(kurbo::Point::new(first.x, first.y)) {
                        2
                    } else {
                        1
                    },
                );
                result
                    .entry(path_offset + index)
                    .or_default()
                    .push((mask.counter, mask.mask.as_slice()));
            } else {
                requested
                    .entry(mask.point)
                    .or_default()
                    .push((mask.counter, mask.mask.as_slice()));
            }
        }
        if !requested.is_empty() {
            let start = contour
                .points
                .iter()
                .position(|p| p.kind != PointKind::OffCurve)
                .ok_or_else(|| {
                    Error::InvalidHints("hint mask has no explicit on-curve point".into())
                })?;
            if let Some(masks) = requested.get(&start) {
                result.insert(path_offset, masks.clone());
            }
            let mut path_index = 0;
            let mut controls = 0;
            for offset in 1..contour.points.len() {
                let index = (start + offset) % contour.points.len();
                let point = &contour.points[index];
                if point.kind == PointKind::OffCurve {
                    controls += 1;
                    continue;
                }
                path_index += if point.kind == PointKind::Curve || controls == 0 {
                    1
                } else {
                    controls
                };
                if let Some(masks) = requested.get(&index) {
                    result.insert(path_offset + path_index, masks.clone());
                }
                controls = 0;
            }
        }
        path_offset += path.elements().len();
    }
    // With no explicit initial active mask, every declared stem is active.
    let initial = result.get(&0).and_then(|masks| {
        masks
            .iter()
            .find(|(counter, _)| !counter)
            .map(|(_, bytes)| *bytes)
    });
    check_active(hints, initial)?;
    for mask in hints.hint_masks.iter().filter(|m| !m.counter) {
        check_active(hints, Some(&mask.mask))?;
    }
    Ok(result)
}

fn check_active(hints: &PsHints, mask: Option<&[u8]>) -> Result<(), Error> {
    for (offset, stems) in [(0, &hints.h_stems), (hints.h_stems.len(), &hints.v_stems)] {
        let mut active: Vec<_> = stems
            .iter()
            .enumerate()
            .filter_map(|(index, stem)| {
                let index = offset + index;
                let enabled =
                    mask.is_none_or(|bytes| bytes[index / 8] & (0x80 >> (index % 8)) != 0);
                let start = if stem.width == -21.0 {
                    stem.start + stem.width
                } else {
                    stem.start
                };
                enabled.then_some((start, start + stem.width.max(0.0)))
            })
            .collect();
        active.sort_by(|a, b| a.0.total_cmp(&b.0));
        if active.windows(2).any(|pair| pair[1].0 < pair[0].1) {
            return Err(Error::InvalidHints("overlapping PostScript stems require nonoverlapping active sets in every hint mask, including before the first contour".into()));
        }
    }
    Ok(())
}
