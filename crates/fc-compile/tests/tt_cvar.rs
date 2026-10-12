//! Variable TrueType export of authored global programs and control values.

mod common;

use fc_model::{Axis, LayerId, LayerKind, MasterId, Tag, TtMasterData};
use read_fonts::{types::Tag as RawTag, FontRef};

fn globals(cvt: Vec<i16>) -> TtMasterData {
    TtMasterData {
        authored_units_per_em: Some(1000),
        cvt,
        // PUSHB[0] 0; SVTCA[y]: a syntactically valid control value program.
        prep: vec![0xb0, 0x00, 0x00],
        fpgm: Vec::new(),
        max_stack_elements: Some(32),
        max_storage: Some(0),
        max_function_defs: Some(0),
        max_instruction_defs: Some(0),
        max_twilight_points: Some(0),
        max_zones: Some(1),
        max_size_of_instructions: None,
    }
}

fn two_master_font(heavy_cvt: Vec<i16>) -> fc_model::Font {
    let mut font = common::scratch_font();
    font.axes.push(Axis {
        tag: Tag(*b"wght"),
        name: "Weight".into(),
        min: 100.0,
        default: 100.0,
        max: 900.0,
        ..Default::default()
    });
    font.masters[0].location.insert(Tag(*b"wght"), 100.0);
    font.masters[0].tt = globals(vec![60, 700, -12]);
    let mut heavy = font.masters[0].clone();
    heavy.id = MasterId::new();
    heavy.name = "Heavy".into();
    heavy.location.insert(Tag(*b"wght"), 900.0);
    heavy.tt = globals(heavy_cvt);
    for glyph in font.glyphs.values_mut() {
        let mut layer = glyph.layers[0].clone();
        layer.id = LayerId::new();
        layer.kind = LayerKind::Master(heavy.id.clone());
        layer.width += 200.0;
        for contour in &mut layer.contours {
            for point in &mut contour.points {
                point.x *= 1.5;
            }
        }
        glyph.layers.push(layer);
    }
    font.masters.push(heavy);
    font
}

fn export(font: &fc_model::Font) -> fc_compile::ExportReport {
    fc_compile::export(
        font,
        &|_| 0,
        &fc_compile::ExportOptions {
            variable: true,
            format: fc_compile::ExportFormat::Ttf,
            ..Default::default()
        },
    )
    .expect("variable TrueType export")
}

#[test]
fn authored_globals_export_with_control_value_variation() {
    let report = export(&two_master_font(vec![140, 700, -24]));
    assert!(report
        .warnings
        .iter()
        .all(|warning| warning.code != "cvar-not-authored"));
    let face = FontRef::new(&report.bytes).expect("face");
    for tag in [b"fvar", b"gvar", b"cvt ", b"prep", b"cvar"] {
        assert!(
            face.table_data(RawTag::new(tag)).is_some(),
            "missing {}",
            String::from_utf8_lossy(tag)
        );
    }
    let cvt = face
        .table_data(RawTag::new(b"cvt "))
        .expect("cvt")
        .as_bytes();
    assert_eq!(cvt, [0, 60, 2, 188, 255, 244], "default master values");
    let encoded = face
        .table_data(RawTag::new(b"cvar"))
        .expect("cvar")
        .as_bytes()
        .to_vec();
    let cvar = <read_fonts::tables::cvar::Cvar as read_fonts::FontRead>::read(
        read_fonts::FontData::new(&encoded),
    )
    .expect("cvar");
    let mut deltas = [0i32; 3];
    cvar.deltas(1, &[read_fonts::types::F2Dot14::ONE], &mut deltas)
        .expect("deltas");
    // 140 - 60 = 80, 700 - 700 = 0, -24 - -12 = -12.
    assert_eq!(deltas, [80 << 16, 0, -12 << 16]);
}

#[test]
fn equal_control_values_need_no_variation_and_mismatched_lengths_are_reported() {
    let report = export(&two_master_font(vec![60, 700, -12]));
    let face = FontRef::new(&report.bytes).expect("face");
    assert!(face.table_data(RawTag::new(b"cvar")).is_none());
    let report = export(&two_master_font(vec![140, 700]));
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.code == "cvar-not-authored"),
        "{:?}",
        report.warnings
    );
    let face = FontRef::new(&report.bytes).expect("face");
    assert!(face.table_data(RawTag::new(b"cvar")).is_none());
}

#[test]
fn an_authored_glyph_program_still_requires_a_static_variable_free_export() {
    let mut font = two_master_font(vec![140, 700, -24]);
    let master = font.masters[0].id.clone();
    let name = font.glyph_order().next().expect("a glyph").clone();
    let layer = font
        .glyph_mut(&name)
        .expect("glyph")
        .master_layer_mut(&master)
        .expect("layer");
    layer.tt_instructions = Some(vec![0x00]);
    layer.tt_binding = Some(fc_model::TtInstructionBinding {
        fingerprint: format!("tt-points-v1:{}", "0".repeat(64)),
        authored: true,
    });
    let error = fc_compile::export(
        &font,
        &|_| 0,
        &fc_compile::ExportOptions {
            variable: true,
            format: fc_compile::ExportFormat::Ttf,
            ..Default::default()
        },
    )
    .expect_err("authored glyph programs cannot bind to a variable compile");
    assert!(
        error.to_string().contains("static master export"),
        "{error}"
    );
}

#[test]
fn independent_tools_read_the_authored_control_value_variation() {
    use std::process::Command;
    if !Command::new("python3")
        .args(["-c", "import fontTools, freetype"])
        .status()
        .is_ok_and(|status| status.success())
    {
        eprintln!("skipping independent cvar check: fontTools/freetype unavailable");
        return;
    }
    let directory = std::env::temp_dir().join(format!("fc-tt-cvar-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("temporary directory");
    let path = directory.join("variable.ttf");
    std::fs::write(&path, export(&two_master_font(vec![140, 700, -24])).bytes).expect("write");
    let script = r#"
import sys, freetype
from fontTools.ttLib import TTFont

font = TTFont(sys.argv[1])
assert list(font['cvt '].values) == [60, 700, -12], font['cvt '].values
cvar = font['cvar'].variations
assert len(cvar) == 1, cvar
tuple_ = cvar[0]
assert tuple_.axes == {'wght': (0.0, 1.0, 1.0)}, tuple_.axes
assert tuple_.coordinates == [80, 0, -12], tuple_.coordinates
assert font['maxp'].maxStackElements == 32, font['maxp'].maxStackElements
assert font['prep'].program.getBytecode() == bytes([0xb0, 0x00, 0x00])

renders = 0
face = freetype.Face(sys.argv[1])
for weight in (100, 500, 900):
    face.set_var_design_coords([weight])
    for size in (12, 24, 48):
        face.set_pixel_sizes(0, size)
        for index in range(face.num_glyphs):
            face.load_glyph(index, freetype.FT_LOAD_RENDER | freetype.FT_LOAD_TARGET_MONO)
            renders += 1
print('renders', renders)
"#;
    let file = directory.join("cvar.py");
    std::fs::write(&file, script).expect("write script");
    let output = Command::new("python3")
        .arg(&file)
        .arg(&path)
        .output()
        .expect("python3");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let status = output.status;
    let _ = std::fs::remove_dir_all(&directory);
    assert!(status.success(), "{stdout}\n{stderr}");
    assert!(stdout.contains("renders "), "{stdout}");
}
