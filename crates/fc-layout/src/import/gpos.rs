//! Checked static GPOS positioning subtable decompilation.
//!
//! Implements the OpenType GPOS formats, independently of FontForge.

use fc_model::GlyphName;

use super::binary::{class, glyph, Reader, Result, Rules, MAX_RULES};

#[path = "gpos_marks.rs"]
mod marks;

// Bound the size of repeated class spellings as well as the number of rules.
const MAX_GLYPH_REFERENCES: usize = MAX_RULES * 16;

/// Read one supported static GPOS subtable into bounded source rules.
pub(super) fn parse(
    reader: Reader<'_>,
    kind: u16,
    names: &[GlyphName],
    lookup_name: &str,
) -> Result<Rules> {
    match kind {
        1 => single(reader, names),
        2 => pair(reader, names),
        3 => marks::cursive(reader, names),
        4..=6 => marks::attachment(reader, kind, names, lookup_name),
        _ => Err(format!("unsupported simple positioning lookup type {kind}")),
    }
}

#[derive(Clone, Copy)]
struct Value {
    values: [i16; 4],
}

impl Value {
    fn read(reader: Reader<'_>, offset: usize, format: u16) -> Result<Self> {
        value_size(format)?;
        let mut values = [0; 4];
        let mut cursor = offset;
        for (bit, value) in values.iter_mut().enumerate() {
            if format & (1 << bit) != 0 {
                *value = reader.i16(cursor)?;
                cursor += 2;
            }
        }
        Ok(Self { values })
    }

    fn fea(self) -> String {
        let [x, y, advance_x, advance_y] = self.values;
        format!("<{x} {y} {advance_x} {advance_y}>")
    }
}

fn value_size(format: u16) -> Result<usize> {
    if format & !0x0f != 0 {
        return Err(
            "device, variation or reserved positioning value fields are unsupported".into(),
        );
    }
    Ok(format.count_ones() as usize * 2)
}

fn single(reader: Reader<'_>, names: &[GlyphName]) -> Result<Rules> {
    let coverage = reader.offset16(2)?.coverage(names)?;
    let format = reader.u16(4)?;
    let size = value_size(format)?;
    let mut rules = Rules::default();
    match reader.u16(0)? {
        1 => {
            let value = Value::read(reader, 6, format)?.fea();
            for gid in coverage {
                rules.push(format!("pos {} {value};", glyph(names, gid)?), vec![gid])?;
            }
        }
        2 => {
            if reader.u16(6)? as usize != coverage.len() {
                return Err("single positioning count does not match coverage".into());
            }
            reader.bytes(8, coverage.len() * size)?;
            for (index, gid) in coverage.into_iter().enumerate() {
                let value = Value::read(reader, 8 + index * size, format)?.fea();
                rules.push(format!("pos {} {value};", glyph(names, gid)?), vec![gid])?;
            }
        }
        _ => return Err("unsupported single positioning format".into()),
    }
    Ok(rules)
}

fn pair(reader: Reader<'_>, names: &[GlyphName]) -> Result<Rules> {
    let coverage = reader.offset16(2)?.coverage(names)?;
    let format1 = reader.u16(4)?;
    let format2 = reader.u16(6)?;
    let size1 = value_size(format1)?;
    let size2 = value_size(format2)?;
    let values = PairValues {
        format1,
        format2,
        size1,
        size2,
    };
    match reader.u16(0)? {
        1 => pair_glyphs(reader, names, coverage, values),
        2 => pair_classes(reader, names, coverage, values),
        _ => Err("unsupported pair positioning format".into()),
    }
}

#[derive(Clone, Copy)]
struct PairValues {
    format1: u16,
    format2: u16,
    size1: usize,
    size2: usize,
}

impl PairValues {
    fn size(self) -> usize {
        self.size1 + self.size2
    }

