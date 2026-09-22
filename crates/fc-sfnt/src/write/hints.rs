//! CFF1 private dictionaries, stem operators and point-bound hint replacement.

use std::collections::BTreeMap;

use fc_model::{Layer, PointKind, PsPrivate};

use super::{
    encoding::{integer, number, real},
    WriteError,
};

pub(super) fn private(values: &PsPrivate) -> Result<Vec<u8>, WriteError> {
    values.validate().map_err(|e| WriteError(e.to_string()))?;
    let mut bytes = vec![139, 20, 139, 21]; // defaultWidthX = nominalWidthX = 0
    for (array, opcode) in [
        (&values.blue_values, &[6][..]),
        (&values.other_blues, &[7]),
        (&values.family_blues, &[8]),
        (&values.family_other_blues, &[9]),
        (&values.stem_snap_h, &[12, 12]),
        (&values.stem_snap_v, &[12, 13]),
    ] {
        if array.is_empty() {
            continue;
        }
        let mut previous = 0.0;
        for value in array {
            dict_number(&mut bytes, *value - previous)?;
            previous = *value;
        }
        bytes.extend(opcode);
    }
    for (value, opcode) in [
        (values.blue_scale, &[12, 9][..]),
        (values.blue_shift, &[12, 10]),
        (values.blue_fuzz, &[12, 11]),
        (values.std_hw, &[10]),
        (values.std_vw, &[11]),
        (values.force_bold.map(|v| f64::from(u8::from(v))), &[12, 14]),
        (values.language_group.map(f64::from), &[12, 17]),
    ] {
        if let Some(value) = value {
            dict_number(&mut bytes, value)?;
            bytes.extend(opcode);
        }
    }
    Ok(bytes)
}

fn dict_number(bytes: &mut Vec<u8>, value: f64) -> Result<(), WriteError> {
    if value.fract() == 0.0 && (i32::MIN as f64..=i32::MAX as f64).contains(&value) {
        integer(bytes, value as i32);
        Ok(())
    } else {
        real(bytes, value)
    }
}

/// Emits all stems after the optional width and before drawing operators.
pub(super) fn stems(bytes: &mut Vec<u8>, layer: &Layer) -> Result<(), WriteError> {
    layer
        .hints
        .validate(&layer.contours)
        .map_err(|e| WriteError(e.to_string()))?;
    if !layer.hints.d_stems.is_empty()
        || layer
            .hints
            .h_stems
            .iter()
            .chain(&layer.hints.v_stems)
            .any(|s| !s.ranges.is_empty())
    {
        return Err(WriteError("CFF hints require axis-aligned stems and explicit point masks; convert diagonal hints or active ranges first".into()));
    }
    let masks = !layer.hints.hint_masks.is_empty();
    for (stems, opcode) in [
        (&layer.hints.h_stems, if masks { 18 } else { 1 }),
        (&layer.hints.v_stems, if masks { 23 } else { 3 }),
    ] {
        // At most 46 operands plus the initial width: Type 2's stack limit is 48.
        for chunk in stems.chunks(23) {
            let mut previous = 0.0;
            for stem in chunk {
                number(bytes, stem.start - previous)?;
                number(bytes, stem.width)?;
                previous = stem.start + stem.width;
            }
            bytes.push(opcode);
        }
    }
    Ok(())
}

/// Maps stored on-curve indices to emitted path operators without coordinate matching.
pub(super) type Masks<'a> = BTreeMap<usize, Vec<(bool, &'a [u8])>>;

pub(super) fn masks(layer: &Layer) -> Result<Masks<'_>, WriteError> {
    let mut result: BTreeMap<usize, Vec<(bool, &[u8])>> = BTreeMap::new();
    let mut path_offset = 0;
    for (ci, contour) in layer.contours.iter().enumerate() {
        let path = contour
            .try_to_bezpath()
            .map_err(|e| WriteError(e.to_string()))?;
        let mut requested: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for mask in layer.hints.hint_masks.iter().filter(|m| m.contour == ci) {
            if mask.closing {
                let first = contour
                    .points
                    .get(mask.point)
                    .ok_or_else(|| WriteError("missing closing hint point".into()))?;
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
                .ok_or_else(|| WriteError("hint mask has no explicit on-curve point".into()))?;
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
    // With no explicit initial active mask, Type 2 activates every declared stem.
    let initial = result.get(&0).and_then(|masks| {
        masks
            .iter()
            .find(|(counter, _)| !counter)
            .map(|(_, bytes)| *bytes)
    });
    check_active(&layer.hints, initial)?;
    for mask in layer.hints.hint_masks.iter().filter(|m| !m.counter) {
        check_active(&layer.hints, Some(&mask.mask))?;
    }
    Ok(result)
}

fn check_active(hints: &fc_model::PsHints, mask: Option<&[u8]>) -> Result<(), WriteError> {
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
            return Err(WriteError("overlapping PostScript stems require nonoverlapping active sets in every hint mask, including before the first contour".into()));
        }
    }
    Ok(())
}
