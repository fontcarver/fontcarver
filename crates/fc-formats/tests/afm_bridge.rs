//! AFM model conversion and all-or-nothing metrics application.

use fc_formats::metrics::{
    apply_to_font, from_font, into_font, read_afm, read_afm_path, write_afm_font,
};
use fc_model::{Component, Contour, FeatureBlock, Font, Glyph, KernSide, Layer, MasterId};
use kurbo::{Affine, BezPath};

const AFM: &str = "StartFontMetrics 4.1\nComment bridge fixture\nFontName Bridge-Medium\nFullName Bridge Medium\nFamilyName Bridge\nWeight Medium\nItalicAngle -12\nIsFixedPitch false\nFontBBox -10 -200 900 800\nUnderlinePosition -100\nUnderlineThickness 50\nCapHeight 700\nXHeight 450\nAscender 800\nDescender -200\nEncodingScheme FontSpecific\nStartCharMetrics 4\nC 17 ; WX 600.5 ; N A ; B -10 0 550 700 ; L V custom ;\nC 65 ; WX 650 ; N V ; B 0 -10 640 700 ;\nC 66 ; WX 400 ; N custom ; B 0 0 0 0 ;\nC -1 ; WX 610 ; N A.alt ; B 0 0 590 700 ;\nEndCharMetrics\nStartKernData\nStartKernPairs 1\nKPX A V -80.5\nEndKernPairs\nEndKernData\nEndFontMetrics\n";

fn source() -> fc_formats::metrics::MetricsFont {
    read_afm(AFM).unwrap()
}

fn font() -> Font {
    into_font(source()).unwrap()
}

fn pair(left: &str, right: &str) -> (KernSide, KernSide) {
    (KernSide::Glyph(left.into()), KernSide::Glyph(right.into()))
}

#[test]
fn import_keeps_metrics_without_inventing_outlines_or_unicode_codes() {
    let font = font();
    let master = &font.default_master().id;
    assert_eq!(font.info.units_per_em, 1000);
    assert_eq!(font.info.family_name, "Bridge");
    assert_eq!(font.info.style_name, "Medium");
    assert_eq!(font.masters[0].metrics.italic_angle, -12.0);
    assert_eq!(font.masters[0].metrics.cap_height, 700.0);
    assert_eq!(font.glyph("A").unwrap().unicodes, [65]);
    assert_eq!(font.glyph("V").unwrap().unicodes, [86]);
    assert!(font.glyph("custom").unwrap().unicodes.is_empty());
    assert!(font.glyph("A.alt").unwrap().unicodes.is_empty());
    assert!(font.glyphs.values().all(|glyph| glyph
        .master_layer(master)
        .unwrap()
        .contours
        .is_empty()));
    assert_eq!(
        font.glyph("A").unwrap().master_layer(master).unwrap().width,
        600.5
    );
    assert_eq!(font.masters[0].kerning_h.0[&pair("A", "V")], -80.5);
    let retained: fc_formats::metrics::MetricsFont =
        serde_json::from_value(font.user_data["org.fontcarver.afm"].clone()).unwrap();
    assert_eq!(retained, source());
}

#[test]
fn imported_semantics_survive_font_json_and_path_roundtrip() {
    let original = source();
    let font: Font = serde_json::from_str(&serde_json::to_string(&font()).unwrap()).unwrap();
    assert_eq!(
        from_font(&font, &font.default_master().id).unwrap(),
        original
    );
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join("metrics.afm");
    write_afm_font(&font, &font.default_master().id, &path).unwrap();
    let reopened = read_afm_path(&path).unwrap();
    assert_eq!(
        from_font(&reopened, &reopened.default_master().id).unwrap(),
        original
    );
}

#[test]
fn apply_scales_only_listed_widths_and_pairs_and_preserves_other_content() {
    let mut font = font();
    font.info.units_per_em = 2000;
    font.info.family_name = "Existing".into();
    font.masters[0].kerning_h.0.insert(pair("V", "A"), -7.0);
    let master = font.default_master().id.clone();
    let mut metrics = source();
    metrics.glyphs.retain(|glyph| glyph.name.as_ref() == "A");
    metrics.glyphs[0].ligatures.clear();
    let before = font.clone();
    apply_to_font(&metrics, &mut font, &master).unwrap();
    let mut expected = before;
    expected
        .glyph_mut("A")
        .unwrap()
        .master_layer_mut(&master)
        .unwrap()
        .width = 1201.0;
    expected.masters[0]
        .kerning_h
        .0
        .insert(pair("A", "V"), -161.0);
    assert_eq!(font, expected);
}

#[test]
fn apply_rejects_unknown_master_glyph_layer_and_nonfinite_input_atomically() {
    let masterless = MasterId::new();
    let mut font = font();
    let before = font.clone();
    assert!(apply_to_font(&source(), &mut font, &masterless).is_err());
    assert_eq!(font, before);
    let master = font.default_master().id.clone();
    let mut metrics = source();
    metrics.glyphs[3].name = "Missing".into();
    assert!(apply_to_font(&metrics, &mut font, &master).is_err());
    assert_eq!(font, before);
    font.glyph_mut("V").unwrap().layers.clear();
    let before = font.clone();
    assert!(apply_to_font(&source(), &mut font, &master).is_err());
    assert_eq!(font, before);
    metrics.glyphs[0].width = f64::NAN;
    assert!(apply_to_font(&metrics, &mut font, &master).is_err());
    assert_eq!(font, before);
}

