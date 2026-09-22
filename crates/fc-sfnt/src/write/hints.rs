//! CFF1 private dictionaries, stem operators and point-bound hint replacement.

use fc_model::{Layer, PsPrivate};

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

/// Maps stored on-curve indices to emitted operators and validates active sets.
pub(super) fn masks(layer: &Layer) -> Result<fc_model::query::PathHintMasks<'_>, WriteError> {
    fc_model::query::path_hint_masks(&layer.hints, &layer.contours)
        .map_err(|error| WriteError(error.to_string()))
}
