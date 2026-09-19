use fc_formats::type1::{read_type1, write_type1};
use fc_model::{Contour, Font, Glyph, Layer, Point, PointKind};
use read_fonts::ps::type1::Type1Font;

fn point(x: f64, y: f64, kind: PointKind) -> Point {
    Point {
        x,
        y,
        kind,
        ..Point::default()
    }
}

fn sample(upem: u16) -> Font {
    let mut font = Font::default();
    font.info.family_name = "Type One Test".into();
    font.info.style_name = "Regular".into();
    font.info.postscript_font_name = Some("TypeOneTest-Regular".into());
    font.info.units_per_em = upem;
    let master = font.default_master().id.clone();
    for name in [".notdef", "space", "A"] {
        let mut glyph = Glyph::new(name);
        let mut layer = Layer::new_master(master.clone());
        layer.width = 612.25;
        if name == "A" {
            glyph.unicodes.push(65);
            layer.contours.push(Contour {
                closed: true,
                points: vec![
                    point(10.25, 20.5, PointKind::Line),
                    point(20.25, 80.125, PointKind::OffCurve),
                    point(100.5, 130.25, PointKind::OffCurve),
                    point(190.75, 20.5, PointKind::Curve),
                ],
                ..Contour::default()
            });
        } else if name == "space" {
            glyph.unicodes.push(32);
        }
        glyph.layers.push(layer);
        font.insert_glyph(glyph).unwrap();
    }
    font
}

#[test]
fn pfa_and_pfb_roundtrip_cubics_fractions_names_and_upem() {
    let started = std::time::Instant::now();
    for upem in [1000, 2048, 2000] {
        for binary in [false, true] {
            let font = sample(upem);
            let dir = tempfile::tempdir().unwrap();
            let path = dir
                .path()
                .join(if binary { "test.pfb" } else { "test.pfa" });
            write_type1(&font, &font.default_master().id, &path, binary).unwrap();
            let bytes = std::fs::read(&path).unwrap();
            assert_eq!(bytes.starts_with(&[0x80, 1]), binary);
            let independent = Type1Font::new(&bytes).unwrap();
            assert_eq!(independent.upem(), i32::from(upem));
            assert_eq!(independent.name(), Some("TypeOneTest-Regular"));
            let result = read_type1(&path).unwrap();
            assert_eq!(result.info.units_per_em, upem);
            assert_eq!(result.info.family_name, "Type One Test");
            let a = result.glyph("A").unwrap();
            assert!(a.unicodes.contains(&65));
            let layer = a.master_layer(&result.default_master().id).unwrap();
            assert!((layer.width - 612.25).abs() < 0.0001);
            let points = &layer.contours[0].points;
            assert_eq!((points[0].x, points[0].y), (10.25, 20.5));
            assert_eq!((points[1].x, points[1].y), (20.25, 80.125));
            assert!(result.glyph("space").unwrap().layers[0].contours.is_empty());
        }
    }
    assert!(started.elapsed() < std::time::Duration::from_secs(30));
}

#[test]
fn corrupt_input_and_invalid_output_do_not_damage_destination() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("font.pfb");
    std::fs::write(&path, b"not a font").unwrap();
    assert!(read_type1(&path).is_err());
    let mut font = sample(1000);
    font.glyph_mut("A").unwrap().layers[0].contours[0].closed = false;
    assert!(write_type1(&font, &font.default_master().id, &path, true).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"not a font");
    font.glyph_mut("A").unwrap().layers[0].contours[0].closed = true;
    font.glyph_mut("A").unwrap().layers[0].width = f64::NAN;
    assert!(write_type1(&font, &font.default_master().id, &path, true).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"not a font");
}

#[test]
fn external_native_type1_fixture() {
    let root = fc_fixtures::skip_unless_external!("fontforge-tests");
    for name in ["cmbsy10.pfb", "n019003l.pfb", "feta20.pfb"] {
        let path = root.join(name);
        if !path.is_file() {
            continue;
        }
        let font = read_type1(&path).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(font.glyphs.len() > 20, "{name}");
        assert!(font
            .glyphs
            .values()
            .any(|glyph| !glyph.layers[0].contours.is_empty()));
        assert!(font.info.units_per_em > 0);
    }
    // This corpus member is intentionally corrupt; fontTools independently
    // rejects its PFB framing as well.
    let corrupt = root.join("ecrm1000.pfb");
    if corrupt.is_file() {
        assert!(read_type1(&corrupt).is_err());
    }
}

