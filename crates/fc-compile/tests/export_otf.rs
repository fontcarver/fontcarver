mod common;

use fc_compile::{export, shaping_fingerprint, ExportFormat, ExportOptions};
use fc_model::{Contour, Glyph, Layer};
use kurbo::BezPath;
use read_fonts::{types::Tag, FontRef, TableProvider};

#[test]
fn otf_has_cff_outlines_and_the_same_layout_as_ttf() {
    let font = common::scratch_font();
    let otf = export(
        &font,
        &|_| 0,
        &ExportOptions {
            format: ExportFormat::Otf,
            ..ExportOptions::default()
        },
    )
    .expect("export OTF");
    assert_eq!(&otf.bytes[..4], b"OTTO");
    let face = FontRef::new(&otf.bytes).expect("read OTF");
    assert!(face.table_data(Tag::new(b"CFF ")).is_some());
    for tag in [b"glyf", b"loca", b"fpgm", b"prep", b"cvt "] {
        assert!(face.table_data(Tag::new(tag)).is_none());
    }
    assert_eq!(face.maxp().expect("maxp").num_glyphs(), 4);
    let ttf = export(&font, &|_| 0, &ExportOptions::default()).expect("export TTF");
    assert_eq!(
        shaping_fingerprint(&otf.bytes, "AV").expect("shape OTF"),
        shaping_fingerprint(&ttf.bytes, "AV").expect("shape TTF")
    );
    let reopened = fc_sfnt::read_bytes(&otf.bytes, &fc_sfnt::ReadOptions::default())
        .expect("reopen OTF")
        .fonts
        .remove(0);
    assert_eq!(
        reopened
            .glyph("A")
            .expect("A")
            .master_layer(&reopened.default_master().id)
            .expect("A layer")
            .width,
        600.0
    );
}

#[test]
fn cubic_control_points_survive_otf_and_nonstandard_upem() {
    let mut font = common::scratch_font();
    font.info.units_per_em = 2048;
    let master = font.default_master().id.clone();
    let mut path = BezPath::new();
    path.move_to((10.25, 0.0));
    path.curve_to((75.5, 800.125), (700.25, 850.5), (900.0, 0.0));
    path.close_path();
    let mut glyph = Glyph::new("curve");
    glyph.unicodes.push(0x43);
    let mut layer = Layer::new_master(master);
    layer.width = 1000.0;
    layer.contours = Contour::from_bezpath(&path);
    glyph.layers.push(layer);
    font.insert_glyph(glyph).expect("insert curve");
    let report = export(
        &font,
        &|_| 0,
        &ExportOptions {
            format: ExportFormat::Otf,
            ..ExportOptions::default()
        },
    )
    .expect("export cubic OTF");
    let reopened = fc_sfnt::read_bytes(&report.bytes, &fc_sfnt::ReadOptions::default())
        .expect("reopen cubic OTF")
        .fonts
        .remove(0);
    assert_eq!(reopened.info.units_per_em, 2048);
    let points = &reopened
        .glyph("curve")
        .expect("curve")
        .master_layer(&reopened.default_master().id)
        .expect("curve layer")
        .contours[0]
        .points;
    assert!(
        points
            .iter()
            .any(|point| (point.x - 75.5).abs() < 0.001 && (point.y - 800.125).abs() < 0.001),
        "{points:?}"
    );
}

#[test]
fn real_cff_fonts_reexport_with_matching_layout_and_outlines() {
    for fixture in ["SourceSans3Min.otf", "SourceSerif4Min.otf"] {
        let original = std::fs::read(fc_fixtures::path(fixture)).expect("fixture bytes");
        let font = fc_sfnt::read_bytes(&original, &fc_sfnt::ReadOptions::default())
            .expect("read CFF fixture")
            .fonts
            .remove(0);
        let report = export(
            &font,
            &|_| 0,
            &ExportOptions {
                format: ExportFormat::Otf,
                ..ExportOptions::default()
            },
        )
        .expect("reexport CFF fixture");
        let reopened = fc_sfnt::read_bytes(&report.bytes, &fc_sfnt::ReadOptions::default())
            .expect("read exported CFF fixture")
            .fonts
            .remove(0);
        assert_eq!(font.glyphs.len(), reopened.glyphs.len());
        assert_eq!(
            shaping_fingerprint(&original, "AoA").expect("original shaping"),
            shaping_fingerprint(&report.bytes, "AoA").expect("export shaping"),
            "{fixture}"
        );
        for name in font.glyph_order() {
            let before = fc_model::flatten_layer(&font, name, &font.default_master().id)
                .expect("original outline");
            let after = fc_model::flatten_layer(&reopened, name, &reopened.default_master().id)
                .expect("roundtrip outline");
            assert_eq!(
                before.elements().len(),
                after.elements().len(),
                "{fixture} {name}"
            );
            for (before, after) in before.elements().iter().zip(after.elements()) {
                assert_eq!(
                    std::mem::discriminant(before),
                    std::mem::discriminant(after)
                );
                let coordinates = |element: &kurbo::PathEl| -> Vec<kurbo::Point> {
                    match *element {
                        kurbo::PathEl::MoveTo(p) | kurbo::PathEl::LineTo(p) => vec![p],
                        kurbo::PathEl::QuadTo(a, b) => vec![a, b],
                        kurbo::PathEl::CurveTo(a, b, c) => vec![a, b, c],
                        kurbo::PathEl::ClosePath => vec![],
                    }
                };
                for (a, b) in coordinates(before).into_iter().zip(coordinates(after)) {
                    assert!(
                        (a.x - b.x).abs() <= 1.0 / 65536.0 && (a.y - b.y).abs() <= 1.0 / 65536.0,
                        "{fixture} {name}: {a:?} != {b:?}"
                    );
                }
            }
        }
        assert!(!report
            .warnings
            .iter()
            .any(|warning| warning.code == "cff-unhinted-outlines"));
    }
}

