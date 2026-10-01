//! CFF2 encoding according to the OpenType CFF2 table specification.
//! <https://learn.microsoft.com/en-us/typography/opentype/spec/cff2>
use super::{
    encoding::{index, integer, number, offset, real},
    WriteError,
};
use kurbo::{BezPath, PathEl, Point, QuadBez};
use std::collections::HashMap;

/// A closed outline represented by relative CFF2 drawing operations.
#[derive(Clone, Debug, Default)]
pub struct Cff2Outline {
    /// One operator per move, line or cubic segment (21, 5 or 8).
    pub operators: Vec<u8>,
    /// Concatenated relative operands, with two per move/line and six per cubic.
    pub operands: Vec<f64>,
}

/// One interpolation region and its operand deltas.
#[derive(Clone, Debug)]
pub struct Cff2Delta {
    /// Normalized start, peak and end for each axis, in fvar order.
    pub region: Vec<[f64; 3]>,
    /// Relative operand adjustments in the same order as the default outline.
    pub values: Vec<f64>,
}

/// Private DICT values for a CFF2 font and their per-region variation.
///
/// CFF2 keeps the PostScript private parameters, but every numeric value may
/// vary. The default-instance values are written as ordinary operands and any
/// value that varies is written through the DICT `blend` operator (23), whose
/// operand order is the default values followed by each value's region deltas
/// (CFF2 specification, "Private DICT Operators").
#[derive(Clone, Debug, Default)]
pub struct Cff2Private {
    /// Default-instance private values.
    pub values: fc_model::PsPrivate,
    /// Variation regions, encoded like [`Cff2Delta::region`].
    pub regions: Vec<Vec<[f64; 3]>>,
    /// One delta vector per region, laid out by [`private_layout`].
    ///
    /// Deltas are differences of the **absolute** values, as the variation model
    /// produces them. The writer converts each array operator's deltas into the
    /// relative basis its operands are stored in.
    pub deltas: Vec<Vec<f64>>,
}

/// The flat operand order [`Cff2Private::deltas`] uses.
///
/// The five blendable arrays in DICT order, then the blendable scalars
/// `BlueScale`, `BlueShift`, `BlueFuzz`, `StdHW` and `StdVW`. An empty array and
/// an absent scalar contribute nothing, so every master must agree on the array
/// lengths and on which scalars exist before its values can vary.
pub fn private_layout(values: &fc_model::PsPrivate) -> Vec<f64> {
    let mut result = Vec::new();
    for array in [
        &values.blue_values,
        &values.other_blues,
        &values.family_blues,
        &values.family_other_blues,
        &values.stem_snap_h,
        &values.stem_snap_v,
    ] {
        result.extend_from_slice(array);
    }
    result.extend(
        [
            values.blue_scale,
            values.blue_shift,
            values.blue_fuzz,
            values.std_hw,
            values.std_vw,
        ]
        .into_iter()
        .flatten(),
    );
    result
}

/// One glyph's default outline and sparse variation deltas.
#[derive(Clone, Debug)]
pub struct Cff2Glyph {
    /// Default-source geometry.
    pub outline: Cff2Outline,
    /// Non-default deltas in their interpolation model's region order.
    pub deltas: Vec<Cff2Delta>,
}

/// Convert a closed path to relative cubic drawing operands without rounding.
pub fn cff2_outline(path: &BezPath) -> Result<Cff2Outline, WriteError> {
    let mut result = Cff2Outline::default();
    let mut current = Point::ZERO;
    let mut source = Point::ZERO;
    let mut start = Point::ZERO;
    let mut open = false;
    for element in path.elements() {
        let (operator, points) = match *element {
            PathEl::MoveTo(p) => {
                if open {
                    return Err(WriteError("CFF2 requires closed contours".into()));
                }
                start = p;
                source = p;
                open = true;
                (21, vec![p])
            }
            PathEl::LineTo(p) => {
                if !open {
                    return Err(WriteError("line outside contour".into()));
                }
                source = p;
                (5, vec![p])
            }
            PathEl::CurveTo(a, b, c) => {
                if !open {
                    return Err(WriteError("curve outside contour".into()));
                }
                source = c;
                (8, vec![a, b, c])
            }
            PathEl::QuadTo(a, b) => {
                if !open {
                    return Err(WriteError("curve outside contour".into()));
                }
                let curve = QuadBez::new(source, a, b).raise();
                source = b;
                (8, vec![curve.p1, curve.p2, curve.p3])
            }
            PathEl::ClosePath => {
                open = false;
                source = start;
                continue;
            }
        };
        result.operators.push(operator);
        for point in points {
            if !point.x.is_finite() || !point.y.is_finite() {
                return Err(WriteError("non-finite CFF2 outline".into()));
            }
            result
                .operands
                .extend([point.x - current.x, point.y - current.y]);
            current = point;
        }
    }
    if open {
        return Err(WriteError("CFF2 requires closed contours".into()));
    }
    Ok(result)
}

