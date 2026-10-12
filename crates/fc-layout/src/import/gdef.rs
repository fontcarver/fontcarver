//! Static GDEF fields that survive recompilation without contour point mapping.

use std::collections::BTreeMap;

use fc_model::GlyphName;

use super::binary::{class, glyph, members, Reader, Result, MAX_RULES};

/// Source declarations and lookup-flag dependencies retained from GDEF.
#[derive(Clone, Debug, Default)]
pub(super) struct GdefData {
    /// Named classes consumed by decompiled lookup flags.
    pub preamble: Vec<String>,
    /// Explicit glyph classes and coordinate ligature carets.
    pub table_source: String,
    /// Original mark attachment IDs and their glyph membership.
    pub mark_attach: BTreeMap<u16, Vec<u16>>,
    /// Mark filtering sets in their original index order.
    pub mark_sets: Vec<Vec<u16>>,
}

/// Decompile GDEF, refusing data whose meaning the pinned compiler cannot retain.
pub(super) fn parse(reader: Reader<'_>, names: &[GlyphName]) -> Result<GdefData> {
    let version = reader.u32(0)?;
    if !matches!(version, 0x0001_0000 | 0x0001_0002 | 0x0001_0003) {
        return Err("unsupported GDEF version".into());
    }
    reader.bytes(
        0,
        if version == 0x0001_0003 {
            18
        } else if version == 0x0001_0002 {
            14
        } else {
            12
        },
    )?;
    if version == 0x0001_0003 && reader.u32(14)? != 0 {
        return Err("GDEF item variation store is unsupported".into());
    }
    if reader.u16(6)? != 0 {
        return Err(
            "GDEF attachment point references require an output outline point mapping".into(),
        );
    }
    let mut result = GdefData::default();
    let mut statements = Vec::new();
    if reader.u16(4)? != 0 {
        let classes = reader.offset16(4)?.classes(names)?;
        if classes.iter().any(|id| *id > 4) {
            return Err("invalid GDEF glyph class".into());
        }
        let entries = (1..=4)
            .map(|id| {
                let glyphs = members(&classes, id);
                if glyphs.is_empty() {
                    Ok(String::new())
                } else {
                    class(names, &glyphs)
                }
            })
            .collect::<Result<Vec<_>>>()?;
        statements.push(format!("GlyphClassDef {};", entries.join(", ")));
    }
    if reader.u16(8)? != 0 {
        statements.extend(carets(reader.offset16(8)?, names)?);
    }
    if reader.u16(10)? != 0 {
        for (gid, id) in reader.offset16(10)?.classes(names)?.into_iter().enumerate() {
            if id > 255 {
                return Err("GDEF mark attachment class exceeds lookup flag range".into());
            }
            if id != 0 {
                result.mark_attach.entry(id).or_default().push(gid as u16);
            }
        }
        for (id, glyphs) in &result.mark_attach {
            result
                .preamble
                .push(format!("@FC_MARK_ATTACH_{id} = {};", class(names, glyphs)?));
        }
    }
    if version >= 0x0001_0002 && reader.u16(12)? != 0 {
        let sets = reader.offset16(12)?;
        if sets.u16(0)? != 1 {
            return Err("unsupported GDEF mark filtering set format".into());
        }
        let count = usize::from(sets.u16(2)?);
        sets.bytes(4, count * 4)?;
        let mut total = 0usize;
        for i in 0..count {
            let offset = sets.u32(4 + i * 4)? as usize;
            if offset == 0 {
                return Err("null GDEF mark filtering set offset".into());
            }
            let glyphs = sets.at(offset)?.coverage(names)?;
            total += glyphs.len();
            if total > MAX_RULES {
                return Err("GDEF mark filtering set resource limit".into());
            }
            result
                .preamble
                .push(format!("@FC_MARK_FILTER_{i} = {};", class(names, &glyphs)?));
            result.mark_sets.push(glyphs);
        }
    }
    if !statements.is_empty() {
        result.table_source = format!("table GDEF {{\n{}\n}} GDEF;", statements.join("\n"));
    }
    Ok(result)
}

