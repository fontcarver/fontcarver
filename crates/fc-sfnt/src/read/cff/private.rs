//! CFF private dictionaries normalized to each master layer's font coordinates.

use std::ops::Range;

use fc_model::{FontInfo, Master, PsPrivate};
use kurbo::Affine;
use read_fonts::{
    ps::cff::{dict, index::Index},
    FontRef,
};

use super::{cff_data_and_top_dict, cff_font, dict_entries, effective_transform};
use crate::{Error, Warning};

pub(crate) fn apply_private(
    face: &FontRef<'_>,
    info: &mut FontInfo,
    master: &mut Master,
    warnings: &mut Vec<Warning>,
) -> Result<bool, Error> {
    let Some(data) = cff_data_and_top_dict(face)? else {
        return Ok(false);
    };
    let Some(cff) = cff_font(face)? else {
        return Ok(false);
    };
    let entries = dict_entries(data.top_dict, "CFF/CFF2 Top DICT")?;
    for entry in &entries {
        if let dict::Entry::UniqueId(value) = entry {
            info.postscript_unique_id = Some(*value);
        }
    }
    let ranges = private_ranges(data.data, &entries, data.is_cff2)?;
    let mut has_hints = false;
    for (index, range) in ranges.into_iter().enumerate() {
        let private = data.data.get(range).ok_or_else(|| Error::Parse {
            table: "CFF/CFF2 Private DICT",
            reason: "private dictionary range is out of bounds".into(),
        })?;
        let mut target = PsPrivate::default();
        for entry in dict_entries(private, "CFF/CFF2 Private DICT")? {
            match entry {
                dict::Entry::BlueValues(v) => target.blue_values = flatten_blues(&v),
                dict::Entry::OtherBlues(v) => target.other_blues = flatten_blues(&v),
                dict::Entry::FamilyBlues(v) => target.family_blues = flatten_blues(&v),
                dict::Entry::FamilyOtherBlues(v) => target.family_other_blues = flatten_blues(&v),
                dict::Entry::BlueScale(v) => target.blue_scale = Some(v.to_f64()),
                dict::Entry::BlueShift(v) => target.blue_shift = Some(v.to_f64()),
                dict::Entry::BlueFuzz(v) => target.blue_fuzz = Some(v.to_f64()),
                dict::Entry::StdHw(v) => target.std_hw = Some(v.to_f64()),
                dict::Entry::StdVw(v) => target.std_vw = Some(v.to_f64()),
                dict::Entry::StemSnapH(v) => {
                    target.stem_snap_h = v.values().iter().map(|v| v.to_f64()).collect()
                }
                dict::Entry::StemSnapV(v) => {
                    target.stem_snap_v = v.values().iter().map(|v| v.to_f64()).collect()
                }
                dict::Entry::ForceBold(v) => target.force_bold = Some(v),
                dict::Entry::LanguageGroup(v) => target.language_group = u8::try_from(v).ok(),
                dict::Entry::DefaultWidthX(v) if index == 0 => {
                    info.postscript_default_width_x = Some(v.to_f64())
                }
                dict::Entry::NominalWidthX(v) if index == 0 => {
                    info.postscript_nominal_width_x = Some(v.to_f64())
                }
                _ => {}
            }
        }
        has_hints |= target != PsPrivate::default();
        let subfont = cff
            .subfont(index as u16, &[])
            .map_err(|e| Error::parse("CFF/CFF2 Font DICT", e))?;
        if !normalize(
            &mut target,
            effective_transform(cff.transform(&subfont, None)),
        ) {
            warnings.push(Warning {
                code: "cff-private-matrix-unsupported",
                message: format!("CFF private dictionary {index} has a rotated, reflected or skewed FontMatrix; its private hint parameters were omitted because they cannot be represented in normalized font coordinates."),
            });
            target = PsPrivate::default();
        }
        if index == 0 {
            master.private_dict = target;
        } else {
            master.cff_private_dicts.push(target);
        }
    }
    Ok(has_hints)
}

