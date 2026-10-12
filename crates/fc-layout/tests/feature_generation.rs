//! Row 48/51 acceptance: every automatic feature family is generated from the
//! document, compiles, and actually changes shaping in the expected direction.

use fc_compile::{
    compile_static, generate_feature_source, validate_feature_candidate, CompileOptions,
    FeatureDiagnosticSeverity,
};
use fc_model::{
    Anchor, Component, Contour, FeatureGenerationSettings, FeatureGenerator, Font, Glyph, Layer,
};
use harfrust::{Direction, Feature, ShapeOptions, ShaperData, UnicodeBuffer};
use kurbo::BezPath;
use read_fonts::{types::Tag, FontRef};

/// One shaped glyph: the model name and its advance.
#[derive(Debug, PartialEq)]
struct Shaped {
    name: String,
    x_advance: i32,
}

fn glyph(font: &mut Font, name: &str, unicode: Option<u32>) {
    let master = font.default_master().id.clone();
    let mut glyph = Glyph::new(name);
    glyph.unicodes.extend(unicode);
    let mut layer = Layer::new_master(master);
    layer.width = 500.0;
    let mut path = BezPath::new();
    path.move_to((20.0, 0.0));
    path.line_to((420.0, 0.0));
    path.line_to((420.0, 600.0));
    path.line_to((20.0, 600.0));
    path.close_path();
    layer.contours = Contour::from_bezpath(&path);
    glyph.layers.push(layer);
    font.insert_glyph(glyph).expect("insert glyph");
}

/// A font with the inventory every enumerated recipe needs.
fn inventory() -> Font {
    let mut font = Font::new_empty("Generated features", 1000);
    for (name, unicode) in [
        (".notdef", None),
        ("space", Some(0x20)),
        ("a", Some(0x61)),
        ("A", Some(0x41)),
        ("o", Some(0x6f)),
        ("O", Some(0x4f)),
        ("f", Some(0x66)),
        ("i", Some(0x69)),
        ("j", Some(0x6a)),
        ("f_i", None),
        ("a.sc", None),
        ("o.sc", None),
        ("a.ss01", None),
        ("a.case", None),
        ("a.vert", None),
        ("a.vrt2", None),
        ("a.salt", None),
        ("one", Some(0x31)),
        ("two", Some(0x32)),
        ("one.lf", None),
        ("one.osf", None),
        ("one.tf", None),
        ("one.pnum", None),
        ("one.sups", None),
        ("one.sinf", None),
        ("one.numr", None),
        ("one.dnom", None),
        ("two.numr", None),
        ("two.dnom", None),
        ("period", Some(0x2e)),
        ("period.ordn", None),
        ("ordfeminine", Some(0xaa)),
        ("ordmasculine", Some(0xba)),
        ("fraction", Some(0x2044)),
        ("slash", Some(0x2f)),
        ("idotless", Some(0x131)),
        ("aacute", Some(0xe1)),
    ] {
        glyph(&mut font, name, unicode);
    }
    let master = font.default_master().id.clone();
    // A combining mark the model classifies, with a mark-attachment anchor.
    let mut mark = Glyph::new("acutecomb");
    mark.unicodes.push(0x301);
    mark.category = Some("Mark".into());
    let mut mark_layer = Layer::new_master(master.clone());
    mark_layer.width = 0.0;
    mark_layer.anchors.push(Anchor {
        name: "_top".into(),
        x: 0.0,
        y: 600.0,
    });
    mark.layers.push(mark_layer);
    font.insert_glyph(mark).expect("insert mark");
    // `aacute` is a real composite, which is what `ccmp` composition reads.
    let composed = font.glyph_mut("aacute").expect("composed glyph");
    let layer = composed
        .master_layer_mut(&master)
        .expect("composed master layer");
    layer.contours.clear();
    for base in ["a", "acutecomb"] {
        layer.components.push(Component {
            base: base.into(),
            ..Component::default()
        });
    }
    font.glyph_mut("a").expect("a").sub_category = Some("Lowercase".into());
    font.glyph_mut("A").expect("A").sub_category = Some("Uppercase".into());
    font
}