#[test]
fn independent_fonttools_and_freetype_read_both_outputs() {
    use std::process::Command;
    if !Command::new("python3")
        .args(["-c", "import fontTools"])
        .status()
        .is_ok_and(|s| s.success())
    {
        eprintln!("skipping fontTools check: Python fontTools is unavailable");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let font = sample(2048);
    for binary in [false, true] {
        let path = dir
            .path()
            .join(if binary { "native.pfb" } else { "native.pfa" });
        write_type1(&font, &font.default_master().id, &path, binary).unwrap();
        let script = r#"
import sys, ctypes, ctypes.util
from fontTools.t1Lib import T1Font
from fontTools.pens.recordingPen import RecordingPen
font = T1Font(sys.argv[1])
assert font['FontName'] == 'TypeOneTest-Regular'
assert abs(font['FontMatrix'][0] - 1/2048) < 1e-12
glyph = font.getGlyphSet()['A']
pen = RecordingPen()
glyph.draw(pen)
assert glyph.width == 612.25, glyph.width
assert pen.value[0] == ('moveTo', ((10.25, 20.5),)), pen.value
assert pen.value[1] == ('curveTo', ((20.25, 80.125), (100.5, 130.25), (190.75, 20.5))), pen.value
library = ctypes.util.find_library('freetype')
if library:
    ft = ctypes.CDLL(library)
    handle, face = ctypes.c_void_p(), ctypes.c_void_p()
    assert ft.FT_Init_FreeType(ctypes.byref(handle)) == 0
    ft.FT_New_Face.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_long, ctypes.POINTER(ctypes.c_void_p)]
    assert ft.FT_New_Face(handle, sys.argv[1].encode(), 0, ctypes.byref(face)) == 0
    assert ft.FT_Done_Face(face) == 0
    assert ft.FT_Done_FreeType(handle) == 0
"#;
        let result = Command::new("python3")
            .args(["-c", script])
            .arg(path)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
}

#[test]
fn components_are_flattened_and_quadratics_raised_with_multiple_contours() {
    let mut font = sample(1000);
    let master = font.default_master().id.clone();
    let mut glyph = Glyph::new("B");
    let mut layer = Layer::new_master(master.clone());
    layer.width = 700.0;
    layer.contours.push(Contour {
        closed: true,
        points: vec![
            point(30.0, 40.0, PointKind::Line),
            point(60.0, 130.0, PointKind::OffCurve),
            point(180.0, 40.0, PointKind::QCurve),
        ],
        ..Contour::default()
    });
    layer.components.push(fc_model::Component {
        base: "A".into(),
        transform: kurbo::Affine::translate((200.5, 100.25)),
        ..Default::default()
    });
    glyph.layers.push(layer);
    font.insert_glyph(glyph).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("composite.pfb");
    write_type1(&font, &master, &path, true).unwrap();
    let result = read_type1(&path).unwrap();
    let contours = &result.glyph("B").unwrap().layers[0].contours;
    assert_eq!(contours.len(), 2);
    assert_eq!(
        (contours[0].points[1].x, contours[0].points[1].y),
        (50.0, 100.0)
    );
    assert_eq!(
        (contours[0].points[2].x, contours[0].points[2].y),
        (100.0, 100.0)
    );
    assert_eq!(
        (contours[1].points[0].x, contours[1].points[0].y),
        (210.75, 120.75)
    );
}

#[test]
fn invalid_drafts_components_names_and_dimensions_are_atomic() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("original.pfa");
    std::fs::write(&path, b"original").unwrap();
    for case in 0..6 {
        let mut font = sample(1000);
        match case {
            0 => font.info.units_per_em = 0,
            1 => font.info.postscript_font_name = Some("bad/name".into()),
            2 => font.glyph_mut("A").unwrap().layers[0]
                .components
                .push(fc_model::Component {
                    base: "A".into(),
                    ..Default::default()
                }),
            3 => font.glyph_mut("A").unwrap().layers[0]
                .components
                .push(fc_model::Component {
                    base: "missing".into(),
                    ..Default::default()
                }),
            4 => font.glyph_mut("A").unwrap().layers[0].contours[0].points[0].x = f64::INFINITY,
            _ => {
                let mut source = font.features.clone();
                source.prefixes.push(fc_model::FeatureBlock {
                    code: "invalid {".into(),
                    ..Default::default()
                });
                font.feature_draft = Some(fc_model::FeatureDraft {
                    source,
                    settings: font.feature_settings.clone(),
                    revision: 1,
                });
            }
        }
        assert!(
            write_type1(&font, &font.default_master().id, &path, false).is_err(),
            "case {case}"
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"original");
    }
}

