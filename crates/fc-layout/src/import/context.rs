//! Static GSUB/GPOS contextual rules, read from OpenType common table formats.
//!
//! Backtrack arrays are stored nearest-first. FEA writes them in text order.
//! Reference: https://learn.microsoft.com/en-us/typography/opentype/spec/chapter2

use fc_model::GlyphName;

use std::collections::BTreeMap;

use super::binary::{class, glyph, Reader, Result, Rules, MAX_RULES};

// Prevent repeated coverage/class offsets from expanding small input tables
// into gigabytes of source. The limit counts input and context glyph members.
const MAX_GLYPH_REFERENCES: usize = 1_000_000;

type Classes = BTreeMap<u16, Vec<u16>>;

#[derive(Default)]
struct Sequence {
    backtrack: Vec<Vec<u16>>,
    input: Vec<Vec<u16>>,
    lookahead: Vec<Vec<u16>>,
    records: Vec<(u16, u16)>,
}

struct Parser<'a> {
    names: &'a [GlyphName],
    positioning: bool,
    table: &'a str,
    remaining: usize,
    rules: Rules,
}

/// Decompile one contextual or chained contextual subtable.
pub(super) fn parse(
    reader: Reader<'_>,
    chained: bool,
    positioning: bool,
    names: &[GlyphName],
    table: &str,
) -> Result<Rules> {
    if table != if positioning { "GPOS" } else { "GSUB" } {
        return Err("context lookup table mismatch".into());
    }
    let mut parser = Parser {
        names,
        positioning,
        table,
        remaining: MAX_GLYPH_REFERENCES,
        rules: Rules::default(),
    };
    match reader.u16(0)? {
        1 => parser.by_glyph(reader, chained)?,
        2 => parser.by_class(reader, chained)?,
        3 => parser.by_coverage(reader, chained)?,
        _ => return Err("unsupported contextual subtable format".into()),
    }
    Ok(parser.rules)
}