fn private_ranges(
    data: &[u8],
    entries: &[dict::Entry],
    cff2: bool,
) -> Result<Vec<Range<usize>>, Error> {
    if let Some(range) = entries.iter().find_map(private_range) {
        return Ok(vec![range]);
    }
    let Some(offset) = entries.iter().find_map(|entry| match entry {
        dict::Entry::FdArrayOffset(v) => Some(*v),
        _ => None,
    }) else {
        return Ok(Vec::new());
    };
    let index_data = data.get(offset..).ok_or_else(|| Error::Parse {
        table: "CFF/CFF2 FDArray",
        reason: "FDArray offset is out of bounds".into(),
    })?;
    let array = Index::new(index_data, cff2).map_err(|e| Error::parse("CFF/CFF2 FDArray", e))?;
    if !cff2 && array.count() > 256 {
        return Err(Error::Parse {
            table: "CFF FDArray",
            reason: "CFF1 supports at most 256 Font DICTs".into(),
        });
    }
    // CFF2 private variation data remains outside the static CID preservation model.
    let count = if cff2 {
        array.count().min(1)
    } else {
        array.count()
    };
    (0..count)
        .map(|index| {
            let fd = array
                .get(index as usize)
                .map_err(|e| Error::parse("CFF/CFF2 Font DICT", e))?;
            Ok(dict_entries(fd, "CFF/CFF2 Font DICT")?
                .iter()
                .find_map(private_range)
                .unwrap_or(0..0))
        })
        .collect()
}

fn private_range(entry: &dict::Entry) -> Option<Range<usize>> {
    match entry {
        dict::Entry::PrivateDictRange(range) => Some(range.clone()),
        _ => None,
    }
}

fn flatten_blues(values: &read_fonts::ps::hinting::Blues) -> Vec<f64> {
    values
        .values()
        .iter()
        .flat_map(|(a, b)| [a.to_f64(), b.to_f64()])
        .collect()
}

fn normalize(values: &mut PsPrivate, transform: Affine) -> bool {
    if *values == PsPrivate::default() {
        return true;
    }
    let [xx, yx, xy, yy, _, dy] = transform.as_coeffs();
    if !transform.as_coeffs().iter().all(|v| v.is_finite())
        || xx <= 0.0
        || yy <= 0.0
        || xy != 0.0
        || yx != 0.0
    {
        return false;
    }
    for array in [
        &mut values.blue_values,
        &mut values.other_blues,
        &mut values.family_blues,
        &mut values.family_other_blues,
    ] {
        for value in array {
            *value = *value * yy + dy;
        }
    }
    for value in &mut values.stem_snap_h {
        *value *= yy;
    }
    for value in &mut values.stem_snap_v {
        *value *= xx;
    }
    values.std_hw = values.std_hw.map(|v| v * yy);
    values.std_vw = values.std_vw.map(|v| v * xx);
    if yy != 1.0 {
        // The default thresholds are expressed in source glyph coordinates too.
        values.blue_scale = Some(values.blue_scale.unwrap_or(0.039625) / yy);
        values.blue_shift = Some(values.blue_shift.unwrap_or(7.0) * yy);
        values.blue_fuzz = Some(values.blue_fuzz.unwrap_or(1.0) * yy);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_private_zones_widths_and_implicit_thresholds() {
        let mut values = PsPrivate {
            blue_values: vec![-10.0, 0.0, 700.0, 710.0],
            std_hw: Some(40.0),
            std_vw: Some(60.0),
            stem_snap_h: vec![40.0, 50.0],
            stem_snap_v: vec![60.0, 70.0],
            ..PsPrivate::default()
        };
        assert!(normalize(
            &mut values,
            Affine::new([2.0, 0.0, 0.0, 3.0, 7.0, 9.0])
        ));
        assert_eq!(values.blue_values, vec![-21.0, 9.0, 2109.0, 2139.0]);
        assert_eq!(values.std_hw, Some(120.0));
        assert_eq!(values.std_vw, Some(120.0));
        assert_eq!(values.stem_snap_h, vec![120.0, 150.0]);
        assert_eq!(values.stem_snap_v, vec![120.0, 140.0]);
        assert_eq!(values.blue_scale, Some(0.039625 / 3.0));
        assert_eq!(values.blue_shift, Some(21.0));
        assert_eq!(values.blue_fuzz, Some(3.0));
    }

    #[test]
    fn rejects_unrepresentable_private_matrices_without_mutating_values() {
        let original = PsPrivate {
            std_vw: Some(60.0),
            ..PsPrivate::default()
        };
        for transform in [
            Affine::new([-1.0, 0.0, 0.0, 1.0, 0.0, 0.0]),
            Affine::new([1.0, 0.0, 0.2, 1.0, 0.0, 0.0]),
        ] {
            let mut values = original.clone();
            assert!(!normalize(&mut values, transform));
            assert_eq!(values, original);
        }
    }
}
