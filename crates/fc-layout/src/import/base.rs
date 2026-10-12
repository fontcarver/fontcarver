//! Checked decompilation of static BASE axis coordinates.

use fc_model::GlyphName;

use super::binary::{Reader, Result, MAX_RULES};

/// Produce FEA BASE source, refusing unsupported coordinate and MinMax data.
pub(super) fn parse(reader: Reader<'_>, _names: &[GlyphName]) -> Result<String> {
    let version = reader.u32(0)?;
    if !matches!(version, 0x0001_0000 | 0x0001_0001) {
        return Err("unsupported BASE version".into());
    }
    reader.bytes(0, if version == 0x0001_0001 { 12 } else { 8 })?;
    if version == 0x0001_0001 && reader.u32(8)? != 0 {
        return Err("BASE item variation store is unsupported".into());
    }
    let mut statements = Vec::new();
    let mut total = 0usize;
    for (field, name) in [(4, "HorizAxis"), (6, "VertAxis")] {
        if reader.u16(field)? != 0 {
            statements.extend(axis(reader.offset16(field)?, name, &mut total)?);
        }
    }
    if statements.is_empty() {
        return Err("BASE without axes is not representable by the pinned compiler".into());
    }
    Ok(format!(
        "table BASE {{\n{}\n}} BASE;",
        statements.join("\n")
    ))
}

fn axis(reader: Reader<'_>, name: &str, total: &mut usize) -> Result<Vec<String>> {
    let tags = reader
        .offset16(0)
        .map_err(|_| "BASE axis needs a baseline tag list")?;
    let count = usize::from(tags.u16(0)?);
    tags.bytes(2, count * 4)?;
    if count == 0 {
        return Err("empty BASE baseline tag list is not representable".into());
    }
    let tags = (0..count)
        .map(|i| tag(tags, 2 + i * 4))
        .collect::<Result<Vec<_>>>()?;
    if tags.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err("BASE baseline tags are not strictly ordered".into());
    }
    let scripts = reader.offset16(2)?;
    let count = usize::from(scripts.u16(0)?);
    scripts.bytes(2, count * 6)?;
    if count == 0 {
        return Err("empty BASE script list is not representable".into());
    }
    let mut records = Vec::with_capacity(count);
    let mut previous = String::new();
    for i in 0..count {
        let script_tag = tag(scripts, 2 + i * 6)?;
        if previous >= script_tag {
            return Err("BASE scripts are not strictly ordered".into());
        }
        previous = script_tag.clone();
        let script = scripts.offset16(6 + i * 6)?;
        if script.u16(2)? != 0 || script.u16(4)? != 0 {
            return Err("BASE default or language MinMax data is unsupported".into());
        }
        let values = script.offset16(0)?;
        let default = usize::from(values.u16(0)?);
        let count = usize::from(values.u16(2)?);
        if count != tags.len() {
            return Err("BASE coordinate count does not match baseline tags".into());
        }
        let default = tags
            .get(default)
            .ok_or("BASE default baseline index is out of range")?;
        values.bytes(4, count * 2)?;
        *total += count;
        if *total > MAX_RULES {
            return Err("BASE coordinate resource limit".into());
        }
        let mut coordinates = Vec::with_capacity(count);
        for j in 0..count {
            let value = values.offset16(4 + j * 2)?;
            match value.u16(0)? {
                1 => {}
                2 => {
                    return Err(
                        "BASE coordinate point references require an output outline point mapping"
                            .into(),
                    )
                }
                3 if value.u16(4)? == 0 => {}
                3 => {
                    return Err(
                        "BASE coordinate device or variation adjustment is unsupported".into(),
                    )
                }
                _ => return Err("unsupported BASE coordinate format".into()),
            }
            coordinates.push(value.i16(2)?.to_string());
        }
        records.push(format!("{script_tag} {default} {}", coordinates.join(" ")));
    }
    Ok(vec![
        format!("{name}.BaseTagList {};", tags.join(" ")),
        format!("{name}.BaseScriptList {};", records.join(", ")),
    ])
}