/// Write a CFF2 table with deduplicated regions and per-glyph region subsets.
/// Glyphs must already be arranged in the enclosing font's glyph-ID order.
pub fn write_cff2(
    upem: u16,
    axis_count: usize,
    glyphs: &[Cff2Glyph],
    private: &Cff2Private,
) -> Result<Vec<u8>, WriteError> {
    if !(16..=16384).contains(&upem)
        || axis_count == 0
        || axis_count > 16
        || glyphs.is_empty()
        || glyphs.len() > 65535
    {
        return Err(WriteError("invalid CFF2 dimensions".into()));
    }
    let mut regions = Vec::<Vec<[i16; 3]>>::new();
    let mut region_map = HashMap::new();
    let mut groups = Vec::<Vec<u16>>::new();
    let mut group_map = HashMap::new();
    // The Private DICT's blends use the variation-data group its own `vsindex`
    // selects, which defaults to zero and is also the default for this Font
    // DICT's charstrings. Registering the private regions first makes that group
    // zero, so no charstring has to be told which group it uses unless it really
    // differs.
    if !private.regions.is_empty() {
        let indices = private
            .regions
            .iter()
            .map(|region| region_index(region, axis_count, &mut regions, &mut region_map))
            .collect::<Result<Vec<_>, _>>()?;
        group_map.insert(indices.clone(), 0u16);
        groups.push(indices);
    }
    let mut programs = Vec::new();
    let mut work = 0usize;
    let mut program_bytes = 0usize;
    for glyph in glyphs {
        work = work.saturating_add(
            glyph
                .outline
                .operands
                .len()
                .saturating_mul(glyph.deltas.len() + 1),
        );
        if work > 64_000_000 {
            return Err(WriteError("CFF2 operand work limit".into()));
        }
        if glyph
            .outline
            .operators
            .first()
            .is_some_and(|operator| *operator != 21)
        {
            return Err(WriteError("CFF2 contour must begin with moveto".into()));
        }
        if glyph.deltas.len() > 505 {
            return Err(WriteError(
                "CFF2 glyph exceeds 505 active variation regions".into(),
            ));
        }
        let mut indices = Vec::new();
        for delta in &glyph.deltas {
            if delta.values.len() != glyph.outline.operands.len()
                || delta.region.len() != axis_count
            {
                return Err(WriteError("inconsistent CFF2 delta dimensions".into()));
            }
            indices.push(region_index(
                &delta.region,
                axis_count,
                &mut regions,
                &mut region_map,
            )?);
        }
        let next = u16::try_from(groups.len())
            .map_err(|_| WriteError("too many CFF2 region subsets".into()))?;
        let group = *group_map.entry(indices.clone()).or_insert_with(|| {
            groups.push(indices);
            next
        });
        let mut program = Vec::new();
        if group > 0 {
            number(&mut program, f64::from(group))?;
            program.push(15);
        }
        let mut operand = 0;
        for operator in &glyph.outline.operators {
            let count = match operator {
                21 | 5 => 2,
                8 => 6,
                _ => return Err(WriteError("unsupported CFF2 drawing operator".into())),
            };
            for _ in 0..count {
                let value = *glyph
                    .outline
                    .operands
                    .get(operand)
                    .ok_or_else(|| WriteError("missing CFF2 operand".into()))?;
                number(&mut program, value)?;
                if glyph.deltas.iter().any(|d| d.values[operand] != 0.0) {
                    for delta in &glyph.deltas {
                        number(&mut program, delta.values[operand])?;
                    }
                    integer(&mut program, 1);
                    program.push(16);
                }
                operand += 1;
            }
            program.push(*operator);
            if program.len() > 65_535 {
                return Err(WriteError("CFF2 charstring exceeds 65535 bytes".into()));
            }
        }
        if operand != glyph.outline.operands.len() {
            return Err(WriteError("extra CFF2 operands".into()));
        }
        program_bytes = program_bytes.saturating_add(program.len());
        if program_bytes > 256_000_000 {
            return Err(WriteError("CFF2 total charstring work limit".into()));
        }
        programs.push(program);
    }
    let varstore = variation_store(axis_count, &regions, &groups)?;
    let charstrings = index32(&programs)?;
    let private_dict = private_dict(private)?;
    // Fixed-width offsets keep every DICT length independent of the positions.
    let fontdict = index32(&[font_dict(0, 0)?])?;
    let top_size = top_dict(upem, 0, 0, 0)?.len();
    let cs_offset = 5 + top_size + 4;
    let fd_offset = cs_offset + charstrings.len();
    let vs_offset = fd_offset + fontdict.len();
    let private_offset = vs_offset + varstore.len();
    let fontdict = index32(&[font_dict(private_dict.len(), private_offset)?])?;
    let top = top_dict(upem, cs_offset, fd_offset, vs_offset)?;
    let mut bytes = vec![2, 0, 5];
    bytes.extend_from_slice(&(top.len() as u16).to_be_bytes());
    bytes.extend(top);
    bytes.extend([0; 4]);
    bytes.extend(charstrings);
    bytes.extend(fontdict);
    bytes.extend(varstore);
    bytes.extend(private_dict);
    Ok(bytes)
}