#[test]
fn encoding_is_retained_and_unicode_losses_are_reported() {
    let mut font = sample(1000);
    let mut encoding = vec![".notdef"; 256];
    encoding[123] = "A";
    font.user_data = serde_json::json!({"org.fontcarver.type1": {"encoding": encoding}});
    font.glyph_mut("A").unwrap().unicodes.push(0x10000);
    assert!(
        fc_formats::type1::loss_warnings(&font, &font.default_master().id)
            .iter()
            .any(|m| m.code == "type1-unicode-loss")
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("encoding.pfa");
    write_type1(&font, &font.default_master().id, &path, false).unwrap();
    let result = read_type1(&path).unwrap();
    assert_eq!(
        result.user_data["org.fontcarver.type1"]["encoding"][123],
        "A"
    );
    assert!(!result.glyph("A").unwrap().unicodes.contains(&0x10000));
    let path2 = dir.path().join("encoding2.pfb");
    write_type1(&result, &result.default_master().id, &path2, true).unwrap();
    let independent = Type1Font::new(&std::fs::read(path2).unwrap()).unwrap();
    assert_eq!(independent.encoding().unwrap().glyph_name(123), Some("A"));
}

#[test]
fn escaped_names_and_empty_font_roundtrip() {
    let mut font = sample(1000);
    font.info.family_name = "Family (Alt) \\ 中文".into();
    font.info.postscript_is_fixed_pitch = Some(true);
    font.info.postscript_underline_position = Some(-95.5);
    font.info.postscript_underline_thickness = Some(48.25);
    font.masters[0].metrics.italic_angle = -14.035;
    font.glyphs.clear();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.pfa");
    write_type1(&font, &font.default_master().id, &path, false).unwrap();
    let result = read_type1(&path).unwrap();
    assert_eq!(result.info.family_name, font.info.family_name);
    assert_eq!(result.info.postscript_is_fixed_pitch, Some(true));
    assert_eq!(result.info.postscript_underline_position, Some(-95.5));
    assert_eq!(result.info.postscript_underline_thickness, Some(48.25));
    assert_eq!(result.default_master().metrics.italic_angle, -14.035);
    assert_eq!(result.glyphs.len(), 1);
    assert!(result.glyph(".notdef").unwrap().layers[0]
        .contours
        .is_empty());
}

#[test]
fn truncated_binary_frames_are_rejected() {
    let font = sample(1000);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("truncated.pfb");
    write_type1(&font, &font.default_master().id, &path, true).unwrap();
    let original = std::fs::read(&path).unwrap();
    for length in [1, 4, 20, original.len() - 2] {
        std::fs::write(&path, &original[..length]).unwrap();
        assert!(read_type1(&path).is_err());
    }
}

#[test]
fn expansion_is_bounded_and_unexported_sketches_are_ignored() {
    let mut font = sample(1000);
    let master = font.default_master().id.clone();
    let mut sketch = Glyph::new("sketch");
    sketch.export = false;
    let mut layer = Layer::new_master(master.clone());
    layer.contours.push(Contour {
        points: vec![point(0.0, 0.0, PointKind::Move)],
        ..Default::default()
    });
    sketch.layers.push(layer);
    font.insert_glyph(sketch).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bounded.pfb");
    write_type1(&font, &master, &path, true).unwrap();
    let original = std::fs::read(&path).unwrap();
    let mut base = "A".to_owned();
    for level in 0..20 {
        let mut glyph = Glyph::new(format!("level{level}"));
        let mut layer = Layer::new_master(master.clone());
        layer.components = vec![
            fc_model::Component {
                base: base.clone().into(),
                ..Default::default()
            };
            2
        ];
        glyph.layers.push(layer);
        base = glyph.name.to_string();
        font.insert_glyph(glyph).unwrap();
    }
    let error = write_type1(&font, &master, &path, true).unwrap_err();
    assert!(error.to_string().contains("complexity"));
    assert_eq!(std::fs::read(&path).unwrap(), original);
}

#[test]
fn stroked_type1_is_reported_instead_of_importing_as_filled() {
    let font = sample(1000);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("stroked.pfa");
    write_type1(&font, &font.default_master().id, &path, false).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, text.replace("/PaintType 0", "/PaintType 2")).unwrap();
    assert!(read_type1(&path)
        .unwrap_err()
        .to_string()
        .contains("PaintType"));
}