#[test]
fn unusual_upem_preserves_model_coordinates_and_widths() {
    for upem in [1001, 10000, 16384] {
        let mut font = common::scratch_font();
        font.info.units_per_em = upem;
        let report = export(
            &font,
            &|_| 0,
            &ExportOptions {
                format: ExportFormat::Otf,
                ..ExportOptions::default()
            },
        )
        .unwrap();
        let reopened = fc_sfnt::read_bytes(&report.bytes, &fc_sfnt::ReadOptions::default())
            .unwrap()
            .fonts
            .remove(0);
        assert_eq!(reopened.info.units_per_em, upem);
        let before = font
            .glyph("A")
            .unwrap()
            .master_layer(&font.default_master().id)
            .unwrap();
        let after = reopened
            .glyph("A")
            .unwrap()
            .master_layer(&reopened.default_master().id)
            .unwrap();
        assert_eq!(before.width, after.width);
        for (a, b) in before.contours[0]
            .points
            .iter()
            .zip(&after.contours[0].points)
        {
            assert!(
                (a.x - b.x).abs() < 0.001 && (a.y - b.y).abs() < 0.001,
                "UPEM {upem}: {a:?} != {b:?}"
            );
        }
    }
}

#[test]
fn otf_table_subset_keeps_required_cff_tables() {
    let font = common::scratch_font();
    let report = export(
        &font,
        &|_| 0,
        &ExportOptions {
            format: ExportFormat::Otf,
            table_subset: Some(vec![]),
            ..ExportOptions::default()
        },
    )
    .expect("subset OTF");
    assert_eq!(&report.bytes[..4], b"OTTO");
    let face = FontRef::new(&report.bytes).expect("OTF");
    assert!(face.table_data(Tag::new(b"CFF ")).is_some());
    assert!(face.table_data(Tag::new(b"GPOS")).is_none());
    assert!(fc_sfnt::read_bytes(&report.bytes, &fc_sfnt::ReadOptions::default()).is_ok());
}

#[test]
fn invalid_geometry_is_rejected_before_fontc_can_process_it() {
    let mut font = common::scratch_font();
    let master = font.default_master().id.clone();
    font.glyph_mut("A")
        .expect("A")
        .master_layer_mut(&master)
        .expect("layer")
        .contours[0]
        .points[0]
        .x = f64::NAN;
    let result = export(
        &font,
        &|_| 0,
        &ExportOptions {
            format: ExportFormat::Otf,
            ..ExportOptions::default()
        },
    );
    assert!(result
        .expect_err("reject NaN")
        .to_string()
        .contains("non-finite"));
}

#[test]
fn wide_individual_right_bearing_does_not_overflow_aggregate_minimum() {
    let mut font = common::scratch_font();
    let master = font.default_master().id.clone();
    let mut path = BezPath::new();
    path.move_to((-1000.0, 0.0));
    path.line_to((-500.0, 200.0));
    path.line_to((-500.0, 0.0));
    path.close_path();
    let layer = font
        .glyph_mut("A")
        .unwrap()
        .master_layer_mut(&master)
        .unwrap();
    layer.width = 32767.0;
    layer.contours = Contour::from_bezpath(&path);
    let report = export(
        &font,
        &|_| 0,
        &ExportOptions {
            format: ExportFormat::Otf,
            ..ExportOptions::default()
        },
    )
    .expect("valid aggregate despite one right bearing > i16");
    let face = FontRef::new(&report.bytes).unwrap();
    assert!(face.hhea().unwrap().min_right_side_bearing().to_i16() < 32767);
}
