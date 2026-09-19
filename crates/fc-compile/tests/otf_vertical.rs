mod common;

use fc_compile::{export, ExportFormat, ExportOptions};
use fc_model::{Contour, Font};
use kurbo::{BezPath, Shape};
use read_fonts::{types::GlyphId, FontRef, TableProvider};

fn options() -> ExportOptions {
    ExportOptions {
        format: ExportFormat::Otf,
        ..ExportOptions::default()
    }
}

fn vertical_font() -> Font {
    let mut font = common::scratch_font();
    for glyph in font.glyphs.values_mut() {
        glyph.layers[0].vertical_width = Some(1000.0);
        glyph.layers[0].vertical_origin = Some(900.0);
    }
    // The cubic has an actual maximum y of 600.1875. The intermediate
    // quadratic compilation has different control-point extrema.
    let mut curve = BezPath::new();
    curve.move_to((0.0, 0.0));
    curve.curve_to((0.0, 800.25), (500.0, 800.25), (500.0, 0.0));
    curve.close_path();
    font.glyph_mut("A").unwrap().layers[0].contours = Contour::from_bezpath(&curve);
    font.glyph_mut("V").unwrap().layers[0].vertical_width = Some(1200.0);
    font.glyph_mut("V").unwrap().layers[0].vertical_origin = Some(850.0);
    // Deliberately extreme empty-glyph bearings must not enter vhea extrema.
    font.glyph_mut(".notdef").unwrap().layers[0].vertical_origin = Some(2000.0);
    font.glyph_mut("space").unwrap().layers[0].vertical_origin = Some(-100.0);
    font
}

fn gid_for(face: &FontRef<'_>, codepoint: u32) -> GlyphId {
    let mapping = skrifa::charmap::Charmap::new(face);
    mapping.map(codepoint).expect("mapped glyph")
}

#[test]
fn cubic_bounds_regenerate_vertical_bearings_and_preserve_origins_and_advances() {
    let report = export(&vertical_font(), &|_| 0, &options()).expect("export vertical OTF");
    let face = FontRef::new(&report.bytes).unwrap();
    let a = gid_for(&face, 0x41);
    let v = gid_for(&face, 0x56);
    let space = gid_for(&face, 0x20);
    let vmtx = face.vmtx().expect("vertical metrics");
    assert_eq!(vmtx.advance(a), Some(1000));
    assert_eq!(vmtx.advance(v), Some(1200));
    assert_eq!(vmtx.side_bearing(a), Some(299));
    assert_eq!(vmtx.side_bearing(v), Some(150));
    assert_eq!(vmtx.side_bearing(space), Some(-100));
    let vorg = face.vorg().expect("explicit CFF vertical origins");
    assert_eq!(vorg.vertical_origin_y(a), 900);
    assert_eq!(vorg.vertical_origin_y(v), 850);
    assert_eq!(vorg.vertical_origin_y(space), -100);
    let vhea = face.vhea().unwrap();
    assert_eq!(vhea.advance_height_max().to_u16(), 1200);
    assert_eq!(vhea.min_top_side_bearing().to_i16(), 150);
    assert_eq!(vhea.min_bottom_side_bearing().to_i16(), 100);
    assert_eq!(vhea.y_max_extent().to_i16(), 900);
    let reopened = fc_sfnt::read_bytes(&report.bytes, &fc_sfnt::ReadOptions::default())
        .expect("reopen OTF")
        .fonts
        .remove(0);
    let a_layer = &reopened.glyph("A").unwrap().layers[0];
    assert_eq!(a_layer.vertical_width, Some(1000.0));
    assert_eq!(a_layer.vertical_origin, Some(900.0));
    let bounds = a_layer.contours[0].to_bezpath().bounding_box();
    assert!((bounds.y1 - 600.1875).abs() < 0.001, "{bounds:?}");
}

#[test]
fn unmodeled_vertical_origins_preserve_the_compilers_effective_origin() {
    let mut font = vertical_font();
    for glyph in font.glyphs.values_mut() {
        glyph.layers[0].vertical_origin = None;
    }
    let ttf = export(&font, &|_| 0, &ExportOptions::default()).unwrap();
    let ttf_face = FontRef::new(&ttf.bytes).unwrap();
    let compiled_vmtx = ttf_face.vmtx().unwrap();
    let loca = ttf_face.loca(None).unwrap();
    let glyf = ttf_face.glyf().unwrap();
    let otf = export(&font, &|_| 0, &options()).unwrap();
    let otf_face = FontRef::new(&otf.bytes).unwrap();
    for id in 0..ttf_face.maxp().unwrap().num_glyphs() {
        let gid = GlyphId::new(u32::from(id));
        let compiled_y_max = loca
            .get_glyf(gid, &glyf)
            .unwrap()
            .map_or(0, |glyph| glyph.y_max());
        let expected =
            i32::from(compiled_y_max) + i32::from(compiled_vmtx.side_bearing(gid).unwrap());
        assert_eq!(
            i32::from(otf_face.vorg().unwrap().vertical_origin_y(gid)),
            expected
        );
        assert_eq!(
            otf_face.vmtx().unwrap().advance(gid),
            compiled_vmtx.advance(gid)
        );
    }
}