#[test]
fn oversized_charstrings_are_rejected_without_touching_output() {
    let mut font = sample(1000);
    font.glyph_mut("A").unwrap().layers[0].contours[0].points = (0..25_000)
        .map(|index| point(f64::from(index % 2), f64::from(index % 3), PointKind::Line))
        .collect();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("overlong.pfb");
    std::fs::write(&path, b"original").unwrap();
    let error = write_type1(&font, &font.default_master().id, &path, true).unwrap_err();
    assert!(error.to_string().contains("65,535-byte"));
    assert_eq!(std::fs::read(path).unwrap(), b"original");
}

#[test]
fn p2b_alternates_reopen_without_duplicate_unicode_and_shape_without_layout() {
    let mut font = sample(1000);
    font.glyphs.clear();
    let master = font.default_master().id.clone();
    for name in [
        ".notdef", "A", "V", "a", "v", "f", "i", "f_i", "A.sc", "A.ss01", "A.case", "A.vert",
    ] {
        let mut glyph = Glyph::new(name);
        if name.len() == 1 {
            glyph.unicodes.push(u32::from(name.as_bytes()[0]));
        }
        let mut layer = Layer::new_master(master.clone());
        layer.width = 600.0;
        glyph.layers.push(layer);
        font.insert_glyph(glyph).unwrap();
    }
    font.masters[0].kerning_h.0.insert(
        (
            fc_model::KernSide::Glyph("A".into()),
            fc_model::KernSide::Glyph("V".into()),
        ),
        -80.0,
    );
    font.features.features.push(fc_model::FeatureBlock {
        tag: "liga".into(),
        code: "sub f i by f_i;".into(),
        ..Default::default()
    });
    let dir = tempfile::tempdir().unwrap();
    for binary in [false, true] {
        let path = dir.path().join(if binary { "p2b.pfb" } else { "p2b.pfa" });
        write_type1(&font, &master, &path, binary).unwrap();
        let reopened = read_type1(&path).unwrap();
        assert_eq!(reopened.glyph("A").unwrap().unicodes, [65]);
        for name in ["A.sc", "A.ss01", "A.case", "A.vert"] {
            assert!(reopened.glyph(name).unwrap().unicodes.is_empty(), "{name}");
        }
        let compiled = fc_compile::compile_static_with_metadata(
            &reopened,
            &reopened.default_master().id,
            &Default::default(),
        )
        .unwrap();
        let shaped = fc_compile::shaping_fingerprint(&compiled.bytes, "AVfi").unwrap();
        let names: Vec<_> = shaped
            .iter()
            .map(|(gid, _, _)| compiled.glyph_order[*gid as usize].as_ref())
            .collect();
        assert_eq!(names, ["A", "V", "f", "i"]);
        assert!(shaped.iter().all(|(_, width, _)| *width == 600));
    }
}