/// Generate the candidate, require it to validate, and install it.
fn apply(font: &mut Font, settings: FeatureGenerationSettings) -> Font {
    let generated = generate_feature_source(font, &font.features, &settings);
    let errors: Vec<_> = generated
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == FeatureDiagnosticSeverity::Error)
        .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
        .collect();
    assert!(errors.is_empty(), "generation errors: {errors:?}");
    let validation = validate_feature_candidate(font, &generated.source, &generated.settings);
    assert!(
        validation.is_valid(),
        "generated source does not compile: {:?}",
        validation
            .diagnostics
            .iter()
            .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
            .collect::<Vec<_>>()
    );
    let mut candidate = font.clone();
    candidate.features = generated.source;
    candidate.feature_settings = generated.settings;
    candidate
}

fn compiled(font: &Font) -> (Vec<u8>, Vec<fc_model::GlyphName>) {
    let compiled = fc_compile::compile_static_with_metadata(
        font,
        &font.default_master().id,
        &CompileOptions::default(),
    )
    .expect("compile generated font");
    (compiled.bytes, compiled.glyph_order)
}

fn shape_with(
    bytes: &[u8],
    order: &[fc_model::GlyphName],
    text: &str,
    features: &[(&str, bool)],
    vertical: bool,
) -> Vec<Shaped> {
    let face = FontRef::new(bytes).expect("parse compiled font");
    let data = ShaperData::new(&face);
    let shaper = data.shaper(&face).build();
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.guess_segment_properties();
    if vertical {
        buffer.set_direction(Direction::TopToBottom);
    }
    let requested: Vec<Feature> = features
        .iter()
        .map(|(tag, on)| {
            Feature::new(
                Tag::new_checked(tag.as_bytes()).expect("four-byte feature tag"),
                u32::from(*on),
                ..,
            )
        })
        .collect();
    let run = shaper.shape(buffer, ShapeOptions::new().features(&requested));
    run.glyph_infos()
        .iter()
        .zip(run.glyph_positions())
        .map(|(info, position)| Shaped {
            name: order
                .get(info.glyph_id as usize)
                .map(|name| name.to_string())
                .unwrap_or_else(|| format!("gid{}", info.glyph_id)),
            x_advance: position.x_advance,
        })
        .collect()
}

fn names(shaped: &[Shaped]) -> Vec<&str> {
    shaped.iter().map(|glyph| glyph.name.as_str()).collect()
}

