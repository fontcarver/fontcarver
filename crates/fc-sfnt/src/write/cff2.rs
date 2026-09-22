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
            let region: Vec<_> = delta
                .region
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
                    let encoded = tent.map(|v| (v * 16384.0).round() as i16);
                    if (tent[0] < 0.0 && tent[2] > 0.0 && tent[1] != 0.0)
                        || (tent[1] != 0.0 && encoded[1] == 0)
                        || (tent[1] == 0.0 && *tent != [0.0; 3])
                    {
                        return Err(WriteError(
                            "CFF2 region cannot be represented as F2DOT14".into(),
                        ));
                    }
                    Ok(encoded)
                })
                .collect::<Result<_, _>>()?;
            if regions.len() >= usize::from(u16::MAX) && !region_map.contains_key(&region) {
                return Err(WriteError("too many CFF2 regions".into()));
            }
            let next = regions.len() as u16;
            let index = *region_map.entry(region.clone()).or_insert_with(|| {
                regions.push(region);
                next
            });
            indices.push(index);
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
    let fontdict = index32(&[vec![139, 139, 18]])?; // Empty PrivateDICT: size and offset are zero.
    let top_size = top_dict(upem, 0, 0, 0)?.len();
    let cs_offset = 5 + top_size + 4;
    let fd_offset = cs_offset + charstrings.len();
    let vs_offset = fd_offset + fontdict.len();
    let top = top_dict(upem, cs_offset, fd_offset, vs_offset)?;
    let mut bytes = vec![2, 0, 5];
    bytes.extend_from_slice(&(top.len() as u16).to_be_bytes());
    bytes.extend(top);
    bytes.extend([0; 4]);
    bytes.extend(charstrings);
    bytes.extend(fontdict);
    bytes.extend(varstore);
    Ok(bytes)
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
