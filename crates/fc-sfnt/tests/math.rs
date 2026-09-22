use fc_model::{
    MathAssembly, MathConstants, MathConstruction, MathDevice, MathGlyphInfo, MathKern, MathPart,
    MathTable, MathValue, MathVariant,
};

fn sample() -> MathTable {
    let adjusted = MathValue {
        value: -25.,
        device: Some(MathDevice {
            start_size: 12,
            deltas: vec![-2, -1, 0, 1],
        }),
    };
    let mut constants = MathConstants {
        script_percent_scale_down: 80,
        script_script_percent_scale_down: 60,
        radical_degree_bottom_raise_percent: -60,
        ..Default::default()
    };
    for (i, value) in constants.values_mut().into_iter().enumerate() {
        *value = MathValue::from(i as f64 - 25.);
    }
    constants.axis_height = adjusted.clone();
    let construction = MathConstruction {
        variants: vec![MathVariant {
            glyph: "part".into(),
            advance: 900.,
        }],
        assembly: Some(MathAssembly {
            italic_correction: adjusted.clone(),
            parts: vec![MathPart {
                glyph: "part".into(),
                start_connector: 40.,
                end_connector: 50.,
                full_advance: 400.,
                extender: true,
            }],
        }),
    };
    let mut glyph = MathGlyphInfo {
        italic_correction: Some(adjusted.clone()),
        top_accent: Some(MathValue::from(250.)),
        extended_shape: true,
        vertical: Some(construction.clone()),
        horizontal: Some(construction),
        ..Default::default()
    };
    glyph.kern.bottom_left = Some(MathKern {
        heights: vec![adjusted.clone()],
        values: vec![MathValue::from(-10.), adjusted],
    });
    MathTable {
        constants: Some(constants),
        min_connector_overlap: 20.,
        glyphs: [("base".into(), glyph)].into(),
    }
}

#[test]
fn every_math_field_roundtrips_across_glyph_orders() {
    let source = sample();
    for order in [
        vec![".notdef".into(), "base".into(), "part".into()],
        vec!["part".into(), ".notdef".into(), "base".into()],
    ] {
        let binary = fc_sfnt::math::write_table(&source, &order).unwrap();
        assert_eq!(fc_sfnt::math::read_table(&binary, &order).unwrap(), source);
        for end in 0..10 {
            assert!(fc_sfnt::math::read_table(&binary[..end], &order).is_err());
        }
    }
}

#[test]
fn device_formats_and_boundary_sizes_roundtrip() {
    for deltas in [vec![-2, 1, 0], vec![-8, 7, 2], vec![-128, 127, 8]] {
        let mut source = sample();
        source.constants.as_mut().unwrap().axis_height.device = Some(MathDevice {
            start_size: 65533,
            deltas,
        });
        let order = vec!["base".into(), "part".into()];
        let binary = fc_sfnt::math::write_table(&source, &order).unwrap();
        assert_eq!(fc_sfnt::math::read_table(&binary, &order).unwrap(), source);
    }
}

#[test]
fn malformed_offsets_counts_and_devices_are_errors() {
    let order = vec!["base".into(), "part".into()];
    let source = sample();
    let binary = fc_sfnt::math::write_table(&source, &order).unwrap();
    let mut bad = binary.clone();
    bad[0] = 2;
    assert!(fc_sfnt::math::read_table(&bad, &order).is_err());
    bad = binary.clone();
    bad[4..6].copy_from_slice(&65535u16.to_be_bytes());
    assert!(fc_sfnt::math::read_table(&bad, &order).is_err());
    let constants = u16::from_be_bytes([binary[4], binary[5]]) as usize;
    let axis_device =
        constants + u16::from_be_bytes([binary[constants + 14], binary[constants + 15]]) as usize;
    bad = binary.clone();
    bad[axis_device + 4..axis_device + 6].copy_from_slice(&0x8000u16.to_be_bytes());
    assert!(
        fc_sfnt::math::read_table(&bad, &order).is_err(),
        "MATH has no variation-store contract"
    );
    assert!(fc_sfnt::math::write_table(&source, &["base".into()]).is_err());
}

#[test]
fn coverage_ranges_validate_indices_and_names() {
    let words: [u16; 14] = [
        1, 0, 0, 10, 0, // MATH header.
        0, 0, 8, 0, // Glyph info, extended-shape coverage at offset 8.
        2, 1, 1, 2, 0, // One coverage range: glyphs 1..=2, index 0.
    ];
    let mut bytes: Vec<u8> = words.into_iter().flat_map(u16::to_be_bytes).collect();
    let order = vec![".notdef".into(), "base".into(), "part".into()];
    let math = fc_sfnt::math::read_table(&bytes, &order).unwrap();
    assert_eq!(math.glyphs.len(), 2);
    assert!(math.glyphs["base"].extended_shape);
    assert!(math.glyphs["part"].extended_shape);
    bytes[27] = 1;
    assert!(fc_sfnt::math::read_table(&bytes, &order).is_err());
}