impl Parser<'_> {
    fn set(&mut self, values: Vec<u16>) -> Result<Vec<u16>> {
        if values.is_empty() {
            return Err("empty contextual glyph set cannot be represented in FEA".into());
        }
        self.remaining = self
            .remaining
            .checked_sub(values.len())
            .ok_or("context glyph expansion resource limit")?;
        for &gid in &values {
            glyph(self.names, gid)?;
        }
        Ok(values)
    }

    fn by_glyph(&mut self, reader: Reader<'_>, chained: bool) -> Result<()> {
        let coverage = reader.offset16(2)?.coverage(self.names)?;
        let count = usize::from(reader.u16(4)?);
        if coverage.len() != count {
            return Err("context rule set count differs from coverage".into());
        }
        let offsets = reader.array(6, count)?;
        for (gid, offset) in coverage.into_iter().zip(offsets) {
            if offset == 0 {
                continue;
            }
            let set = reader.at(usize::from(offset))?;
            for offset in set.array(2, usize::from(set.u16(0)?))? {
                if offset == 0 {
                    return Err("context rule offset is null".into());
                }
                let rule = set.at(usize::from(offset))?;
                let sequence = self.rule(rule, vec![gid], chained, None)?;
                self.emit(sequence)?;
            }
        }
        Ok(())
    }

    fn by_class(&mut self, reader: Reader<'_>, chained: bool) -> Result<()> {
        let coverage = reader.offset16(2)?.coverage(self.names)?;
        let (back, input, ahead, count_at) = if chained {
            (
                classes(reader.offset16(4)?.classes(self.names)?),
                classes(reader.offset16(6)?.classes(self.names)?),
                classes(reader.offset16(8)?.classes(self.names)?),
                10,
            )
        } else {
            (
                Classes::new(),
                classes(reader.offset16(4)?.classes(self.names)?),
                Classes::new(),
                6,
            )
        };
        let count = usize::from(reader.u16(count_at)?);
        if input.keys().any(|id| usize::from(*id) >= count) {
            return Err("input class exceeds context class-set count".into());
        }
        for (id, offset) in reader.array(count_at + 2, count)?.into_iter().enumerate() {
            if offset == 0 {
                continue;
            }
            let first = input
                .get(&(id as u16))
                .into_iter()
                .flatten()
                .filter(|gid| coverage.binary_search(gid).is_ok())
                .copied()
                .collect::<Vec<_>>();
            let set = reader.at(usize::from(offset))?;
            for offset in set.array(2, usize::from(set.u16(0)?))? {
                if offset == 0 {
                    return Err("context class rule offset is null".into());
                }
                let rule = set.at(usize::from(offset))?;
                let sequence =
                    self.rule(rule, first.clone(), chained, Some([&back, &input, &ahead]))?;
                self.emit(sequence)?;
            }
        }
        Ok(())
    }

    fn rule(
        &mut self,
        reader: Reader<'_>,
        first: Vec<u16>,
        chained: bool,
        definitions: Option<[&Classes; 3]>,
    ) -> Result<Sequence> {
        let mut result = Sequence::default();
        let mut at = 0;
        if chained {
            result.backtrack = self.values(reader, &mut at, definitions.map(|defs| defs[0]), 0)?;
        }
        let input_count = usize::from(reader.u16(at)?);
        if input_count == 0 {
            return Err("context input sequence is empty".into());
        }
        let record_count = if chained {
            None
        } else {
            Some(reader.u16(at + 2)?)
        };
        at += if chained { 2 } else { 4 };
        result.input.push(self.set(first)?);
        for value in reader.array(at, input_count - 1)? {
            let gids = resolve(value, definitions.map(|defs| defs[1]))?;
            result.input.push(self.set(gids)?);
        }
        at += (input_count - 1) * 2;
        if chained {
            result.lookahead = self.values(reader, &mut at, definitions.map(|defs| defs[2]), 0)?;
        }
        let count = match record_count {
            Some(count) => count,
            None => {
                let count = reader.u16(at)?;
                at += 2;
                count
            }
        };
        result.records = records(reader, at, count, input_count)?;
        Ok(result)
    }

    fn values(
        &mut self,
        reader: Reader<'_>,
        at: &mut usize,
        definitions: Option<&Classes>,
        minimum: usize,
    ) -> Result<Vec<Vec<u16>>> {
        let count = usize::from(reader.u16(*at)?);
        *at += 2;
        if count < minimum {
            return Err("context input sequence is empty".into());
        }
        let mut result = Vec::with_capacity(count);
        for value in reader.array(*at, count)? {
            result.push(self.set(resolve(value, definitions)?)?);
        }
        *at += count * 2;
        Ok(result)
    }

    fn coverages(
        &mut self,
        reader: Reader<'_>,
        at: &mut usize,
        count: usize,
    ) -> Result<Vec<Vec<u16>>> {
        let mut result = Vec::with_capacity(count);
        for field in (*at..*at + count * 2).step_by(2) {
            let gids = reader.offset16(field)?.coverage(self.names)?;
            result.push(self.set(gids)?);
        }
        *at += count * 2;
        Ok(result)
    }

    fn by_coverage(&mut self, reader: Reader<'_>, chained: bool) -> Result<()> {
        let mut result = Sequence::default();
        let mut at = 2;
        if chained {
            let count = usize::from(reader.u16(at)?);
            at += 2;
            result.backtrack = self.coverages(reader, &mut at, count)?;
        }
        let count = usize::from(reader.u16(at)?);
        if count == 0 {
            return Err("context input sequence is empty".into());
        }
        at += 2;
        let record_count = if chained {
            None
        } else {
            let count = reader.u16(at)?;
            at += 2;
            Some(count)
        };
        result.input = self.coverages(reader, &mut at, count)?;
        if chained {
            let count = usize::from(reader.u16(at)?);
            at += 2;
            result.lookahead = self.coverages(reader, &mut at, count)?;
        }
        let record_count = match record_count {
            Some(count) => count,
            None => {
                let count = reader.u16(at)?;
                at += 2;
                count
            }
        };
        result.records = records(reader, at, record_count, result.input.len())?;
        self.emit(result)
    }

    fn emit(&mut self, sequence: Sequence) -> Result<()> {
        let mut tokens = Vec::new();
        let mut gids = Vec::new();
        for set in sequence.backtrack.iter().rev() {
            tokens.push(atom(self.names, set)?);
            gids.extend(set);
        }
        let mut records = sequence.records.iter().peekable();
        for (index, set) in sequence.input.iter().enumerate() {
            tokens.push(format!("{}'", atom(self.names, set)?));
            gids.extend(set);
            while records
                .peek()
                .is_some_and(|(position, _)| usize::from(*position) == index)
            {
                if let Some((_, lookup)) = records.next() {
                    tokens.push(format!("lookup FC_{}_{lookup}", self.table));
                    self.rules.references.push(*lookup);
                }
            }
        }
        for set in &sequence.lookahead {
            tokens.push(atom(self.names, set)?);
            gids.extend(set);
        }
        if self.rules.references.len() > MAX_RULES {
            return Err("context lookup reference resource limit".into());
        }
        let verb = if self.positioning { "pos" } else { "sub" };
        let ignore = if sequence.records.is_empty() {
            "ignore "
        } else {
            ""
        };
        self.rules
            .push(format!("{ignore}{verb} {};", tokens.join(" ")), gids)
    }
}

