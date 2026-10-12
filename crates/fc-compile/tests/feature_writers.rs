mod common;

use fc_compile::{compile_static, shaping_fingerprint, CompileOptions};
use fc_model::{Anchor, FeatureBlock, Glyph, Layer};

fn compile(font: &fc_model::Font) -> Vec<u8> {
    compile_static(font, &font.masters[0].id, &CompileOptions::default()).unwrap()
}

#[test]
fn source_owned_kerning_wins_and_explicit_policy_suppresses_markers() {
    let mut font = common::scratch_font();
    font.features.features.push(FeatureBlock {
        tag: "kern".into(),
        code: "pos A V -80;".into(),
        ..Default::default()
    });
    assert_eq!(
        shaping_fingerprint(&compile(&font), "AV").unwrap()[0].1,
        520
    );
    font.features.features[0].code = "# Automatic Code\npos A V -80;".into();
    font.feature_settings.writer_policy.kerning = false;
    assert_eq!(
        shaping_fingerprint(&compile(&font), "AV").unwrap()[0].1,
        520
    );
    assert_eq!(font.masters[0].kerning_h.0.len(), 1);
}

#[test]
fn automatic_code_marker_inserts_model_kerning_exactly_once() {
    let mut font = common::scratch_font();
    font.features.features.push(FeatureBlock {
        tag: "kern".into(),
        code: "# Automatic Code\n".into(),
        ..Default::default()
    });
    assert_eq!(
        shaping_fingerprint(&compile(&font), "AV").unwrap()[0].1,
        550
    );
    font.feature_settings.writer_policy.kerning = false;
    assert_eq!(
        shaping_fingerprint(&compile(&font), "AV").unwrap()[0].1,
        600
    );
}

#[test]
fn mixed_manual_and_model_rules_with_a_marker_require_explicit_ownership() {
    let mut font = common::scratch_font();
    font.features.features.push(FeatureBlock {
        tag: "kern".into(),
        code: "# Automatic Code\npos A V -80;".into(),
        ..Default::default()
    });
    let result =
        fc_compile::validate_feature_candidate(&font, &font.features, &font.feature_settings);
    assert!(!result.is_valid());
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.code == "feature-writer-conflict" && d.source_id.is_some()));
    assert!(compile_static(&font, &font.masters[0].id, &CompileOptions::default()).is_err());
    font.features.features[0]
        .code
        .push_str(&format!("\n#{}", "padding".repeat(160_000)));
    assert!(compile_static(&font, &font.masters[0].id, &CompileOptions::default()).is_err());
    font.features.features[0].code = "# Automatic Code\npos A V -80;".into();
    font.feature_settings.writer_policy.kerning = false;
    assert_eq!(
        shaping_fingerprint(&compile(&font), "AV").unwrap()[0].1,
        520
    );
}

fn anchor_font() -> fc_model::Font {
    let mut font = common::scratch_font();
    font.features.prefixes.push(FeatureBlock {
        code: "languagesystem DFLT dflt;\nlanguagesystem latn dflt;".into(),
        ..Default::default()
    });
    font.glyphs.get_mut("A").unwrap().layers[0]
        .anchors
        .push(Anchor {
            name: "top".into(),
            x: 200.0,
            y: 700.0,
        });
    let mut mark = Glyph::new("acutecomb");
    mark.unicodes.push(0x301);
    let mut layer = Layer::new_master(font.masters[0].id.clone());
    layer.width = 0.0;
    layer.anchors.push(Anchor {
        name: "_top".into(),
        x: 10.0,
        y: 20.0,
    });
    mark.layers.push(layer);
    font.insert_glyph(mark).unwrap();
    font
}

fn offsets(bytes: &[u8]) -> Vec<(i32, i32)> {
    let face = read_fonts::FontRef::new(bytes).unwrap();
    let data = harfrust::ShaperData::new(&face);
    let shaper = data.shaper(&face).build();
    let mut buffer = harfrust::UnicodeBuffer::new();
    buffer.push_str("A\u{301}");
    buffer.set_script(harfrust::script::LATIN);
    buffer.guess_segment_properties();
    let shaped = shaper.shape(buffer, Default::default());
    shaped
        .glyph_positions()
        .iter()
        .map(|p| (p.x_offset, p.y_offset))
        .collect()
}

#[test]
fn source_owned_marks_keep_anchors_without_executing_the_model_writer() {
    let mut font = anchor_font();
    font.features.prefixes.push(FeatureBlock {
        code: "markClass acutecomb <anchor 10 20> @TOP;".into(),
        ..Default::default()
    });
    font.features.features.push(FeatureBlock {
        tag: "mark".into(),
        code: "# Automatic Code\npos base A <anchor 100 300> mark @TOP;".into(),
        ..Default::default()
    });
    font.feature_settings.writer_policy.anchors = false;
    let positioned = offsets(&compile(&font));
    assert_eq!(positioned[1], (-510, 280));
    assert_eq!(font.glyphs["A"].layers[0].anchors.len(), 1);
    // Disabled model writers do not validate or normalize preserved anchors.
    let anchor = font.glyphs["A"].layers[0].anchors[0].clone();
    font.glyphs.get_mut("A").unwrap().layers[0]
        .anchors
        .push(anchor);
    assert_eq!(offsets(&compile(&font)), positioned);
}

#[test]
fn automatic_code_marker_inserts_model_marks_exactly_once() {
    let mut font = anchor_font();
    font.features.features.push(FeatureBlock {
        tag: "mark".into(),
        code: "# Automatic Code\n".into(),
        ..Default::default()
    });
    assert_eq!(offsets(&compile(&font))[1], (-410, 680));
}
