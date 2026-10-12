//! Static substitution subtables expressed as deterministic feature source.
//!
//! Reference: https://learn.microsoft.com/en-us/typography/opentype/spec/gsub

use fc_model::GlyphName;

use super::binary::{class, glyph, Reader, Result, Rules};
use super::context;

const MAX_GLYPH_REFERENCES: usize = 1_000_000;

/// Decompile a non-extension GSUB subtable; extension wrappers are caller-owned.
pub(super) fn parse(
    reader: Reader<'_>,
    kind: u16,
    names: &[GlyphName],
    _lookup_name: &str,
) -> Result<Rules> {
    match kind {
        1 => single(reader, names),
        2 | 3 => sequence(reader, kind == 3, names),
        4 => ligature(reader, names),
        5 | 6 => context::parse(reader, kind == 6, false, names, "GSUB"),
        8 => reverse(reader, names),
        7 => Err("nested substitution extension lookup is unsupported".into()),
        _ => Err("unsupported GSUB lookup type".into()),
    }
}

fn single(reader: Reader<'_>, names: &[GlyphName]) -> Result<Rules> {
    let coverage = reader.offset16(2)?.coverage(names)?;
    let substitutes = match reader.u16(0)? {
        1 => {
            let delta = reader.u16(4)?;
            coverage.iter().map(|gid| gid.wrapping_add(delta)).collect()
        }
        2 => {
            let count = usize::from(reader.u16(4)?);
            if count != coverage.len() {
                return Err("single substitution count differs from coverage".into());
            }
            reader.array(6, count)?
        }
        _ => return Err("unsupported single substitution format".into()),
    };
    let mut result = Rules::default();
    for (from, to) in coverage.into_iter().zip(substitutes) {
        result.push(
            format!("sub {} by {};", glyph(names, from)?, glyph(names, to)?),
            vec![from, to],
        )?;
    }
    Ok(result)
}

fn sequence(reader: Reader<'_>, alternate: bool, names: &[GlyphName]) -> Result<Rules> {
    if reader.u16(0)? != 1 {
        return Err("unsupported multiple/alternate substitution format".into());
    }
    let coverage = reader.offset16(2)?.coverage(names)?;
    let count = usize::from(reader.u16(4)?);
    if count != coverage.len() {
        return Err("substitution sequence count differs from coverage".into());
    }
    reader.bytes(6, count * 2)?;
    let mut result = Rules::default();
    let mut remaining = MAX_GLYPH_REFERENCES;
    for (index, from) in coverage.into_iter().enumerate() {
        let sequence = reader.offset16(6 + index * 2)?;
        let count = usize::from(sequence.u16(0)?);
        remaining = remaining
            .checked_sub(count + 1)
            .ok_or("substitution expansion resource limit")?;
        let to = sequence.array(2, count)?;
        let action = if alternate {
            format!("from {}", class(names, &to)?)
        } else if to.is_empty() {
            "by NULL".to_owned()
        } else {
            format!(
                "by {}",
                to.iter()
                    .map(|gid| glyph(names, *gid))
                    .collect::<Result<Vec<_>>>()?
                    .join(" ")
            )
        };
        let mut gids = vec![from];
        gids.extend(to);
        result.push(format!("sub {} {action};", glyph(names, from)?), gids)?;
    }
    Ok(result)
}

fn ligature(reader: Reader<'_>, names: &[GlyphName]) -> Result<Rules> {
    if reader.u16(0)? != 1 {
        return Err("unsupported ligature substitution format".into());
    }
    let coverage = reader.offset16(2)?.coverage(names)?;
    let count = usize::from(reader.u16(4)?);
    if count != coverage.len() {
        return Err("ligature set count differs from coverage".into());
    }
    reader.bytes(6, count * 2)?;
    let mut result = Rules::default();
    let mut remaining = MAX_GLYPH_REFERENCES;
    for (index, first) in coverage.into_iter().enumerate() {
        let set = reader.offset16(6 + index * 2)?;
        let count = usize::from(set.u16(0)?);
        set.bytes(2, count * 2)?;
        let mut previous_length = usize::MAX;
        for index in 0..count {
            let rule = set.offset16(2 + index * 2)?;
            let to = rule.u16(0)?;
            let component_count = usize::from(rule.u16(2)?);
            if component_count < 2 {
                return Err("ligature has fewer than two components".into());
            }
            // The pinned writer sorts longest-first. Reject other preference
            // orders instead of silently reordering overlapping ligatures.
            if component_count > previous_length {
                return Err("ligature preference order cannot be retained by the compiler".into());
            }
            previous_length = component_count;
            remaining = remaining
                .checked_sub(component_count + 1)
                .ok_or("ligature expansion resource limit")?;
            let mut from = vec![first];
            from.extend(rule.array(4, component_count - 1)?);
            let source = from
                .iter()
                .map(|gid| glyph(names, *gid))
                .collect::<Result<Vec<_>>>()?
                .join(" ");
            let statement = format!("sub {source} by {};", glyph(names, to)?);
            from.push(to);
            result.push(statement, from)?;
        }
    }
    Ok(result)
}