#[test]
fn native_encoding_selects_one_glyph_for_ambiguous_unicode_names() {
    let mut font = sample(1000);
    let master = font.default_master().id.clone();
    for name in ["A.alt", "uni0041"] {
        let mut glyph = font.glyph("A").unwrap().clone();
        glyph.name = name.into();
        glyph.unicodes.clear();
        font.insert_glyph(glyph).unwrap();
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ambiguous.pfb");
    for chosen in ["A", "A.alt", "uni0041"] {
        let mut encoding = vec![".notdef"; 256];
        encoding[65] = chosen;
        font.user_data = serde_json::json!({"org.fontcarver.type1": {"encoding": encoding}});
        assert_eq!(
            fc_formats::type1::loss_warnings(&font, &master)
                .iter()
                .any(|warning| warning.code == "type1-unicode-loss"),
            chosen != "A"
        );
        write_type1(&font, &master, &path, true).unwrap();
        let result = read_type1(&path).unwrap();
        for name in ["A", "A.alt", "uni0041"] {
            assert_eq!(
                result.glyph(name).unwrap().unicodes.contains(&65),
                name == chosen,
                "encoding {chosen}, glyph {name}"
            );
        }
    }
}

#[test]
fn font_bbox_contains_quantized_curve_extrema() {
    use kurbo::Shape;
    let mut font = sample(1000);
    let a = 0.000023;
    let b = (4.0 - a) / 3.0;
    let mut path = kurbo::BezPath::new();
    path.move_to((0.0, a));
    path.curve_to((100.0, b), (200.0, b), (300.0, a));
    path.close_path();
    assert!(path.bounding_box().y1 < 1.0);
    font.glyph_mut("A").unwrap().layers[0].contours = Contour::from_bezpath(&path);
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("bounds.pfb");
    write_type1(&font, &font.default_master().id, &output, true).unwrap();
    let native = Type1Font::new(&std::fs::read(&output).unwrap()).unwrap();
    assert_eq!(native.bbox().y_max.to_i32(), 2);
    let reopened = read_type1(&output).unwrap();
    let bounds = reopened.glyph("A").unwrap().layers[0].contours[0]
        .to_bezpath()
        .bounding_box();
    assert!(
        bounds.y1 > 1.0 && bounds.y1 <= 2.0,
        "{bounds:?}: {:?}",
        reopened.glyph("A").unwrap().layers[0].contours
    );
}

#[test]
fn large_upem_and_copyright_preserve_native_values() {
    let dir = tempfile::tempdir().unwrap();
    for upem in [10000, 12000, 16001, 65535] {
        let mut font = sample(upem);
        font.info.copyright = Some("© 2026 (Test) \\ Author".into());
        for binary in [false, true] {
            let path = dir
                .path()
                .join(if binary { "large.pfb" } else { "large.pfa" });
            write_type1(&font, &font.default_master().id, &path, binary).unwrap();
            let result = read_type1(&path).unwrap();
            assert_eq!(result.info.units_per_em, upem);
            assert_eq!(result.info.copyright, font.info.copyright);
            assert_eq!(result.glyph("A").unwrap().layers[0].width, 612.25);
            assert_eq!(
                result.glyph("A").unwrap().layers[0].contours[0].points[0].x,
                10.25
            );
            if !binary {
                let text = std::fs::read_to_string(&path).unwrap();
                std::fs::write(&path, text.replace('©', "\\251")).unwrap();
                assert_eq!(
                    read_type1(&path).unwrap().info.copyright,
                    font.info.copyright
                );
            }
        }
    }
}

#[test]
fn tiny_signed_fractions_do_not_overflow_type1_division() {
    let mut font = sample(1000);
    let layer = &mut font.glyph_mut("A").unwrap().layers[0];
    layer.width = 0.25;
    layer.contours[0].points = vec![
        point(1.0 / 65536.0, -1.0 / 65536.0, PointKind::Line),
        point(0.125, 0.25, PointKind::Line),
        point(-0.25, 0.0, PointKind::Line),
    ];
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tiny.pfb");
    write_type1(&font, &font.default_master().id, &path, true).unwrap();
    let result = read_type1(&path).unwrap();
    assert_eq!(result.glyph("A").unwrap().layers[0].width, 0.25);
    let points = &result.glyph("A").unwrap().layers[0].contours[0].points;
    assert_eq!((points[0].x, points[0].y), (1.0 / 65536.0, -1.0 / 65536.0));
    assert_eq!((points[1].x, points[1].y), (0.125, 0.25));
    font.glyph_mut("A").unwrap().layers[0].width = 31999.999999;
    for binary in [false, true] {
        write_type1(&font, &font.default_master().id, &path, binary).unwrap();
        assert_eq!(
            read_type1(&path).unwrap().glyph("A").unwrap().layers[0].width,
            32000.0
        );
    }
}

#[test]
fn unsupported_native_metrics_and_nonhorizontal_matrix_are_rejected() {
    let font = sample(1000);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("metrics.pfa");
    write_type1(&font, &font.default_master().id, &path, false).unwrap();
    let original = std::fs::read_to_string(&path).unwrap();
    for key in ["Metrics", "Metrics2", "CDevProc"] {
        std::fs::write(
            &path,
            original.replace("/FontName", &format!("/{key} 0 dict def\n/FontName")),
        )
        .unwrap();
        assert!(read_type1(&path)
            .unwrap_err()
            .to_string()
            .contains("overrides"));
    }
    std::fs::write(
        &path,
        original.replace(
            "[0.001000000000000 0 0 0.001000000000000 0 0]",
            "[0.001 0.001 0 0.001 0 0]",
        ),
    )
    .unwrap();
    assert!(read_type1(&path)
        .unwrap_err()
        .to_string()
        .contains("nonhorizontal"));
    std::fs::write(
        &path,
        original.replace(
            "[0.001000000000000 0 0 0.001000000000000 0 0]",
            "[0.001 0 0.0005 -0.001 0 0.02]",
        ),
    )
    .unwrap();
    let flipped = read_type1(&path).unwrap();
    assert!(flipped.default_master().metrics.descender < 0.0);
    assert!((flipped.glyph("A").unwrap().layers[0].contours[0].points[0].y + 0.5).abs() < 1e-8);
}

#[test]
fn native_sbw_charstrings_and_subroutines_are_rejected() {
    // Native unencrypted charstrings (lenIV=-1), wrapped in a genuine eexec
    // program. This exercises the guard independently of our hsbw-only writer.
    fn encrypted(plain: &[u8]) -> Vec<u8> {
        let mut seed = 55665u16;
        plain
            .iter()
            .map(|byte| {
                let encoded = *byte ^ (seed >> 8) as u8;
                seed = seed
                    .wrapping_add(u16::from(encoded))
                    .wrapping_mul(52845)
                    .wrapping_add(22719);
                encoded
            })
            .collect()
    }
    let clear = b"%!PS-AdobeFont-1.0: Guard 1.0\n/FontName /Guard def\n/FontType 1 def\n/FontMatrix [0.001 0 0 0.001 0 0] def\n/Encoding StandardEncoding def\ncurrentfile eexec\n";
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sbw.pfb");
    for subroutine in [false, true] {
        let mut private = b"0000/Private 8 dict dup begin /lenIV -1 def\n".to_vec();
        // 0 0 600 50 sbw endchar; binary data deliberately contains an escape.
        let sbw = [139, 139, 248, 236, 189, 12, 7, 14];
        if subroutine {
            private.extend_from_slice(b"/Subrs 1 array dup 0 8 RD ");
            private.extend_from_slice(&sbw);
            private.extend_from_slice(b" NP\n/CharStrings 1 dict begin /.notdef 4 RD ");
            private.extend_from_slice(&[139, 139, 13, 14]);
        } else {
            private.extend_from_slice(b"/Subrs 0 array /CharStrings 1 dict begin /.notdef 8 RD ");
            private.extend_from_slice(&sbw);
        }
        private.extend_from_slice(b" ND end end currentfile closefile\n");
        let mut bytes = Vec::new();
        for (kind, data) in [
            (1, clear.to_vec()),
            (2, encrypted(&private)),
            (1, b"cleartomark\n".to_vec()),
        ] {
            bytes.extend_from_slice(&[0x80, kind]);
            bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&data);
        }
        bytes.extend_from_slice(&[0x80, 3]);
        std::fs::write(&path, bytes).unwrap();
        assert!(read_type1(&path).unwrap_err().to_string().contains("sbw"));
    }
}