/// Registers one interpolation region, returning its index in the shared store.
fn region_index(
    region: &[[f64; 3]],
    axis_count: usize,
    regions: &mut Vec<Vec<[i16; 3]>>,
    region_map: &mut HashMap<Vec<[i16; 3]>, u16>,
) -> Result<u16, WriteError> {
    if region.len() != axis_count {
        return Err(WriteError("inconsistent CFF2 delta dimensions".into()));
    }
    let encoded: Vec<_> = region
        .iter()
        .map(|tent| {
            if tent
                .iter()
                .any(|v| !v.is_finite() || !(-1.0..=1.0).contains(v))
                || tent[0] > tent[1]
                || tent[1] > tent[2]
            {
                return Err(WriteError("invalid CFF2 variation region".into()));
            }
            let value = tent.map(|v| (v * 16384.0).round() as i16);
            if (tent[0] < 0.0 && tent[2] > 0.0 && tent[1] != 0.0)
                || (tent[1] != 0.0 && value[1] == 0)
                || (tent[1] == 0.0 && *tent != [0.0; 3])
            {
                return Err(WriteError(
                    "CFF2 region cannot be represented as F2DOT14".into(),
                ));
            }
            Ok(value)
        })
        .collect::<Result<_, _>>()?;
    if let Some(index) = region_map.get(&encoded) {
        return Ok(*index);
    }
    let next =
        u16::try_from(regions.len()).map_err(|_| WriteError("too many CFF2 regions".into()))?;
    region_map.insert(encoded.clone(), next);
    regions.push(encoded);
    Ok(next)
}

fn font_dict(size: usize, position: usize) -> Result<Vec<u8>, WriteError> {
    let mut bytes = Vec::new();
    offset(&mut bytes, size)?;
    offset(&mut bytes, position)?;
    bytes.push(18); // Private
    Ok(bytes)
}

/// Writes the CFF2 Private DICT, blending any value that varies.
fn private_dict(private: &Cff2Private) -> Result<Vec<u8>, WriteError> {
    private
        .values
        .validate()
        .map_err(|error| WriteError(error.to_string()))?;
    let defaults = private_layout(&private.values);
    let masters = private.regions.len() + 1;
    for deltas in &private.deltas {
        if deltas.len() != defaults.len() {
            return Err(WriteError(
                "CFF2 private variation deltas must match the default value layout".into(),
            ));
        }
    }
    if private.deltas.len() != private.regions.len() {
        return Err(WriteError(
            "CFF2 private variation needs one delta vector per region".into(),
        ));
    }
    let mut bytes = Vec::new();
    let mut at = 0usize;
    let blended = |bytes: &mut Vec<u8>,
                   operands: &[f64],
                   deltas: &[Vec<f64>],
                   opcode: &[u8]|
     -> Result<(), WriteError> {
        let varies = deltas.iter().any(|row| row.iter().any(|v| *v != 0.0));
        if varies && operands.len() * masters + 1 > 500 {
            return Err(WriteError(
                "CFF2 private value cannot be blended across this many masters".into(),
            ));
        }
        for value in operands {
            dict_number(bytes, *value)?;
        }
        if varies {
            for row in deltas {
                for value in row {
                    dict_number(bytes, *value)?;
                }
            }
            dict_number(bytes, operands.len() as f64)?;
            bytes.push(23); // blend
        }
        bytes.extend_from_slice(opcode);
        Ok(())
    };
    for (array, opcode) in [
        (&private.values.blue_values, &[6][..]),
        (&private.values.other_blues, &[7]),
        (&private.values.family_blues, &[8]),
        (&private.values.family_other_blues, &[9]),
        (&private.values.stem_snap_h, &[12, 12]),
        (&private.values.stem_snap_v, &[12, 13]),
    ] {
        if array.is_empty() {
            continue;
        }
        // `blend` works on the operand stack, and an array operator's operands
        // are each stored relative to the previous one. The default operands and
        // the deltas therefore both have to be in that relative basis: a reader
        // blends the relative operands first and only then takes their prefix
        // sum. Taking the first difference of a delta row is exactly the same
        // linear map that relativises the default values, so applying it to
        // every row converts `deltas`, which are differences of the absolute
        // values, into the operand basis. This is what FontTools writes in
        // `varLib/cff.py`, where `getDeltas` is given the relative rows.
        let relative = relativise(array);
        let rows: Vec<Vec<f64>> = private
            .deltas
            .iter()
            .map(|row| relativise(&row[at..at + array.len()]))
            .collect();
        // Transposed per value: all regions for value one, then value two.
        let mut grouped = Vec::with_capacity(array.len());
        for index in 0..array.len() {
            grouped.push(rows.iter().map(|row| row[index]).collect::<Vec<f64>>());
        }
        blended(&mut bytes, &relative, &grouped, opcode)?;
        at += array.len();
    }
    for (value, opcode) in [
        (private.values.blue_scale, &[12, 9][..]),
        (private.values.blue_shift, &[12, 10]),
        (private.values.blue_fuzz, &[12, 11]),
        (private.values.std_hw, &[10]),
        (private.values.std_vw, &[11]),
    ] {
        let Some(value) = value else { continue };
        let row: Vec<Vec<f64>> = vec![private.deltas.iter().map(|row| row[at]).collect()];
        blended(&mut bytes, &[value], &row, opcode)?;
        at += 1;
    }
    // LanguageGroup selects a hinting mode rather than a measurement, so CFF2
    // does not allow it to vary; the default instance's value is authoritative.
    if let Some(group) = private.values.language_group {
        dict_number(&mut bytes, f64::from(group))?;
        bytes.extend_from_slice(&[12, 17]);
    }
    Ok(bytes)
}