fn reverse(reader: Reader<'_>, names: &[GlyphName]) -> Result<Rules> {
    if reader.u16(0)? != 1 {
        return Err("unsupported reverse chaining substitution format".into());
    }
    let input = reader.offset16(2)?.coverage(names)?;
    let mut remaining = MAX_GLYPH_REFERENCES;
    let mut at = 4;
    let backtrack = coverages(reader, names, &mut at, &mut remaining)?;
    let lookahead = coverages(reader, names, &mut at, &mut remaining)?;
    let count = usize::from(reader.u16(at)?);
    if count != input.len() {
        return Err("reverse substitution count differs from coverage".into());
    }
    let output = reader.array(at + 2, count)?;
    let mut tokens = Vec::new();
    let mut gids = Vec::new();
    for set in backtrack.iter().rev() {
        tokens.push(class(names, set)?);
        gids.extend(set);
    }
    tokens.push(format!("{}'", class(names, &input)?));
    gids.extend(input);
    for set in &lookahead {
        tokens.push(class(names, set)?);
        gids.extend(set);
    }
    let statement = format!("rsub {} by {};", tokens.join(" "), class(names, &output)?);
    gids.extend(output);
    let mut result = Rules::default();
    result.push(statement, gids)?;
    Ok(result)
}

fn coverages(
    reader: Reader<'_>,
    names: &[GlyphName],
    at: &mut usize,
    remaining: &mut usize,
) -> Result<Vec<Vec<u16>>> {
    let count = usize::from(reader.u16(*at)?);
    *at += 2;
    reader.bytes(*at, count * 2)?;
    let mut result = Vec::with_capacity(count);
    for field in (*at..*at + count * 2).step_by(2) {
        let coverage = reader.offset16(field)?.coverage(names)?;
        *remaining = remaining
            .checked_sub(coverage.len())
            .ok_or("reverse chaining expansion resource limit")?;
        if coverage.is_empty() {
            return Err("empty reverse chaining coverage cannot be represented in FEA".into());
        }
        result.push(coverage);
    }
    *at += count * 2;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fc_model::{FeatureBlock, FeaturesSource, Font, Glyph, Layer, Tag};

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
    fn single_delta_and_explicit_substitution_match() {
        for words in [&[1, 6, 1, 1, 1, 1][..], &[2, 8, 1, 2, 1, 1, 1]] {
            let data = bytes(words);
            assert_eq!(
                parse(Reader::new(&data), 1, &names(), "L")
                    .unwrap()
                    .statements,
                ["sub \\a by \\b;"]
            );
        }
    }

    #[test]
    fn multiple_and_alternate_preserve_output_order() {
        let data = bytes(&[1, 14, 1, 8, 2, 3, 2, 1, 1, 1]);
        assert_eq!(
            parse(Reader::new(&data), 2, &names(), "L")
                .unwrap()
                .statements,
            ["sub \\a by \\c \\b;"]
        );
        assert_eq!(
            parse(Reader::new(&data), 3, &names(), "L")
                .unwrap()
                .statements,
            ["sub \\a from [\\c \\b];"]
        );
    }

    #[test]
    fn multiple_deletion_is_explicit_and_empty_alternate_is_rejected() {
        let data = bytes(&[1, 10, 1, 8, 0, 1, 1, 1]);
        assert_eq!(
            parse(Reader::new(&data), 2, &names(), "L")
                .unwrap()
                .statements,
            ["sub \\a by NULL;"]
        );
        assert!(parse(Reader::new(&data), 3, &names(), "L").is_err());
    }

    #[test]
    fn singleton_multiple_substitution_cannot_silently_change_lookup_kind() {
        let words = vec![1, 12, 1, 8, 1, 2, 1, 1, 1];
        let rules = parse(Reader::new(&bytes(&words)), 2, &names(), "L").unwrap();
        assert_eq!(rules.statements, ["sub \\a by \\b;"]);
        // fea-rs emits GSUB1 when a complete GSUB2 lookup only has singleton
        // outputs. Do not weaken the whole-graph proof to accept that change.
        let tables =
            std::collections::BTreeMap::from([(Tag(*b"GSUB"), layout_table(&[(2, words)]))]);
        let import = super::super::LayoutImport::from_tables(&tables, &names());
        let candidate = font(import.decompile().unwrap());
        let exported = fc_compile::export(&candidate, &|_| 0, &Default::default())
            .unwrap()
            .bytes;
        assert!(super::super::equivalent_layout(&import, &exported, &names()).is_err());
    }

    #[test]
    fn ligature_includes_the_covered_first_component() {
        let data = bytes(&[1, 18, 1, 8, 1, 4, 3, 2, 2, 1, 1, 1]);
        assert_eq!(
            parse(Reader::new(&data), 4, &names(), "L")
                .unwrap()
                .statements,
            ["sub \\a \\b by \\c;"]
        );
    }

    #[test]
    fn reverse_chaining_reverses_backtrack_and_keeps_replacement_order() {
        let data = bytes(&[1, 18, 2, 26, 32, 0, 2, 3, 4, 1, 2, 1, 2, 1, 1, 3, 1, 1, 4]);
        assert_eq!(
            parse(Reader::new(&data), 8, &names(), "L")
                .unwrap()
                .statements,
            ["rsub [\\d] [\\c] [\\a \\b]' by [\\c \\d];"]
        );
    }

    #[test]
    fn mismatched_counts_out_of_range_glyphs_and_unknown_formats_are_rejected() {
        for (kind, words) in [
            (1, vec![2, 8, 2, 2, 1, 1, 1]),
            (1, vec![1, 6, 100, 1, 1, 1]),
            (1, vec![3, 6, 1, 1, 1, 1]),
            (7, vec![1, 1, 0, 8, 1, 6, 1, 1, 1, 1]),
        ] {
            assert!(parse(Reader::new(&bytes(&words)), kind, &names(), "L").is_err());
        }
    }

    fn font(source: String) -> Font {
        let mut font = Font::new_empty("GSUB decompiler", 1000);
        let master = font.default_master().id.clone();
        for name in names() {
            let mut glyph = Glyph::new(name.clone());
            if name.len() == 1 {
                glyph.unicodes.push(u32::from(name.as_bytes()[0]));
            }
            let mut layer = Layer::new_master(master.clone());
            layer.width = 500.0;
            glyph.layers.push(layer);
            font.insert_glyph(glyph).unwrap();
        }
        font.features.prefixes.push(FeatureBlock {
            code: source,
            ..FeatureBlock::default()
        });
        font
    }

    fn shape_source(source: String, text: &str) -> Vec<String> {
        let font = font(source);
        shape_font(&font, text)
            .names
            .into_iter()
            .map(Option::unwrap)
            .collect()
    }

    fn shape_font(font: &Font, text: &str) -> crate::ShapedRun {
        let shaper = crate::Shaper::new();
        let run = shaper
            .shape(
                font,
                &crate::FontKey {
                    doc: 1,
                    master: font.default_master().id.clone(),
                },
                1,
                &crate::ShapeRequest {
                    text: text.into(),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(run.warnings.is_empty(), "{:?}", run.warnings);
        run
    }

    #[test]
    fn generated_simple_substitutions_compile_and_shape_with_the_pinned_writer() {
        for (kind, words, text, expected) in [
            (1, vec![1, 6, 1, 1, 1, 1], "a", vec!["b"]),
            (2, vec![1, 14, 1, 8, 2, 3, 2, 1, 1, 1], "a", vec!["c", "b"]),
            (2, vec![1, 10, 1, 8, 0, 1, 1, 1], "ab", vec!["b"]),
            (3, vec![1, 14, 1, 8, 2, 3, 2, 1, 1, 1], "a", vec!["c"]),
            (
                4,
                vec![1, 18, 1, 8, 1, 4, 3, 2, 2, 1, 1, 1],
                "ab",
                vec!["c"],
            ),
            (
                8,
                vec![1, 18, 2, 26, 32, 0, 2, 3, 4, 1, 2, 1, 2, 1, 1, 3, 1, 1, 4],
                "dca",
                vec!["d", "c", "c"],
            ),
        ] {
            let rules = parse(Reader::new(&bytes(&words)), kind, &names(), "L").unwrap();
            let source = format!("feature calt {{ {} }} calt;", rules.statements.join("\n"));
            assert_eq!(shape_source(source, text), expected, "GSUB type {kind}");
        }
    }

    #[test]
    fn generated_context_and_class_zero_rules_compile_and_shape() {
        let fixtures = [
            (
                vec![1, 22, 1, 8, 1, 4, 2, 1, 2, 1, 0, 1, 1, 1],
                "ab",
                vec!["a", "c"],
            ),
            (
                vec![
                    2, 26, 32, 2, 0, 12, 1, 4, 2, 1, 0, 1, 0, 1, 1, 1, 1, 1, 1, 1,
                ],
                "ab",
                vec!["a", "c"],
            ),
            (
                vec![3, 2, 1, 14, 20, 1, 0, 1, 1, 1, 1, 1, 2],
                "ab",
                vec!["a", "c"],
            ),
        ];
        for (words, text, expected) in fixtures {
            let rules = context::parse(Reader::new(&bytes(&words)), false, false, &names(), "GSUB")
                .unwrap();
            let source = format!(
                "lookup FC_GSUB_0 {{ sub b by c; }} FC_GSUB_0; feature calt {{ {} }} calt;",
                rules.statements.join("\n")
            );
            assert_eq!(shape_source(source, text), expected);
        }
    }

    #[test]
    fn compiler_reordering_ligature_preferences_is_rejected() {
        let data = bytes(&[1, 28, 1, 8, 2, 6, 12, 3, 2, 2, 4, 3, 2, 3, 1, 1, 1]);
        let error = parse(Reader::new(&data), 4, &names(), "L").unwrap_err();
        assert!(error.contains("preference order"), "{error}");
    }

    fn layout_table(lookups: &[(u16, Vec<u16>)]) -> Vec<u8> {
        // DFLT/dflt activates the last lookup through calt. Preceding lookups
        // are contextual dependencies, so every fixture has complete reachability.
        let mut table = vec![
            1,
            0,
            10,
            30,
            44,
            1,
            0x4446,
            0x4c54,
            8,
            4,
            0,
            0,
            0xffff,
            1,
            0,
            1,
            0x6361,
            0x6c74,
            8,
            0,
            1,
            (lookups.len() - 1) as u16,
        ];
        let list_at = table.len();
        table.push(lookups.len() as u16);
        table.resize(table.len() + lookups.len(), 0);
        for (index, (kind, subtable)) in lookups.iter().enumerate() {
            table[list_at + index + 1] = ((table.len() - list_at) * 2) as u16;
            table.extend([*kind, 0, 1, 8]);
            table.extend(subtable);
        }
        bytes(&table)
    }

    fn roundtrip_layout(tag: [u8; 4], lookups: &[(u16, Vec<u16>)], samples: &[&str]) {
        roundtrip_registered_layout(tag, lookups, (lookups.len() - 1) as u16, samples);
    }

    fn roundtrip_registered_layout(
        tag: [u8; 4],
        lookups: &[(u16, Vec<u16>)],
        registered: u16,
        samples: &[&str],
    ) {
        let scaffold = fc_compile::export(&font(String::new()), &|_| 0, &Default::default())
            .unwrap()
            .bytes;
        let mut original = fc_sfnt::read_bytes(&scaffold, &Default::default())
            .unwrap()
            .fonts
            .remove(0);
        let mut table = layout_table(lookups);
        table[42..44].copy_from_slice(&registered.to_be_bytes());
        original.extra.raw_tables.insert(Tag(tag), table);
        original.features.source_of_truth = FeaturesSource::ImportedTables;
        let import = super::super::LayoutImport::from_font(&original);
        assert!(import.can_convert(), "{:?}", import.blockers);
        let source = import.decompile().unwrap();
        let candidate = font(source.clone());
        let exported = fc_compile::export(&candidate, &|_| 0, &Default::default())
            .unwrap()
            .bytes;
        super::super::equivalent_layout(&import, &exported, &names())
            .unwrap_or_else(|error| panic!("{error}\n{source}"));
        for text in samples {
            let before = shape_font(&original, text);
            let after = shape_font(&candidate, text);
            assert_eq!(before.names, after.names, "{text}: {source}");
            assert_eq!(before.glyphs, after.glyphs, "{text}: {source}");
        }
    }

    #[test]
    fn whole_gsub_graph_roundtrips_simple_types_through_actual_binary_writer() {
        for (kind, words) in [
            (1, vec![1, 6, 1, 1, 1, 1]),
            (1, vec![2, 8, 1, 2, 1, 1, 1]),
            (2, vec![1, 14, 1, 8, 2, 3, 2, 1, 1, 1]),
            (2, vec![1, 10, 1, 8, 0, 1, 1, 1]),
            (3, vec![1, 14, 1, 8, 2, 3, 2, 1, 1, 1]),
            (4, vec![1, 18, 1, 8, 1, 4, 3, 2, 2, 1, 1, 1]),
            (
                8,
                vec![1, 18, 2, 26, 32, 0, 2, 3, 4, 1, 2, 1, 2, 1, 1, 3, 1, 1, 4],
            ),
        ] {
            roundtrip_layout(
                *b"GSUB",
                &[(kind, words)],
                &["ab", "dca", "abcdab", "dcbdca"],
            );
        }
    }

    #[test]
    fn whole_context_graph_roundtrips_all_six_formats_for_substitution_and_positioning() {
        let context_rules = [
            (false, vec![1, 22, 1, 8, 1, 4, 2, 1, 2, 1, 0, 1, 1, 1]),
            (
                false,
                vec![
                    2, 26, 32, 2, 0, 12, 1, 4, 2, 1, 0, 1, 0, 1, 1, 1, 1, 1, 1, 1,
                ],
            ),
            (false, vec![3, 2, 1, 14, 20, 1, 0, 1, 1, 1, 1, 1, 2]),
            (
                true,
                vec![1, 32, 1, 8, 1, 4, 2, 2, 1, 1, 1, 4, 1, 0, 0, 0, 1, 1, 3],
            ),
            (true, chain_class()),
            (true, chain_coverage()),
        ];
        for (chained, words) in context_rules {
            for (tag, kind, dependency) in [
                (
                    *b"GSUB",
                    if chained { 6 } else { 5 },
                    vec![1, 6, 1, 1, 2, 2, 3],
                ),
                (
                    *b"GPOS",
                    if chained { 8 } else { 7 },
                    vec![1, 8, 4, (-50i16) as u16, 1, 2, 2, 3],
                ),
            ] {
                roundtrip_layout(
                    tag,
                    &[(1, dependency), (kind, words.clone())],
                    &["ab", "abcd", "ac", "aab", "dabcdab"],
                );
            }
        }
    }

    fn append(words: &mut Vec<u16>, values: &[u16]) -> u16 {
        let offset = u16::try_from(words.len() * 2).unwrap();
        words.extend(values);
        offset
    }

    fn chain_class() -> Vec<u16> {
        let mut words = vec![2, 0, 0, 0, 0, 2, 0, 0];
        words[7] = append(&mut words, &[1, 4]);
        append(&mut words, &[2, 2, 1, 1, 1, 2, 1, 0, 0]);
        words[1] = append(&mut words, &[1, 1, 3]);
        words[2] = append(&mut words, &[1, 1, 2, 1, 2]);
        words[3] = append(&mut words, &[1, 3, 1, 1]);
        words[4] = append(&mut words, &[1, 4, 1, 2]);
        words
    }

    fn chain_coverage() -> Vec<u16> {
        let mut words = vec![3, 2, 0, 0, 1, 0, 1, 0, 1, 0, 0];
        words[2] = append(&mut words, &[1, 1, 2]);
        words[3] = append(&mut words, &[1, 1, 1]);
        words[5] = append(&mut words, &[1, 1, 3]);
        words[7] = append(&mut words, &[1, 1, 4]);
        words
    }

    #[test]
    fn whole_context_graph_preserves_forward_and_nested_dependencies() {
        let context = vec![3, 2, 1, 14, 20, 1, 1, 1, 1, 1, 1, 1, 2];
        let single = vec![1, 6, 1, 1, 2, 2, 3];
        // Lookup 0 calls the later lookup 1. The only registered feature calls 0.
        roundtrip_registered_layout(
            *b"GSUB",
            &[(5, context.clone()), (1, single.clone())],
            0,
            &["ab", "ac", "abab"],
        );
        // Lookup 2 calls lookup 0, which calls the later lookup 1.
        let nested = vec![3, 2, 1, 14, 20, 0, 0, 1, 1, 1, 1, 1, 2];
        roundtrip_layout(
            *b"GSUB",
            &[(5, context), (1, single), (5, nested)],
            &["ab", "ac", "abab"],
        );
    }

    #[test]
    fn ignore_and_repeated_position_actions_keep_binary_execution_semantics() {
        let single = vec![1, 6, 1, 1, 2, 2, 3];
        let ignore_then_apply = vec![1, 30, 1, 8, 2, 6, 12, 2, 0, 2, 2, 1, 3, 1, 0, 1, 1, 1];
        roundtrip_layout(
            *b"GSUB",
            &[(1, single.clone()), (5, ignore_then_apply)],
            &["ab", "ac", "abac", "acac"],
        );
        let repeated = vec![3, 1, 2, 16, 0, 0, 0, 0, 1, 1, 2];
        roundtrip_layout(*b"GSUB", &[(1, single), (5, repeated)], &["b", "ab", "bb"]);
    }
}
