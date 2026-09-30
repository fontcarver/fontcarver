//! Row 49 acceptance: `kern`, `mark`, `mkmk` and `curs` generated from model
//! kerning and anchors, and their precedence against hand-written source.

use std::collections::BTreeSet;

use fc_compile::{compile_static_with_metadata, validate_feature_candidate, CompileOptions};
use fc_model::{Anchor, Contour, FeatureBlock, Font, Glyph, KernSide, Layer};
use harfrust::{Feature, ShapeOptions, ShaperData, UnicodeBuffer};
use kurbo::BezPath;
use read_fonts::{types::Tag, FontRef, TableProvider};

fn glyph(font: &mut Font, name: &str, unicode: Option<u32>, width: f64) {
    let master = font.default_master().id.clone();
    let mut glyph = Glyph::new(name);
    glyph.unicodes.extend(unicode);
    let mut layer = Layer::new_master(master);
    layer.width = width;
    let mut path = BezPath::new();
    path.move_to((20.0, 0.0));
    path.line_to((width.max(80.0) - 20.0, 0.0));
    path.line_to((width.max(80.0) - 20.0, 600.0));
    path.line_to((20.0, 600.0));
    path.close_path();
    layer.contours = Contour::from_bezpath(&path);
    glyph.layers.push(layer);
    font.insert_glyph(glyph).expect("insert glyph");
}

fn anchor(font: &mut Font, glyph: &str, name: &str, x: f64, y: f64) {
    let master = font.default_master().id.clone();
    font.glyph_mut(glyph)
        .expect("glyph")
        .master_layer_mut(&master)
        .expect("master layer")
        .anchors
        .push(Anchor {
            name: name.into(),
            x,
            y,
        });
}

fn base_font() -> Font {
    let mut font = Font::new_empty("Model layout writers", 1000);
    for (name, unicode, width) in [
        (".notdef", None, 500.0),
        ("A", Some(0x41), 600.0),
        ("Agrave", Some(0xc0), 600.0),
        ("V", Some(0x56), 600.0),
        ("a", Some(0x61), 500.0),
        ("b", Some(0x62), 500.0),
        ("acutecomb", Some(0x301), 0.0),
        ("gravecomb", Some(0x300), 0.0),
    ] {
        glyph(&mut font, name, unicode, width);
    }
    font.features.prefixes.push(FeatureBlock {
        code: "languagesystem DFLT dflt;\nlanguagesystem latn dflt;".into(),
        ..Default::default()
    });
    font
}

fn compile(font: &Font) -> (Vec<u8>, Vec<fc_model::GlyphName>) {
    let compiled =
        compile_static_with_metadata(font, &font.default_master().id, &CompileOptions::default())
            .expect("compile font");
    (compiled.bytes, compiled.glyph_order)
}

/// Feature tags present in the compiled GPOS table.
fn gpos_features(bytes: &[u8]) -> BTreeSet<String> {
    let face = FontRef::new(bytes).expect("parse font");
    let Ok(gpos) = face.gpos() else {
        return BTreeSet::new();
    };
    gpos.feature_list()
        .expect("feature list")
        .feature_records()
        .iter()
        .map(|record| record.feature_tag().to_string())
        .collect()
}

struct Position {
    name: String,
    x_advance: i32,
    x_offset: i32,
    y_offset: i32,
}

fn shape(
    bytes: &[u8],
    order: &[fc_model::GlyphName],
    text: &str,
    features: &[(&str, bool)],
) -> Vec<Position> {
    let face = FontRef::new(bytes).expect("parse font");
    let data = ShaperData::new(&face);
    let shaper = data.shaper(&face).build();
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.set_script(harfrust::script::LATIN);
    buffer.guess_segment_properties();
    let requested: Vec<Feature> = features
        .iter()
        .map(|(tag, on)| {
            Feature::new(
                Tag::new_checked(tag.as_bytes()).expect("four-byte tag"),
                u32::from(*on),
                ..,
            )
        })
        .collect();
    let run = shaper.shape(buffer, ShapeOptions::new().features(&requested));
    run.glyph_infos()
        .iter()
        .zip(run.glyph_positions())
        .map(|(info, position)| Position {
            name: order
                .get(info.glyph_id as usize)
                .map(|name| name.to_string())
                .unwrap_or_default(),
            x_advance: position.x_advance,
            x_offset: position.x_offset,
            y_offset: position.y_offset,
        })
        .collect()
}