#[test]
fn edited_origins_replace_imported_vorg_values() {
    let source = export(&vertical_font(), &|_| 0, &options()).unwrap();
    let mut imported = fc_sfnt::read_bytes(&source.bytes, &fc_sfnt::ReadOptions::default())
        .unwrap()
        .fonts
        .remove(0);
    assert!(imported
        .extra
        .raw_tables
        .contains_key(&fc_model::Tag(*b"VORG")));
    imported.glyph_mut("A").unwrap().layers[0].vertical_origin = Some(975.0);
    let report = export(&imported, &|_| 1, &options()).expect("export edited origin");
    let face = FontRef::new(&report.bytes).unwrap();
    let a = gid_for(&face, 0x41);
    assert_eq!(face.vorg().unwrap().vertical_origin_y(a), 975);
    assert_eq!(face.vmtx().unwrap().side_bearing(a), Some(374));
}

#[test]
fn master_default_origin_is_used_when_a_layer_has_none() {
    let mut font = vertical_font();
    font.masters[0].metrics.default_vertical_origin = Some(925.0);
    font.glyph_mut("A").unwrap().layers[0].vertical_origin = None;
    let report = export(&font, &|_| 0, &options()).unwrap();
    let face = FontRef::new(&report.bytes).unwrap();
    let a = gid_for(&face, 0x41);
    let v = gid_for(&face, 0x56);
    assert_eq!(face.vorg().unwrap().default_vert_origin_y(), 925);
    assert_eq!(face.vorg().unwrap().vertical_origin_y(a), 925);
    assert_eq!(face.vorg().unwrap().vertical_origin_y(v), 850);
    assert_eq!(face.vmtx().unwrap().side_bearing(a), Some(324));
}

#[test]
fn vertical_origins_use_opentype_rounding_for_negative_halves() {
    let mut font = vertical_font();
    font.glyph_mut("space").unwrap().layers[0].vertical_origin = Some(-100.5);
    let report = export(&font, &|_| 0, &options()).unwrap();
    let face = FontRef::new(&report.bytes).unwrap();
    assert_eq!(
        face.vorg().unwrap().vertical_origin_y(gid_for(&face, 0x20)),
        -100
    );
}

#[test]
fn horizontal_fonts_do_not_gain_vertical_tables() {
    let report = export(&common::scratch_font(), &|_| 0, &options()).unwrap();
    let face = FontRef::new(&report.bytes).unwrap();
    for tag in [b"vhea", b"vmtx", b"VORG"] {
        assert!(face.table_data(read_fonts::types::Tag::new(tag)).is_none());
    }
}

#[test]
fn vertical_metric_overflow_is_reported_instead_of_wrapping() {
    let mut font = vertical_font();
    // Master defaults are not passed through fontc's per-layer narrowing, so
    // this exercises validation in the final CFF vertical metadata writer.
    font.masters[0].metrics.default_vertical_origin = Some(40000.0);
    let error = export(&font, &|_| 0, &options())
        .expect_err("out-of-range origin")
        .to_string();
    assert!(error.contains("vertical origin 40000"), "{error}");
}

#[test]
fn large_individual_bottom_sidebearing_is_valid_when_the_aggregate_fits() {
    let mut font = vertical_font();
    font.glyph_mut("A").unwrap().layers[0].vertical_width = Some(60000.0);
    let report = export(&font, &|_| 0, &options()).expect("large valid advance height");
    let face = FontRef::new(&report.bytes).unwrap();
    assert_eq!(face.vhea().unwrap().advance_height_max().to_u16(), 60000);
    assert_eq!(face.vhea().unwrap().min_bottom_side_bearing().to_i16(), 350);
}

#[test]
fn empty_vertical_fonts_have_zero_bounds_extrema_but_keep_metrics() {
    let mut font = vertical_font();
    for glyph in font.glyphs.values_mut() {
        glyph.layers[0].contours.clear();
    }
    let report = export(&font, &|_| 0, &options()).expect("empty outlines with vertical metrics");
    let face = FontRef::new(&report.bytes).unwrap();
    let vhea = face.vhea().unwrap();
    assert_eq!(vhea.min_top_side_bearing().to_i16(), 0);
    assert_eq!(vhea.min_bottom_side_bearing().to_i16(), 0);
    assert_eq!(vhea.y_max_extent().to_i16(), 0);
    assert_eq!(vhea.advance_height_max().to_u16(), 1200);
    assert_eq!(
        face.vorg().unwrap().vertical_origin_y(gid_for(&face, 0x41)),
        900
    );
}