#[test]
fn every_enumerated_generator_family_compiles_and_shapes() {
    let mut font = inventory();
    let settings = FeatureGenerationSettings {
        small_caps: true,
        caps_to_small_caps: true,
        stylistic_sets: true,
        case_forms: true,
        vertical_forms: true,
        ligatures: true,
        figure_styles: true,
        positional_figures: true,
        fractions: true,
        ordinals: true,
        composition: true,
        access_all_alternates: true,
        ..FeatureGenerationSettings::default()
    };
    let candidate = apply(&mut font, settings);
    let tags: Vec<&str> = candidate
        .features
        .features
        .iter()
        .map(|block| block.tag.as_str())
        .collect();
    for expected in [
        "smcp", "c2sc", "ss01", "case", "vert", "vrt2", "liga", "lnum", "onum", "tnum", "pnum",
        "sups", "sinf", "numr", "dnom", "frac", "ordn", "ccmp", "aalt",
    ] {
        assert!(
            tags.contains(&expected),
            "missing generated {expected} in {tags:?}"
        );
    }
    assert!(
        candidate
            .features
            .features
            .iter()
            .all(|block| block.automatic),
        "generated blocks must be marked automatic"
    );

    let (bytes, order) = compiled(&candidate);
    // Single substitutions: the suffixed alternate replaces the default glyph.
    for (tag, text, expected) in [
        ("smcp", "a", "a.sc"),
        ("c2sc", "A", "a.sc"),
        ("ss01", "a", "a.ss01"),
        ("case", "a", "a.case"),
        ("lnum", "1", "one.lf"),
        ("onum", "1", "one.osf"),
        ("tnum", "1", "one.tf"),
        ("pnum", "1", "one.pnum"),
        ("sups", "1", "one.sups"),
        ("sinf", "1", "one.sinf"),
        ("numr", "1", "one.numr"),
        ("dnom", "1", "one.dnom"),
    ] {
        assert_eq!(
            names(&shape_with(&bytes, &order, text, &[(tag, true)], false)),
            vec![expected],
            "{tag} substitution"
        );
        assert_eq!(
            names(&shape_with(&bytes, &order, text, &[(tag, false)], false))[0],
            match text {
                "a" => "a",
                "A" => "A",
                _ => "one",
            },
            "{tag} must be off by default for a non-default feature"
        );
    }
    // Ligature substitution (many to one) still comes from underscore names.
    assert_eq!(
        names(&shape_with(&bytes, &order, "fi", &[("liga", true)], false)),
        vec!["f_i"]
    );
    // Composition: base plus mark becomes the precomposed glyph, and the
    // dotless form replaces `i` before a mark.
    assert_eq!(
        names(&shape_with(&bytes, &order, "a\u{301}", &[], false)),
        vec!["aacute"]
    );
    assert_eq!(
        names(&shape_with(&bytes, &order, "i\u{301}", &[], false)),
        vec!["idotless", "acutecomb"]
    );
    // Ordinals apply only after a figure.
    assert_eq!(
        names(&shape_with(&bytes, &order, "1a", &[("ordn", true)], false)),
        vec!["one", "ordfeminine"]
    );
    assert_eq!(
        names(&shape_with(&bytes, &order, "1o", &[("ordn", true)], false)),
        vec!["one", "ordmasculine"]
    );
    assert_eq!(
        names(&shape_with(&bytes, &order, "1.", &[("ordn", true)], false)),
        vec!["one", "period.ordn"]
    );
    assert_eq!(
        names(&shape_with(&bytes, &order, "a1", &[("ordn", true)], false)),
        vec!["a", "one"],
        "ordn must not fire without a preceding figure"
    );
    // Fractions: multi-digit numerators and denominators both convert, and the
    // plain slash becomes the fraction slash between figures.
    assert_eq!(
        names(&shape_with(
            &bytes,
            &order,
            "12/12",
            &[("frac", true)],
            false
        )),
        vec!["one.numr", "two.numr", "fraction", "one.dnom", "two.dnom"]
    );
    assert_eq!(
        names(&shape_with(&bytes, &order, "a/a", &[("frac", true)], false)),
        vec!["a", "slash", "a"],
        "frac must not convert a slash outside figures"
    );
}

#[test]
fn vertical_alternates_shape_in_a_vertical_run_only() {
    let mut font = inventory();
    let candidate = apply(
        &mut font,
        FeatureGenerationSettings {
            vertical_forms: true,
            ..FeatureGenerationSettings::default()
        },
    );
    let vert = candidate
        .features
        .features
        .iter()
        .find(|block| block.tag == "vert")
        .expect("generated vert");
    let vrt2 = candidate
        .features
        .features
        .iter()
        .find(|block| block.tag == "vrt2")
        .expect("generated vrt2");
    assert!(vert.code.contains("sub \\a by \\a.vert;"), "{}", vert.code);
    assert!(!vert.code.contains("a.vrt2"), "{}", vert.code);
    // A `.vrt2` alternate is rotated and belongs only to `vrt2`.
    assert!(vrt2.code.contains("sub \\a by \\a.vert;"), "{}", vrt2.code);
    assert!(vrt2.code.contains("sub \\a by \\a.vrt2;"), "{}", vrt2.code);

    let (bytes, order) = compiled(&candidate);
    // `vert` is on by default in a vertical run and absent horizontally.
    assert_eq!(
        names(&shape_with(&bytes, &order, "a", &[], true)),
        vec!["a.vert"]
    );
    assert_eq!(
        names(&shape_with(&bytes, &order, "a", &[], false)),
        vec!["a"]
    );
    assert_eq!(
        names(&shape_with(&bytes, &order, "a", &[("vert", false)], true)),
        vec!["a"]
    );
}

