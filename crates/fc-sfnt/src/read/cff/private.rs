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

/// Whether a Private DICT blends any of its values across variation regions.
fn blends(private: &[u8]) -> bool {
    dict::tokens(private)
        .any(|token| matches!(token, Ok(dict::Token::Operator(dict::Operator::Blend))))
}

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
    // A CFF2 Private DICT may blend every numeric value. Resolving the blends at
    // the default location keeps the imported values meaningful instead of
    // failing the whole import, and the caller discloses that the variation data
    // itself is flattened.
    let store = data
        .is_cff2
        .then(|| variation_store(data.data, &entries))
        .transpose()?
        .flatten();
    let mut has_hints = false;
    for (index, range) in ranges.into_iter().enumerate() {
        let private = data.data.get(range).ok_or_else(|| Error::Parse {
            table: "CFF/CFF2 Private DICT",
            reason: "private dictionary range is out of bounds".into(),
        })?;
        if blends(private) {
            warnings.push(Warning {
                code: "cff2-private-variation-flattened",
                message: "The CFF2 private dictionary varies across the design space; its values were imported at the default location, and a re-export writes variation computed from this document's masters".into(),
            });
        }
        let mut target = PsPrivate::default();
        for entry in private_entries(private, store.clone())? {
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

/// Private DICT byte ranges in Font DICT order, or one range for a plain CFF.
pub(super) fn ranges(face: &FontRef<'_>) -> Result<Vec<Range<usize>>, Error> {
    let Some(data) = super::cff_data_and_top_dict(face)? else {
        return Ok(Vec::new());
    };
    let entries = dict_entries(data.top_dict, "CFF/CFF2 Top DICT")?;
    private_ranges(data.data, &entries, data.is_cff2)
}

/// The local subroutine index one Private DICT declares, if it declares any.
///
/// `Subrs` is stored as an offset from the start of the Private DICT.
pub(super) fn local_subrs<'a>(
    data: &'a [u8],
    range: &Range<usize>,
) -> Option<read_fonts::ps::cff::index::Index<'a>> {
    let private = data.get(range.clone())?;
    let offset = dict::entries(private, None)
        .filter_map(Result::ok)
        .find_map(|entry| match entry {
            dict::Entry::SubrsOffset(offset) => Some(offset),
            _ => None,
        })?;
    read_fonts::ps::cff::index::Index::new(data.get(range.start + offset..)?, false).ok()
}

/// The CFF2 item variation store a Top DICT points at, when it has one.
fn variation_store<'a>(
    data: &'a [u8],
    entries: &[dict::Entry],
) -> Result<Option<read_fonts::tables::variations::ItemVariationStore<'a>>, Error> {
    let Some(offset) = entries.iter().find_map(|entry| match entry {
        dict::Entry::VariationStoreOffset(offset) => Some(*offset),
        _ => None,
    }) else {
        return Ok(None);
    };
    // The CFF2 store is preceded by its own two-byte length.
    let bytes = data.get(offset + 2..).ok_or_else(|| Error::Parse {
        table: "CFF2 VariationStore",
        reason: "variation store offset is out of bounds".into(),
    })?;
    read_fonts::FontRead::read(read_fonts::FontData::new(bytes))
        .map(Some)
        .map_err(|error| Error::parse("CFF2 VariationStore", error))
}

/// Private DICT entries, resolving CFF2 blends at the default location.
fn private_entries<'a>(
    private: &'a [u8],
    store: Option<read_fonts::tables::variations::ItemVariationStore<'a>>,
) -> Result<Vec<dict::Entry>, Error> {
    let blend = match store {
        Some(store) => Some(
            read_fonts::ps::cff::blend::BlendState::new(store, &[], 0)
                .map_err(|error| Error::parse("CFF2 VariationStore", error))?,
        ),
        None => None,
    };
    dict::entries(private, blend)
        .map(|entry| entry.map_err(|error| Error::parse("CFF/CFF2 Private DICT", error)))
        .collect()
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