/// Rewrites a run of values as the differences a DICT array operator stores.
fn relativise(values: &[f64]) -> Vec<f64> {
    let mut previous = 0.0;
    values
        .iter()
        .map(|value| {
            let relative = *value - previous;
            previous = *value;
            relative
        })
        .collect()
}

fn dict_number(bytes: &mut Vec<u8>, value: f64) -> Result<(), WriteError> {
    if value.fract() == 0.0 && (i32::MIN as f64..=i32::MAX as f64).contains(&value) {
        integer(bytes, value as i32);
        Ok(())
    } else {
        real(bytes, value)
    }
}

fn index32(objects: &[Vec<u8>]) -> Result<Vec<u8>, WriteError> {
    let encoded = index(objects)?;
    let mut result = (objects.len() as u32).to_be_bytes().to_vec();
    result.extend_from_slice(&encoded[2..]);
    Ok(result)
}

fn top_dict(upem: u16, cs: usize, fd: usize, vs: usize) -> Result<Vec<u8>, WriteError> {
    let mut bytes = Vec::new();
    offset(&mut bytes, cs)?;
    bytes.push(17);
    offset(&mut bytes, fd)?;
    bytes.extend([12, 36]);
    offset(&mut bytes, vs)?;
    bytes.push(24);
    if upem != 1000 {
        for value in [
            1.0 / f64::from(upem),
            0.0,
            0.0,
            1.0 / f64::from(upem),
            0.0,
            0.0,
        ] {
            real(&mut bytes, value)?;
        }
        bytes.extend([12, 7]);
    }
    Ok(bytes)
}

fn variation_store(
    axes: usize,
    regions: &[Vec<[i16; 3]>],
    groups: &[Vec<u16>],
) -> Result<Vec<u8>, WriteError> {
    let region_offset = 8 + 4 * groups.len();
    let mut group_offset = region_offset + 4 + 6 * axes * regions.len();
    let mut store = vec![0, 1];
    store.extend_from_slice(&(region_offset as u32).to_be_bytes());
    store.extend_from_slice(&(groups.len() as u16).to_be_bytes());
    for group in groups {
        store.extend_from_slice(&(group_offset as u32).to_be_bytes());
        group_offset += 6 + 2 * group.len();
    }
    store.extend_from_slice(&(axes as u16).to_be_bytes());
    store.extend_from_slice(&(regions.len() as u16).to_be_bytes());
    for region in regions {
        for tent in region {
            for value in tent {
                store.extend_from_slice(&value.to_be_bytes());
            }
        }
    }
    for group in groups {
        store.extend([0; 4]);
        store.extend_from_slice(&(group.len() as u16).to_be_bytes());
        for index in group {
            store.extend_from_slice(&index.to_be_bytes());
        }
    }
    let length = u16::try_from(store.len())
        .map_err(|_| WriteError("CFF2 variation store exceeds 65,535 bytes".into()))?;
    let mut bytes = length.to_be_bytes().to_vec();
    bytes.extend(store);
    Ok(bytes)
}