#[test]
fn automatic_classes_are_generated_entries_a_user_can_hold_manual() {
    let mut font = inventory();
    let settings = FeatureGenerationSettings {
        automatic_classes: vec!["Uppercase".into(), "Figures".into()],
        ..FeatureGenerationSettings::default()
    };
    let candidate = apply(&mut font, settings.clone());
    let uppercase = candidate
        .features
        .classes
        .iter()
        .find(|class| class.name == "Uppercase")
        .expect("generated Uppercase class");
    assert!(uppercase.automatic);
    assert!(uppercase.code.contains("\\A"), "{}", uppercase.code);
    assert!(!uppercase.code.contains("\\a "), "{}", uppercase.code);
    assert_eq!(
        candidate
            .feature_settings
            .generated
            .iter()
            .filter(|record| record.generator == FeatureGenerator::Classes)
            .count(),
        2
    );
    // Holding one entry manual keeps its text and stops regenerating it.
    let frozen = fc_compile::freeze_feature_source(
        &candidate.features,
        &candidate.feature_settings,
        FeatureGenerator::Classes,
        Some("Figures"),
    );
    assert!(frozen.diagnostics.is_empty(), "{:?}", frozen.diagnostics);
    let figures = frozen
        .source
        .classes
        .iter()
        .find(|class| class.name == "Figures")
        .expect("frozen Figures class is kept");
    assert!(!figures.automatic);
    assert!(figures.code.contains("\\one"));
    assert!(frozen.settings.entry_is_manual("Figures"));
    assert!(frozen
        .settings
        .generated
        .iter()
        .any(|record| record.source_id.0.ends_with("Uppercase")));
    // Regenerating now leaves the frozen entry alone.
    let held = generate_feature_source(&font, &frozen.source, &frozen.settings);
    assert!(held.diagnostics.is_empty(), "{:?}", held.diagnostics);
    assert_eq!(
        held.source
            .classes
            .iter()
            .filter(|class| class.name == "Figures")
            .count(),
        1
    );
    assert!(held
        .source
        .classes
        .iter()
        .any(|class| class.name == "Uppercase" && class.automatic));
    // Freezing an entry the recipe does not own is refused.
    let refused = fc_compile::freeze_feature_source(
        &held.source,
        &held.settings,
        FeatureGenerator::Classes,
        Some("Figures"),
    );
    assert_eq!(
        refused.diagnostics.first().map(|d| d.code.as_str()),
        Some("feature-generation-not-generated")
    );
}

#[test]
fn a_user_owned_feature_still_blocks_its_generator_unless_held_manual() {
    let mut font = inventory();
    font.features.features.push(fc_model::FeatureBlock {
        tag: "ordn".into(),
        code: "sub \\a by \\a.sc;".into(),
        ..Default::default()
    });
    let settings = FeatureGenerationSettings {
        ordinals: true,
        ..FeatureGenerationSettings::default()
    };
    let blocked = generate_feature_source(&font, &font.features, &settings);
    assert!(blocked
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "feature-generation-conflict"));
    assert_eq!(blocked.source.features.len(), 1);

    let mut held = settings.clone();
    held.manual_entries.push("ordn".into());
    let allowed = generate_feature_source(&font, &font.features, &held);
    assert!(allowed.diagnostics.is_empty(), "{:?}", allowed.diagnostics);
    assert_eq!(allowed.source.features.len(), 1);
    assert_eq!(allowed.source.features[0].code, "sub \\a by \\a.sc;");
    let _ = compile_static(&font, &font.default_master().id, &CompileOptions::default())
        .expect("the user feature still compiles");
}