fn carets(reader: Reader<'_>, names: &[GlyphName]) -> Result<Vec<String>> {
    let coverage = reader.offset16(0)?.coverage(names)?;
    let count = usize::from(reader.u16(2)?);
    reader.bytes(4, count * 2)?;
    if count != coverage.len() {
        return Err("GDEF ligature caret count does not match coverage".into());
    }
    let mut statements = Vec::new();
    let mut total = 0usize;
    for (i, gid) in coverage.into_iter().enumerate() {
        let ligature = reader.offset16(4 + i * 2)?;
        let count = usize::from(ligature.u16(0)?);
        if count == 0 {
            return Err("empty GDEF ligature caret record is not representable".into());
        }
        total += count;
        if total > MAX_RULES {
            return Err("GDEF ligature caret resource limit".into());
        }
        ligature.bytes(2, count * 2)?;
        let mut coordinates = Vec::with_capacity(count);
        for j in 0..count {
            let value = ligature.offset16(2 + j * 2)?;
            match value.u16(0)? {
                1 => {}
                2 => {
                    return Err(
                        "GDEF caret point references require an output outline point mapping"
                            .into(),
                    )
                }
                3 if value.u16(4)? == 0 => {}
                3 => return Err("GDEF caret device or variation adjustment is unsupported".into()),
                _ => return Err("unsupported GDEF caret value format".into()),
            }
            coordinates.push(value.i16(2)?);
        }
        // fea-rs sorts caret coordinates; reject input that it would reorder.
        if coordinates.windows(2).any(|pair| pair[0] > pair[1]) {
            return Err("unsorted GDEF caret coordinates cannot retain their order".into());
        }
        statements.push(format!(
            "LigatureCaretByPos {} {};",
            glyph(names, gid)?,
            coordinates
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }
    Ok(statements)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fc_compile::{compile_static_with_metadata, CompileOptions};
    use fc_model::{FeatureBlock, Font, Glyph, Layer};
    use read_fonts::{types::Tag, FontRef, TableProvider};

    fn words(values: &[u16]) -> Vec<u8> {
        values
            .iter()
            .flat_map(|value| value.to_be_bytes())
            .collect()
    }
    fn names() -> Vec<GlyphName> {
        [".notdef", "A", "f_i", "acute", "component"]
            .into_iter()
            .map(GlyphName::from)
            .collect()
    }

    #[test]
    fn retains_all_nonzero_glyph_classes_and_coordinate_carets() {
        let bytes = words(&[
            1, 0, 12, 0, 28, 0, 1, 0, 5, 0, 1, 2, 3, 4, 6, 1, 12, 1, 1, 2, 2, 6, 10, 1, 200, 1, 420,
        ]);
        let data = parse(Reader::new(&bytes), &names()).unwrap();
        assert!(data
            .table_source
            .contains("GlyphClassDef [\\A], [\\f_i], [\\acute], [\\component];"));
        assert!(data
            .table_source
            .contains("LigatureCaretByPos \\f_i 200 420;"));
        assert!(!data.table_source.contains(".notdef"));
    }

    #[test]
    fn returns_lookup_flag_dependencies_with_original_indices() {
        let bytes = words(&[1, 2, 0, 0, 0, 14, 24, 1, 3, 2, 7, 7, 1, 1, 0, 8, 1, 2, 3, 4]);
        let data = parse(Reader::new(&bytes), &names()).unwrap();
        assert_eq!(data.mark_attach[&7], vec![3, 4]);
        assert_eq!(data.mark_sets, vec![vec![3, 4]]);
        assert_eq!(
            data.preamble,
            vec![
                "@FC_MARK_ATTACH_7 = [\\acute \\component];",
                "@FC_MARK_FILTER_0 = [\\acute \\component];"
            ]
        );
    }

    #[test]
    fn blocks_point_references_devices_variations_and_malformed_data() {
        for (bytes, error) in [
            (words(&[1, 0, 0, 12, 0, 0, 0]), "point mapping"),
            (words(&[1, 3, 0, 0, 0, 0, 0, 0, 18, 0]), "variation store"),
            (
                words(&[1, 0, 0, 0, 12, 0, 6, 1, 12, 1, 1, 2, 1, 4, 2, 7]),
                "point mapping",
            ),
            (
                words(&[1, 0, 0, 0, 12, 0, 6, 1, 12, 1, 1, 2, 1, 4, 3, 200, 6, 0]),
                "device",
            ),
            (words(&[1, 0, 12, 0, 0, 0, 1, 1, 1, 5]), "glyph class"),
            (words(&[1, 0, 12, 0, 0, 0]), "truncated"),
        ] {
            let result = parse(Reader::new(&bytes), &names()).unwrap_err();
            assert!(result.contains(error), "{result}");
        }
    }

    #[test]
    fn null_device_carets_are_static_and_truncated_tables_are_rejected() {
        let bytes = words(&[1, 0, 0, 0, 12, 0, 6, 1, 12, 1, 1, 2, 1, 4, 3, 200, 0]);
        assert!(parse(Reader::new(&bytes), &names())
            .unwrap()
            .table_source
            .contains("LigatureCaretByPos \\f_i 200;"));
        for len in 0..bytes.len() {
            assert!(
                parse(Reader::new(&bytes[..len]), &names()).is_err(),
                "{len}"
            );
        }
    }

    fn compile(source: &str) -> GdefData {
        let mut font = Font::new_empty("GDEF roundtrip", 1_000);
        let master = font.default_master().id.clone();
        for name in names() {
            let mut glyph = Glyph::new(name);
            glyph.layers.push(Layer::new_master(master.clone()));
            font.insert_glyph(glyph).unwrap();
        }
        font.features.prefixes.push(FeatureBlock {
            code: source.into(),
            ..FeatureBlock::default()
        });
        let output =
            compile_static_with_metadata(&font, &master, &CompileOptions::default()).unwrap();
        let face = FontRef::new(&output.bytes).unwrap();
        let table = face.data_for_tag(Tag::new(b"GDEF")).unwrap();
        parse(Reader::new(table.as_bytes()), &output.glyph_order).unwrap()
    }

    #[test]
    fn pinned_compiler_roundtrip_retains_classes_carets_and_used_mark_dependencies() {
        let source = "@marks = [acute];
            table GDEF {
              GlyphClassDef [A], [f_i], [acute], [component];
              LigatureCaretByPos f_i -20 300;
            } GDEF;
            lookup Attach { lookupflag MarkAttachmentType @marks; pos A 10; } Attach;
            lookup Filter { lookupflag UseMarkFilteringSet @marks; pos A 20; } Filter;
            feature kern { lookup Attach; lookup Filter; } kern;";
        let original = compile(source);
        let roundtrip = compile(&format!(
            "{}\n{}\n\
             lookup Attach {{ lookupflag MarkAttachmentType @FC_MARK_ATTACH_1; pos A 10; }} Attach;\n\
             lookup Filter {{ lookupflag UseMarkFilteringSet @FC_MARK_FILTER_0; pos A 20; }} Filter;\n\
             feature kern {{ lookup Attach; lookup Filter; }} kern;",
            original.preamble.join("\n"), original.table_source
        ));
        assert_eq!(original.table_source, roundtrip.table_source);
        assert_eq!(original.mark_attach, roundtrip.mark_attach);
        assert_eq!(original.mark_sets, roundtrip.mark_sets);
    }
}