#[test]
fn model_kerning_writes_class_pairs_and_glyph_exceptions() {
    let mut font = base_font();
    font.kerning_groups.side1.insert(
        "public.kern1.A".into(),
        ["A".into(), "Agrave".into()].into_iter().collect(),
    );
    font.kerning_groups
        .side2
        .insert("public.kern2.V".into(), ["V".into()].into_iter().collect());
    let kerning = &mut font.masters[0].kerning_h.0;
    kerning.insert(
        (
            KernSide::Group("public.kern1.A".into()),
            KernSide::Group("public.kern2.V".into()),
        ),
        -80.0,
    );
    // A glyph pair inside the same classes must win as an exception.
    kerning.insert(
        (
            KernSide::Glyph("Agrave".into()),
            KernSide::Group("public.kern2.V".into()),
        ),
        -20.0,
    );
    let (bytes, order) = compile(&font);
    assert!(gpos_features(&bytes).contains("kern"));
    let class_pair = shape(&bytes, &order, "AV", &[]);
    assert_eq!(class_pair[0].name, "A");
    assert_eq!(class_pair[0].x_advance, 520, "class pair kerning");
    let exception = shape(&bytes, &order, "\u{c0}V", &[]);
    assert_eq!(exception[0].name, "Agrave");
    assert_eq!(exception[0].x_advance, 580, "glyph pair exception");
    // Turning the writer off removes every model pair without touching data.
    font.feature_settings.writer_policy.kerning = false;
    let (plain, plain_order) = compile(&font);
    assert!(!gpos_features(&plain).contains("kern"));
    assert_eq!(shape(&plain, &plain_order, "AV", &[])[0].x_advance, 600);
    assert_eq!(font.masters[0].kerning_h.0.len(), 2);
}

#[test]
fn model_anchors_write_mark_mkmk_and_curs_together() {
    let mut font = base_font();
    anchor(&mut font, "a", "top", 250.0, 600.0);
    anchor(&mut font, "acutecomb", "_top", 0.0, 0.0);
    // A stacking anchor on the mark itself is what turns on mark-to-mark.
    anchor(&mut font, "acutecomb", "top", 0.0, 800.0);
    anchor(&mut font, "gravecomb", "_top", 0.0, 0.0);
    // Cursive attachment comes from the exit/entry anchor pair.
    anchor(&mut font, "a", "exit", 480.0, 100.0);
    anchor(&mut font, "b", "entry", 20.0, 300.0);

    let (bytes, order) = compile(&font);
    let features = gpos_features(&bytes);
    for tag in ["mark", "mkmk", "curs"] {
        assert!(features.contains(tag), "missing {tag} in {features:?}");
    }
    let stacked = shape(&bytes, &order, "a\u{301}\u{300}", &[]);
    assert_eq!(
        stacked.iter().map(|g| g.name.as_str()).collect::<Vec<_>>(),
        vec!["a", "acutecomb", "gravecomb"]
    );
    // The first mark lands on the base anchor, the second on the first mark.
    assert_eq!((stacked[1].x_offset, stacked[1].y_offset), (-250, 600));
    assert_eq!((stacked[2].x_offset, stacked[2].y_offset), (-250, 1400));
    // Disabling mkmk alone leaves the base attachment intact.
    let without_mkmk = shape(&bytes, &order, "a\u{301}\u{300}", &[("mkmk", false)]);
    assert_eq!(
        (without_mkmk[1].x_offset, without_mkmk[1].y_offset),
        (-250, 600)
    );
    assert_ne!(
        (without_mkmk[2].x_offset, without_mkmk[2].y_offset),
        (stacked[2].x_offset, stacked[2].y_offset)
    );
    // Cursive attachment moves the joined pair vertically.
    let joined = shape(&bytes, &order, "ab", &[("curs", true)]);
    assert_ne!(
        (joined[0].y_offset, joined[1].y_offset),
        (0, 0),
        "cursive attachment did not move the pair"
    );

    font.feature_settings.writer_policy.anchors = false;
    let (plain, _) = compile(&font);
    let plain_features = gpos_features(&plain);
    for tag in ["mark", "mkmk", "curs"] {
        assert!(
            !plain_features.contains(tag),
            "{tag} survived a disabled anchor writer"
        );
    }
}