fn tag(reader: Reader<'_>, field: usize) -> Result<String> {
    let value = reader.tag(field)?;
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
        return Err("BASE tag cannot be represented safely in FEA".into());
    }
    Ok(value)
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

    fn axis_fixture() -> Vec<u8> {
        let mut data = words(&[4, 14, 2]);
        data.extend(b"ideoromn");
        data.extend(words(&[1]));
        data.extend(b"latn");
        data.extend(words(&[8, 6, 0, 0, 1, 2, 8, 12, 1, (-120i16) as u16, 1, 0]));
        data
    }

    fn table() -> Vec<u8> {
        let mut data = words(&[1, 0, 8, 0]);
        data.extend(axis_fixture());
        data
    }

    #[test]
    fn retains_horizontal_vertical_axes_and_default_baseline() {
        let mut data = table();
        let offset = data.len() as u16;
        data[6..8].copy_from_slice(&offset.to_be_bytes());
        data.extend(axis_fixture());
        let source = parse(Reader::new(&data), &[]).unwrap();
        for name in ["HorizAxis", "VertAxis"] {
            assert!(source.contains(&format!("{name}.BaseTagList ideo romn;")));
            assert!(source.contains(&format!("{name}.BaseScriptList latn romn -120 0;")));
        }
    }

    #[test]
    fn blocks_minmax_point_device_variation_and_malformed_data() {
        for (field, value, expected) in [
            (32, 6, "MinMax"),
            (34, 1, "MinMax"),
            (36, 2, "index"),
            (38, 1, "count"),
            (44, 2, "point mapping"),
            (44, 3, "device"),
            (12, 0, "empty"),
            (28, 0, "required offset"),
        ] {
            let mut data = table();
            data[field..field + 2].copy_from_slice(&u16::to_be_bytes(value));
            let result = parse(Reader::new(&data), &[]).unwrap_err();
            assert!(result.contains(expected), "{field}: {result}");
        }
        let bytes = words(&[1, 1, 0, 0, 0, 12, 0]);
        assert!(parse(Reader::new(&bytes), &[])
            .unwrap_err()
            .contains("variation store"));
        assert!(parse(Reader::new(&[0, 1]), &[])
            .unwrap_err()
            .contains("truncated"));
        assert!(parse(Reader::new(&words(&[1, 0, 0, 0])), &[])
            .unwrap_err()
            .contains("without axes"));
    }

    #[test]
    fn null_device_coordinates_are_static_and_truncated_tables_are_rejected() {
        let mut bytes = table();
        let source = parse(Reader::new(&bytes), &[]).unwrap();
        // Convert the last coordinate to format 3 with a null device offset.
        bytes[48..50].copy_from_slice(&3u16.to_be_bytes());
        bytes.extend([0, 0]);
        assert_eq!(source, parse(Reader::new(&bytes), &[]).unwrap());
        for len in 0..bytes.len() {
            assert!(parse(Reader::new(&bytes[..len]), &[]).is_err(), "{len}");
        }
    }

    fn compile(source: &str) -> String {
        let mut font = Font::new_empty("BASE roundtrip", 1_000);
        let master = font.default_master().id.clone();
        let mut glyph = Glyph::new(".notdef");
        glyph.layers.push(Layer::new_master(master.clone()));
        font.insert_glyph(glyph).unwrap();
        font.features.prefixes.push(FeatureBlock {
            code: source.into(),
            ..FeatureBlock::default()
        });
        let output =
            compile_static_with_metadata(&font, &master, &CompileOptions::default()).unwrap();
        let face = FontRef::new(&output.bytes).unwrap();
        let table = face.data_for_tag(Tag::new(b"BASE")).unwrap();
        parse(Reader::new(table.as_bytes()), &output.glyph_order).unwrap()
    }

    #[test]
    fn pinned_compiler_roundtrip_retains_both_axes_and_multiple_scripts() {
        let source = "table BASE {
            HorizAxis.BaseTagList ideo romn;
            HorizAxis.BaseScriptList cyrl romn -100 0, latn romn -120 0;
            VertAxis.BaseTagList ideo romn;
            VertAxis.BaseScriptList hani ideo 0 110, latn romn -110 0;
        } BASE;";
        let decompiled = compile(source);
        assert!(decompiled.contains("cyrl romn -100 0, latn romn -120 0"));
        assert!(decompiled.contains("hani ideo 0 110, latn romn -110 0"));
        assert_eq!(decompiled, compile(&decompiled));
    }
}