    fn statement(
        self,
        reader: Reader<'_>,
        offset: usize,
        first: &str,
        second: &str,
    ) -> Result<String> {
        let value1 = Value::read(reader, offset, self.format1)?.fea();
        let value2 = Value::read(reader, offset + self.size1, self.format2)?;
        // Nonzero ValueFormat2 changes the next position at which the lookup
        // continues, even if all its encoded values are zero. fea-rs retains
        // explicit PairPos record-two zeros; omitting it preserves format zero.
        let second_value = if self.format2 == 0 {
            String::new()
        } else {
            format!(" {}", value2.fea())
        };
        Ok(format!("pos {first} {value1} {second}{second_value};"))
    }
}

fn pair_glyphs(
    reader: Reader<'_>,
    names: &[GlyphName],
    coverage: Vec<u16>,
    values: PairValues,
) -> Result<Rules> {
    if reader.u16(8)? as usize != coverage.len() {
        return Err("pair-set count does not match coverage".into());
    }
    let mut rules = Rules::default();
    for (index, first) in coverage.into_iter().enumerate() {
        let set = reader.offset16(10 + index * 2)?;
        let count = set.u16(0)? as usize;
        if count > MAX_RULES - rules.statements.len() {
            return Err("pair positioning rule resource limit".into());
        }
        set.bytes(2, count * (2 + values.size()))?;
        let first_name = glyph(names, first)?;
        let mut previous = None;
        for index in 0..count {
            let offset = 2 + index * (2 + values.size());
            let second = set.u16(offset)?;
            if previous.is_some_and(|prev| prev >= second) {
                return Err("pair-set second glyph IDs are not strictly ordered".into());
            }
            previous = Some(second);
            let statement =
                values.statement(set, offset + 2, &first_name, &glyph(names, second)?)?;
            rules.push(statement, vec![first, second])?;
        }
    }
    Ok(rules)
}

fn pair_classes(
    reader: Reader<'_>,
    names: &[GlyphName],
    coverage: Vec<u16>,
    values: PairValues,
) -> Result<Rules> {
    let classes1 = reader.offset16(8)?.classes(names)?;
    let classes2 = reader.offset16(10)?.classes(names)?;
    let count1 = reader.u16(12)? as usize;
    let count2 = reader.u16(14)? as usize;
    let count = count1
        .checked_mul(count2)
        .ok_or("pair class count overflow")?;
    if count > MAX_RULES {
        return Err("pair class matrix resource limit".into());
    }
    if count1 == 0
        || count2 == 0
        || classes1.iter().any(|id| *id as usize >= count1)
        || classes2.iter().any(|id| *id as usize >= count2)
    {
        return Err("pair class index outside declared matrix".into());
    }
    reader.bytes(16, count * values.size())?;
    let mut first_groups = vec![Vec::new(); count1];
    for gid in coverage {
        first_groups[classes1[gid as usize] as usize].push(gid);
    }
    let mut second_groups = vec![Vec::new(); count2];
    for (gid, class) in classes2.into_iter().enumerate() {
        second_groups[class as usize].push(gid as u16);
    }
    let mut references = 0;
    let mut rules = Rules::default();
    for (first_id, first) in first_groups.iter().enumerate() {
        for (second_id, second) in second_groups.iter().enumerate() {
            let offset = 16 + (first_id * count2 + second_id) * values.size();
            if first.is_empty() || second.is_empty() {
                // Empty classes still occupy cells in the rectangular matrix;
                // in particular class zero has a row even with no covered
                // members. Canonical zero padding has no rule to preserve, but
                // nonzero stored values must never disappear during conversion.
                let first_value = Value::read(reader, offset, values.format1)?;
                let second_value = Value::read(reader, offset + values.size1, values.format2)?;
                if first_value.values != [0; 4] || second_value.values != [0; 4] {
                    return Err(
                        "unreachable nonzero pair class value cannot be retained in FEA".into(),
                    );
                }
                continue;
            }
            references += first.len() + second.len();
            if references > MAX_GLYPH_REFERENCES {
                return Err("pair class glyph-reference resource limit".into());
            }
            let statement = values.statement(
                reader,
                offset,
                &class(names, first)?,
                &class(names, second)?,
            )?;
            rules.push(statement, first.iter().chain(second).copied().collect())?;
        }
    }
    Ok(rules)
}

#[cfg(test)]
#[path = "gpos_tests.rs"]
mod tests;