#[test]
fn hand_written_source_owns_its_tag_and_the_marker_merges_model_rules() {
    let mut font = base_font();
    font.masters[0].kerning_h.0.insert(
        (KernSide::Glyph("A".into()), KernSide::Glyph("V".into())),
        -80.0,
    );
    // 1. A manual feature without the marker owns the tag: the model's pairs are
    //    skipped and only the authored value applies.
    font.features.features.push(FeatureBlock {
        tag: "kern".into(),
        code: "pos A V -30;".into(),
        ..Default::default()
    });
    let (owned, owned_order) = compile(&font);
    assert_eq!(shape(&owned, &owned_order, "AV", &[])[0].x_advance, 570);

    // 2. The `# Automatic Code` marker asks for the model's pairs instead.
    font.features.features[0].code = "# Automatic Code\n".into();
    let (merged, merged_order) = compile(&font);
    assert_eq!(shape(&merged, &merged_order, "AV", &[])[0].x_advance, 520);

    // 3. Mixing the marker with authored rules of the same kind is rejected
    //    before activation instead of silently choosing one of them.
    font.features.features[0].code = "# Automatic Code\npos A V -30;".into();
    let conflict = validate_feature_candidate(&font, &font.features, &font.feature_settings);
    assert!(!conflict.is_valid());
    assert!(conflict
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "feature-writer-conflict"));

    // 4. Disabling the writer hands the tag back to the authored rules.
    font.feature_settings.writer_policy.kerning = false;
    let policy = validate_feature_candidate(&font, &font.features, &font.feature_settings);
    assert!(policy.is_valid(), "{:?}", policy.diagnostics);
    let (explicit, explicit_order) = compile(&font);
    assert_eq!(
        shape(&explicit, &explicit_order, "AV", &[])[0].x_advance,
        570
    );
}

#[test]
fn hand_written_mark_source_and_model_anchors_follow_the_same_precedence() {
    let mut font = base_font();
    anchor(&mut font, "a", "top", 250.0, 600.0);
    anchor(&mut font, "acutecomb", "_top", 0.0, 0.0);
    font.features.prefixes.push(FeatureBlock {
        code: "markClass \\acutecomb <anchor 0 0> @MANUAL_TOP;".into(),
        ..Default::default()
    });
    font.features.features.push(FeatureBlock {
        tag: "mark".into(),
        code: "pos base \\a <anchor 100 300> mark @MANUAL_TOP;".into(),
        ..Default::default()
    });
    font.feature_settings.writer_policy.anchors = false;
    let (owned, owned_order) = compile(&font);
    let manual = shape(&owned, &owned_order, "a\u{301}", &[]);
    assert_eq!((manual[1].x_offset, manual[1].y_offset), (-400, 300));

    // With the writer enabled and no marker, the authored `mark` feature still
    // owns its tag; the model's own attachment is not added a second time.
    font.feature_settings.writer_policy.anchors = true;
    let (both, both_order) = compile(&font);
    assert_eq!(
        shape(&both, &both_order, "a\u{301}", &[])
            .iter()
            .map(|g| (g.x_offset, g.y_offset))
            .collect::<Vec<_>>(),
        manual
            .iter()
            .map(|g| (g.x_offset, g.y_offset))
            .collect::<Vec<_>>()
    );
}