#[test]
fn apply_validates_pair_targets_and_scaled_overflow_before_writing() {
    let mut font = font();
    let master = font.default_master().id.clone();
    let mut metrics = source();
    metrics.kerning[0].right = "Missing".into();
    let before = font.clone();
    assert!(apply_to_font(&metrics, &mut font, &master).is_err());
    assert_eq!(font, before);
    font.info.units_per_em = u16::MAX;
    let before = font.clone();
    metrics = source();
    metrics.glyphs[0].width = f64::MAX;
    assert!(apply_to_font(&metrics, &mut font, &master).is_err());
    assert_eq!(font, before);
}

#[test]
fn export_scales_widths_bounds_and_direct_pairs() {
    let mut font = font();
    font.info.units_per_em = 2000;
    let master = font.default_master().id.clone();
    let mut outline = BezPath::new();
    outline.move_to((20.0, -200.0));
    outline.line_to((900.0, 1400.0));
    outline.line_to((1100.0, 0.0));
    outline.close_path();
    let layer = font
        .glyph_mut("A")
        .unwrap()
        .master_layer_mut(&master)
        .unwrap();
    layer.width = 1300.0;
    layer.contours = Contour::from_bezpath(&outline);
    font.masters[0].kerning_h.0.insert(pair("A", "V"), -160.0);
    let metrics = from_font(&font, &master).unwrap();
    let a = metrics
        .glyphs
        .iter()
        .find(|glyph| glyph.name.as_ref() == "A")
        .unwrap();
    assert_eq!(a.width, 650.0);
    assert_eq!(a.bbox, Some([10.0, -100.0, 550.0, 700.0]));
    assert_eq!(a.code, Some(17));
    assert_eq!(metrics.kerning[0].value, -80.0);
}

#[test]
fn export_resolves_component_bounds() {
    let mut font = font();
    let master = font.default_master().id.clone();
    let mut outline = BezPath::new();
    outline.move_to((10.0, 20.0));
    outline.line_to((100.0, 200.0));
    outline.line_to((10.0, 200.0));
    outline.close_path();
    font.glyph_mut("A")
        .unwrap()
        .master_layer_mut(&master)
        .unwrap()
        .contours = Contour::from_bezpath(&outline);
    font.glyph_mut("V")
        .unwrap()
        .master_layer_mut(&master)
        .unwrap()
        .components
        .push(Component {
            base: "A".into(),
            transform: Affine::translate((30.0, 40.0)),
            ..Default::default()
        });
    let metrics = from_font(&font, &master).unwrap();
    assert_eq!(
        metrics
            .glyphs
            .iter()
            .find(|glyph| glyph.name.as_ref() == "V")
            .unwrap()
            .bbox,
        Some([40.0, 60.0, 130.0, 240.0])
    );
}

#[test]
fn fresh_model_has_no_guessed_character_encoding() {
    let mut font = Font::default();
    font.info.units_per_em = 1000;
    font.info.postscript_font_name = Some("Fresh-Regular".into());
    let master = font.default_master().id.clone();
    let mut glyph = Glyph::new("A");
    glyph.unicodes.push(65);
    let mut layer = Layer::new_master(master.clone());
    layer.width = 600.0;
    glyph.layers.push(layer);
    font.insert_glyph(glyph).unwrap();
    assert_eq!(from_font(&font, &master).unwrap().glyphs[0].code, None);
}

#[test]
fn export_rejects_unsupported_metrics_without_replacing_destination() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join("sentinel.afm");
    std::fs::write(&path, b"existing output").unwrap();
    let original = font();
    let master = original.default_master().id.clone();
    let mut variants = Vec::new();
    let mut font = original.clone();
    font.masters[0].kerning_h.0.insert(
        (KernSide::Group("A".into()), KernSide::Glyph("V".into())),
        -10.0,
    );
    variants.push(font);
    let mut font = original.clone();
    font.masters[0].kerning_v.0.insert(pair("A", "V"), -10.0);
    variants.push(font);
    let mut font = original.clone();
    font.features.features.push(FeatureBlock {
        tag: "kern".into(),
        code: "pos A V -20;".into(),
        ..Default::default()
    });
    variants.push(font);
    let mut font = original.clone();
    font.glyph_mut("A")
        .unwrap()
        .master_layer_mut(&master)
        .unwrap()
        .vertical_width = Some(1000.0);
    variants.push(font);
    let mut font = original.clone();
    font.info.units_per_em = 0;
    variants.push(font);
    let mut font = original.clone();
    font.user_data["org.fontcarver.afm"] = serde_json::json!({"broken": true});
    variants.push(font);
    let mut font = original;
    font.glyph_mut("A")
        .unwrap()
        .master_layer_mut(&master)
        .unwrap()
        .width = f64::INFINITY;
    variants.push(font);
    for font in variants {
        assert!(write_afm_font(&font, &master, &path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"existing output");
    }
}