#[test]
fn reused_device_offsets_cannot_amplify_work_without_bound() {
    let mut bytes = vec![0u8; 10 + 214 + 6 + 16384];
    bytes[1] = 1;
    bytes[5] = 10;
    for i in 0..51 {
        let at = 10 + 8 + i * 4 + 2;
        bytes[at..at + 2].copy_from_slice(&214u16.to_be_bytes());
    }
    let device = 10 + 214;
    bytes[device + 2..device + 4].copy_from_slice(&u16::MAX.to_be_bytes());
    bytes[device + 5] = 1;
    let error = fc_sfnt::math::read_table(&bytes, &[]).unwrap_err();
    assert!(error.to_string().contains("item limit"), "{error}");
}

#[test]
fn committed_dejavu_math_roundtrips_semantically() {
    use read_fonts::FontRef;
    let bytes = std::fs::read(fc_fixtures::path("DejaVuSans.ttf")).unwrap();
    let font = fc_sfnt::read_bytes(&bytes, &Default::default())
        .unwrap()
        .fonts
        .remove(0);
    let order: Vec<_> = font.glyph_order().cloned().collect();
    let face = FontRef::new(&bytes).unwrap();
    let data = face
        .table_data(read_fonts::types::Tag::new(b"MATH"))
        .unwrap();
    let math = fc_sfnt::math::read_table(data.as_bytes(), &order).unwrap();
    assert!(math.constants.is_some());
    assert!(!math.glyphs.is_empty());
    let encoded = fc_sfnt::math::write_table(&math, &order).unwrap();
    assert_eq!(fc_sfnt::math::read_table(&encoded, &order).unwrap(), math);
}

#[test]
fn fonttools_independently_decodes_and_reencodes_math() {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    if !Command::new("python3")
        .args(["-c", "import fontTools"])
        .output()
        .is_ok_and(|o| o.status.success())
    {
        eprintln!("skipping independent MATH check: python3/fontTools unavailable");
        return;
    }
    let order = vec![".notdef".into(), "base".into(), "part".into()];
    let source = sample();
    let bytes = fc_sfnt::math::write_table(&source, &order).unwrap();
    let script = r#"
import sys
from fontTools.ttLib import TTFont, newTable
from fontTools.ttLib.tables.otData import otData
font = TTFont()
font.setGlyphOrder(['.notdef', 'base', 'part'])
table = newTable('MATH')
table.decompile(sys.stdin.buffer.read(), font)
constants = table.table.MathConstants
fields = [field[1] for field in dict(otData)['MathConstants'] if field[0] == 'MathValueRecord']
assert len(fields) == 51
for i, field in enumerate(fields):
    assert getattr(constants, field).Value == (-25 if field == 'AxisHeight' else i - 25)
# OpenType 1.9.1 specifies int16; FontTools 4.63 still exposes this as uint16.
# Check the exact bits so either interpretation remains a useful oracle.
assert constants.RadicalDegreeBottomRaisePercent & 0xffff == (-60 & 0xffff)
assert constants.AxisHeight.DeviceTable.DeltaValue == [-2, -1, 0, 1]
info = table.table.MathGlyphInfo
assert info.ExtendedShapeCoverage.glyphs == ['base']
assert info.MathItalicsCorrectionInfo.ItalicsCorrection[0].Value == -25
assert info.MathTopAccentAttachment.TopAccentAttachment[0].Value == 250
kern = info.MathKernInfo.MathKernInfoRecords[0].BottomLeftMathKern
assert [v.Value for v in kern.KernValue] == [-10, -25]
variants = table.table.MathVariants
for constructions in [variants.VertGlyphConstruction, variants.HorizGlyphConstruction]:
    construction = constructions[0]
    assert construction.MathGlyphVariantRecord[0].VariantGlyph == 'part'
    assert construction.MathGlyphVariantRecord[0].AdvanceMeasurement == 900
    assembly = construction.GlyphAssembly
    assert assembly.ItalicsCorrection.DeviceTable.DeltaValue == [-2, -1, 0, 1]
    part = assembly.PartRecords[0]
    assert (part.glyph, part.StartConnectorLength, part.EndConnectorLength, part.FullAdvance, part.PartFlags) == ('part', 40, 50, 400, 1)
sys.stdout.buffer.write(table.compile(font))
"#;
    let mut child = Command::new("python3")
        .args(["-c", script])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(&bytes).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fc_sfnt::math::read_table(&output.stdout, &order).unwrap(),
        source
    );
}