fn classes(values: Vec<u16>) -> Classes {
    let mut result = Classes::new();
    for (gid, id) in values.into_iter().enumerate() {
        result.entry(id).or_default().push(gid as u16);
    }
    result
}

fn atom(names: &[GlyphName], gids: &[u16]) -> Result<String> {
    // fea-rs 1.0's context-format-1 optimizer requires actual glyph atoms:
    // singleton bracket classes enter the same path and panic in to_glyph().
    match gids {
        [gid] => glyph(names, *gid),
        _ => class(names, gids),
    }
}

fn resolve(value: u16, definitions: Option<&Classes>) -> Result<Vec<u16>> {
    match definitions {
        None => Ok(vec![value]),
        Some(classes) => classes
            .get(&value)
            .cloned()
            .ok_or_else(|| "context references an empty glyph class".into()),
    }
}

fn records(
    reader: Reader<'_>,
    at: usize,
    count: u16,
    input_count: usize,
) -> Result<Vec<(u16, u16)>> {
    reader.bytes(at, usize::from(count) * 4)?;
    let mut result = Vec::with_capacity(usize::from(count));
    let mut previous = None;
    for index in 0..usize::from(count) {
        let position = reader.u16(at + index * 4)?;
        let lookup = reader.u16(at + index * 4 + 2)?;
        if usize::from(position) >= input_count {
            return Err("context lookup sequence index is out of range".into());
        }
        if previous.is_some_and(|previous| previous > position) {
            return Err("context lookup execution order cannot be represented in FEA".into());
        }
        previous = Some(position);
        result.push((position, lookup));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(words: &[u16]) -> Vec<u8> {
        words.iter().flat_map(|word| word.to_be_bytes()).collect()
    }

    fn names() -> Vec<GlyphName> {
        [".notdef", "a", "b", "c", "d"]
            .into_iter()
            .map(GlyphName::from)
            .collect()
    }

    #[test]
    fn format_three_preserves_all_input_positions_and_reference_order() {
        let data = bytes(&[3, 2, 2, 18, 24, 0, 3, 1, 4, 1, 1, 1, 1, 1, 2]);
        let result = parse(Reader::new(&data), false, false, &names(), "GSUB").unwrap();
        assert_eq!(
            result.statements,
            ["sub \\a' lookup FC_GSUB_3 \\b' lookup FC_GSUB_4;"]
        );
        assert_eq!(result.references, [3, 4]);
    }

    #[test]
    fn ignore_rules_still_mark_the_entire_input() {
        let data = bytes(&[3, 2, 0, 10, 16, 1, 1, 1, 1, 1, 2]);
        let result = parse(Reader::new(&data), false, true, &names(), "GPOS").unwrap();
        assert_eq!(result.statements, ["ignore pos \\a' \\b';"]);
        assert!(result.references.is_empty());
    }

    #[test]
    fn descending_lookup_record_order_is_rejected_instead_of_reordered() {
        let data = bytes(&[3, 2, 2, 18, 24, 1, 3, 0, 4, 1, 1, 1, 1, 1, 2]);
        let error = parse(Reader::new(&data), false, false, &names(), "GSUB").unwrap_err();
        assert!(error.contains("execution order"), "{error}");
    }

    #[test]
    fn context_format_one_uses_the_coverage_first_glyph() {
        let data = bytes(&[1, 22, 1, 8, 1, 4, 2, 1, 2, 1, 7, 1, 1, 1]);
        let result = parse(Reader::new(&data), false, false, &names(), "GSUB").unwrap();
        assert_eq!(result.statements, ["sub \\a' \\b' lookup FC_GSUB_7;"]);
    }

    #[test]
    fn context_class_zero_contains_unlisted_glyphs() {
        // Coverage = a. Class 1 = a. Rule class 1, class 0 -> lookup 2 at index 1.
        let data = bytes(&[
            2, 26, 32, 2, 0, 12, 1, 4, 2, 1, 0, 1, 2, 1, 1, 1, 1, 1, 1, 1,
        ]);
        let result = parse(Reader::new(&data), false, false, &names(), "GSUB").unwrap();
        assert_eq!(
            result.statements,
            ["sub \\a' [\\.notdef \\b \\c \\d]' lookup FC_GSUB_2;"]
        );
    }

    #[test]
    fn chaining_format_one_reverses_backtrack_order() {
        let data = bytes(&[1, 32, 1, 8, 1, 4, 2, 2, 1, 1, 1, 4, 1, 0, 8, 0, 1, 1, 3]);
        let result = parse(Reader::new(&data), true, false, &names(), "GSUB").unwrap();
        assert_eq!(
            result.statements,
            ["sub \\a \\b \\c' lookup FC_GSUB_8 \\d;"]
        );
    }

    fn append(words: &mut Vec<u16>, values: &[u16]) -> u16 {
        let offset = u16::try_from(words.len() * 2).unwrap();
        words.extend(values);
        offset
    }

    #[test]
    fn chaining_class_and_coverage_formats_preserve_both_contexts() {
        let mut class_words = vec![2, 0, 0, 0, 0, 2, 0, 0];
        class_words[7] = append(&mut class_words, &[1, 4]);
        append(&mut class_words, &[2, 2, 1, 1, 1, 2, 1, 0, 3]);
        class_words[1] = append(&mut class_words, &[1, 1, 3]);
        class_words[2] = append(&mut class_words, &[1, 1, 2, 1, 2]);
        class_words[3] = append(&mut class_words, &[1, 3, 1, 1]);
        class_words[4] = append(&mut class_words, &[1, 4, 1, 2]);

        let mut coverage_words = vec![3, 2, 0, 0, 1, 0, 1, 0, 1, 0, 3];
        coverage_words[2] = append(&mut coverage_words, &[1, 1, 2]);
        coverage_words[3] = append(&mut coverage_words, &[1, 1, 1]);
        coverage_words[5] = append(&mut coverage_words, &[1, 1, 3]);
        coverage_words[7] = append(&mut coverage_words, &[1, 1, 4]);

        for words in [class_words, coverage_words] {
            for (positioning, table, verb) in [(false, "GSUB", "sub"), (true, "GPOS", "pos")] {
                let data = bytes(&words);
                let result = parse(Reader::new(&data), true, positioning, &names(), table).unwrap();
                assert_eq!(
                    result.statements,
                    [format!("{verb} \\a \\b \\c' lookup FC_{table}_3 \\d;")]
                );
                assert_eq!(result.references, [3]);
                for end in 0..data.len() {
                    assert!(
                        parse(
                            Reader::new(&data[..end]),
                            true,
                            positioning,
                            &names(),
                            table
                        )
                        .is_err(),
                        "accepted format {} at {end}",
                        words[0]
                    );
                }
            }
        }
    }

    #[test]
    fn repeated_references_at_one_position_retain_order() {
        let data = bytes(&[3, 1, 2, 16, 0, 3, 0, 2, 1, 1, 1]);
        let result = parse(Reader::new(&data), false, false, &names(), "GSUB").unwrap();
        assert_eq!(
            result.statements,
            ["sub \\a' lookup FC_GSUB_3 lookup FC_GSUB_2;"]
        );
        assert_eq!(result.references, [3, 2]);
    }

    #[test]
    fn invalid_context_sequence_indices_and_empty_classes_are_rejected() {
        let data = bytes(&[3, 1, 1, 12, 1, 0, 1, 1, 1]);
        let error = parse(Reader::new(&data), false, false, &names(), "GSUB").unwrap_err();
        assert!(error.contains("sequence index"), "{error}");

        let data = bytes(&[
            2, 26, 32, 2, 0, 12, 1, 4, 2, 1, 8, 1, 2, 1, 1, 1, 1, 1, 1, 1,
        ]);
        let error = parse(Reader::new(&data), false, false, &names(), "GSUB").unwrap_err();
        assert!(error.contains("empty glyph class"), "{error}");
    }

    #[test]
    fn repeated_large_class_expansion_is_bounded() {
        let glyph_names: Vec<_> = (0..1000)
            .map(|index| GlyphName::from(format!("g{index}")))
            .collect();
        // Class 0 contains every glyph. Repeat it enough times to exceed the
        // total expansion budget, using a small shared class definition.
        let mut words = vec![2, 0, 0, 1, 10, 1, 4, 1002, 0];
        words.resize(words.len() + 1001, 0);
        words[1] = append(&mut words, &[1, 1, 0]);
        words[2] = append(&mut words, &[1, 0, 0]);
        let error = parse(
            Reader::new(&bytes(&words)),
            false,
            false,
            &glyph_names,
            "GSUB",
        )
        .unwrap_err();
        assert!(error.contains("resource limit"), "{error}");
    }

    #[test]
    fn every_truncated_context_is_rejected() {
        let data = bytes(&[3, 2, 2, 18, 24, 0, 3, 1, 4, 1, 1, 1, 1, 1, 2]);
        for end in 0..data.len() {
            assert!(
                parse(Reader::new(&data[..end]), false, false, &names(), "GSUB").is_err(),
                "accepted {end}"
            );
        }
    }
}
