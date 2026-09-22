use fc_compile::{export, shaping_fingerprint, ExportOptions};
use fc_formats::glyphs::{read_glyphs, write_glyphs};
use fc_model::{
    Axis, Component, FeatureBlock, Font, Glyph, KernSide, Layer, Master, Point, PointKind, Tag,
};

const NATIVE_V2: &str = r#"{
familyName = Native; unitsPerEm = 1000; versionMajor = 1; versionMinor = 0;
fontMaster = ({ id = M1; name = Regular; ascender = 800; descender = -200; });
glyphs = (
{ glyphname = A; unicode = 0041; rightKerningGroup = A;
  vendorUnknown = { text = preserved; };
  layers = ({ layerId = M1; width = 600; paths = ({ closed = 1;
    nodes = ("0 0 LINE", "200 700 LINE", "400 0 LINE"); });
    anchors = ({ name = top; position = "{200, 700}"; }); }); },
{ glyphname = B; unicode = 0042; leftKerningGroup = B;
  layers = ({ layerId = M1; width = 610; components = ({ name = A; transform = "{1, 0, 0.2, 1, 20, 30}"; }); }); }
);
kerning = { M1 = { "@MMK_L_A" = { "@MMK_R_B" = -60; }; A = { B = -75; }; }; };
featurePrefixes = ({ name = Languages; code = "languagesystem DFLT dflt;"; });
features = ({ name = liga; code = "sub A A by B;"; });
customParameters = ({ name = MyVendorParameter; value = (one, two); });
}"#;

fn sample() -> Font {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("native.glyphs");
    std::fs::write(&path, NATIVE_V2).unwrap();
    read_glyphs(&path).unwrap()
}

#[test]
fn native_bracket_rules_roundtrip_and_external_edits_take_precedence() {
    let input = NATIVE_V2.replace("name = Regular;", "name = Regular; weightValue = 100;")
        .replace("fontMaster = ({", "fontMaster = ({ id = M2; name = Bold; weightValue = 900; },{")
        .replace("layers = ({ layerId = M1;", "layers = ({ layerId = M2; width = 700; }, { layerId = ALT; associatedMasterId = M1; name = \"[500]\"; width = 650; },{ layerId = M1;");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("conditional.glyphs");
    std::fs::write(&path, input).unwrap();
    let font = read_glyphs(&path).unwrap();
    let kind = &font
        .glyph("A")
        .unwrap()
        .layers
        .iter()
        .find(|l| l.name == "[500]")
        .unwrap()
        .kind;
    assert!(matches!(
        kind,
        fc_model::LayerKind::Bracket {
            min: Some(500.),
            max: None,
            ..
        }
    ));
    write_glyphs(&font, &path).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("axisRules"));
    let reopened = read_glyphs(&path).unwrap();
    assert_eq!(
        &reopened
            .glyph("A")
            .unwrap()
            .layers
            .iter()
            .find(|l| l.name == "[500]")
            .unwrap()
            .kind,
        kind
    );
    std::fs::write(&path, text.replace("min = 500;", "min = 600;")).unwrap();
    let edited = read_glyphs(&path).unwrap();
    assert!(edited.glyph("A").unwrap().layers.iter().any(|l| matches!(
        l.kind,
        fc_model::LayerKind::Bracket {
            min: Some(600.),
            ..
        }
    )));
}

#[test]
fn reads_native_v2_and_writes_native_v3_with_layout() {
    let font = sample();
    assert_eq!(font.info.family_name, "Native");
    assert_eq!(font.glyph("A").unwrap().unicodes, [65]);
    assert_eq!(font.glyph("A").unwrap().layers[0].anchors[0].y, 700.0);
    assert_eq!(
        font.glyph("B").unwrap().layers[0].components[0]
            .transform
            .as_coeffs(),
        [1., 0., 0.2, 1., 20., 30.]
    );
    assert_eq!(font.kerning_groups.side1["A"].len(), 1);
    assert_eq!(
        font.masters[0].kerning_h.0[&(KernSide::Glyph("A".into()), KernSide::Glyph("B".into()))],
        -75.
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("roundtrip.glyphs");
    write_glyphs(&font, &path).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("MyVendorParameter"));
    assert!(text.contains("vendorUnknown"));
    let native = glyphs_reader::Font::load(&path).unwrap();
    assert_eq!(native.glyphs["A"].layers[0].width.into_inner(), 600.);
    assert_eq!(native.glyphs["B"].layers[0].shapes.len(), 1);
    let reopened = read_glyphs(&path).unwrap();
    assert_eq!(reopened.features.to_fea(), font.features.to_fea());
    assert_eq!(reopened.masters[0].kerning_h, font.masters[0].kerning_h);
    for text in ["AA", "AB", "BAA"] {
        let before = export(&font, &|_| 0, &ExportOptions::default())
            .unwrap()
            .bytes;
        let after = export(&reopened, &|_| 0, &ExportOptions::default())
            .unwrap()
            .bytes;
        assert_eq!(
            shaping_fingerprint(&before, text).unwrap(),
            shaping_fingerprint(&after, text).unwrap()
        );
    }
}

#[test]
fn native_external_edits_override_metadata() {
    let mut font = sample();
    font.glyph_mut("A")
        .unwrap()
        .tags
        .insert("retained tag".into());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("edit.glyphs");
    write_glyphs(&font, &path).unwrap();
    let text = std::fs::read_to_string(&path)
        .unwrap()
        .replace("width = 600;", "width = 777;")
        .replace("sub A A by B;", "sub A B by B;");
    std::fs::write(&path, text).unwrap();
    let edited = read_glyphs(&path).unwrap();
    assert_eq!(edited.glyph("A").unwrap().layers[0].width, 777.);
    assert!(edited.glyph("A").unwrap().tags.contains("retained tag"));
    assert!(edited.features.to_fea().contains("sub A B by B;"));
}

#[test]
fn multiple_masters_curves_components_and_extras_roundtrip() {
    let mut font = Font::new_empty("Curves", 2048);
    font.axes.push(Axis {
        tag: Tag(*b"wght"),
        name: "Weight".into(),
        min: 100.,
        default: 100.,
        max: 900.,
        ..Axis::default()
    });
    font.masters[0].location.insert(Tag(*b"wght"), 100.);
    let mut bold = Master {
        name: "Bold".into(),
        ..Master::default()
    };
    bold.location.insert(Tag(*b"wght"), 900.);
    font.add_master(bold);
    let mut glyph = Glyph::new("A");
    glyph.unicodes = vec![65, 0x391];
    for master in &font.masters {
        let mut layer = Layer::new_master(master.id.clone());
        layer.width = 600.;
        layer.contours.push(fc_model::Contour {
            closed: true,
            points: vec![
                Point {
                    x: 0.,
                    y: 0.,
                    kind: PointKind::Line,
                    ..Point::default()
                },
                Point {
                    x: 0.,
                    y: 100.,
                    kind: PointKind::OffCurve,
                    ..Point::default()
                },
                Point {
                    x: 100.,
                    y: 100.,
                    kind: PointKind::OffCurve,
                    ..Point::default()
                },
                Point {
                    x: 100.,
                    y: 0.,
                    kind: PointKind::Curve,
                    smooth: true,
                    ..Point::default()
                },
                Point {
                    x: 50.,
                    y: -50.,
                    kind: PointKind::OffCurve,
                    ..Point::default()
                },
                Point {
                    x: 0.,
                    y: -10.,
                    kind: PointKind::QCurve,
                    ..Point::default()
                },
            ],
            ..fc_model::Contour::default()
        });
        glyph.layers.push(layer);
    }
    font.insert_glyph(glyph).unwrap();
    let mut component = Glyph::new("A.alt");
    for master in &font.masters {
        let mut layer = Layer::new_master(master.id.clone());
        layer.components.push(Component {
            base: "A".into(),
            transform: kurbo::Affine::new([0.8, 0.3, 0.2, 1.1, 22., 33.]),
            ..Component::default()
        });
        component.layers.push(layer);
    }
    font.insert_glyph(component).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("multi.glyphs");
    write_glyphs(&font, &path).unwrap();
    let native = glyphs_reader::Font::load(&path).unwrap();
    assert_eq!(native.masters.len(), 2);
    let reopened = read_glyphs(&path).unwrap();
    assert_eq!(reopened.axes, font.axes);
    assert_eq!(
        reopened.glyph("A").unwrap().unicodes,
        font.glyph("A").unwrap().unicodes
    );
    for index in 0..2 {
        assert_eq!(
            reopened.glyph("A").unwrap().layers[index].contours,
            font.glyph("A").unwrap().layers[index].contours
        );
        let a = reopened.glyph("A.alt").unwrap().layers[index].components[0]
            .transform
            .as_coeffs();
        let b = font.glyph("A.alt").unwrap().layers[index].components[0]
            .transform
            .as_coeffs();
        assert!(a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-10));
    }
}

#[test]
fn rejects_malformed_unsupported_and_deep_inputs() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.glyphs");
    for input in [
        "{".into(),
        format!("{}0{}", "(".repeat(100), ")".repeat(100)),
        format!("{NATIVE_V2} trailing"),
        NATIVE_V2.replace("width = 600", "width = nan"),
        NATIVE_V2.replace(
            "glyphname = A;",
            "glyphname = A; partsSettings = ({ name = Width; bottomValue = 0; topValue = 100; });",
        ),
    ] {
        std::fs::write(&path, input).unwrap();
        assert!(read_glyphs(&path).is_err());
    }
}

#[test]
fn rejects_drafts_and_invalid_geometry_without_touching_destination() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("unchanged.glyphs");
    std::fs::write(&path, "keep me").unwrap();
    let mut font = sample();
    font.glyph_mut("A").unwrap().layers[0].width = f64::NAN;
    assert!(write_glyphs(&font, &path).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "keep me");
    font = sample();
    font.features.source_of_truth = fc_model::FeaturesSource::ImportedTables;
    assert!(write_glyphs(&font, &path).is_err());
    font = sample();
    let mut source = font.features.clone();
    source.prefixes.push(FeatureBlock {
        tag: "draft".into(),
        code: "invalid".into(),
        ..FeatureBlock::default()
    });
    font.feature_draft = Some(fc_model::FeatureDraft {
        source,
        ..fc_model::FeatureDraft::default()
    });
    assert!(write_glyphs(&font, &path).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "keep me");
}

#[test]
fn external_native_source_roundtrip() {
    let Some(path) = std::env::var_os("FC_GLYPHS_SOURCE") else {
        return;
    };
    let path = std::path::Path::new(&path);
    let started = std::time::Instant::now();
    let font = read_glyphs(path).unwrap();
    let native = glyphs_reader::Font::load(path).unwrap();
    assert_eq!(font.glyphs.len(), native.glyphs.len());
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("external.glyphs");
    write_glyphs(&font, &output).unwrap();
    let reopened = read_glyphs(&output).unwrap();
    let independent = glyphs_reader::Font::load(&output).unwrap();
    assert_eq!(reopened.glyphs.len(), font.glyphs.len());
    assert_eq!(independent.glyphs.len(), native.glyphs.len());
    for (name, glyph) in &font.glyphs {
        let other = &reopened.glyphs[name];
        assert_eq!(glyph.unicodes, other.unicodes, "{name}");
        for master in &font.masters {
            let a = glyph.master_layer(&master.id).unwrap();
            let b = other.master_layer(&master.id).unwrap();
            assert_eq!(a.width, b.width, "{name}");
            assert_eq!(a.contours, b.contours, "{name}");
            assert_eq!(a.anchors, b.anchors, "{name}");
        }
    }
    let before = export(&font, &|_| 0, &ExportOptions::default())
        .unwrap()
        .bytes;
    let after = export(&reopened, &|_| 0, &ExportOptions::default())
        .unwrap()
        .bytes;
    for text in ["ABC", "AV", "abc", "123"] {
        assert_eq!(
            shaping_fingerprint(&before, text).unwrap(),
            shaping_fingerprint(&after, text).unwrap()
        );
    }
    assert!(
        started.elapsed().as_secs() < 30,
        "native source roundtrip exceeded 30-second budget"
    );
}

#[test]
fn supplementary_layers_unknown_fields_and_native_deletions() {
    let input = NATIVE_V2.replace("layerId = M1; width = 600;", "layerId = M1; mysteryLayer = kept; width = 600;")
        .replace("layers = ({ layerId = M1; mysteryLayer", "layers = ({ layerId = DRAFT; associatedMasterId = M1; name = Draft; width = 450; mysteryDraft = retained; },{ layerId = M1; mysteryLayer");
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("draft.glyphs");
    std::fs::write(&source, input).unwrap();
    let font = read_glyphs(&source).unwrap();
    assert_eq!(font.glyph("A").unwrap().layers.len(), 2);
    write_glyphs(&font, &source).unwrap();
    let text = std::fs::read_to_string(&source).unwrap();
    assert!(text.contains("mysteryDraft = retained"));
    assert!(text.contains("mysteryLayer = kept"));
    let text = text
        .replace("unicode = (65);", "")
        .replace("kernRight = A;", "");
    // Also remove the class kerning referencing the now removed group.
    let mut raw = glyphs_reader::Plist::parse(&text).unwrap();
    if let glyphs_reader::Plist::Dictionary(root) = &mut raw {
        root.remove("kerningLTR");
    }
    std::fs::write(&source, raw.to_string()).unwrap();
    let reopened = read_glyphs(&source).unwrap();
    assert!(reopened.glyph("A").unwrap().unicodes.is_empty());
    assert!(reopened.glyph("A").unwrap().kern_group_side1.is_none());
}

#[test]
fn generated_feature_ownership_survives_and_external_edits_freeze_recipes() {
    let mut font = sample();
    font.features.features.clear();
    let mut ligature = font.glyph("B").unwrap().clone();
    ligature.name = "A_A".into();
    ligature.unicodes.clear();
    ligature.kern_group_side2 = None;
    font.insert_glyph(ligature).unwrap();
    let settings = fc_model::FeatureGenerationSettings {
        ligatures: true,
        ..Default::default()
    };
    let generated = fc_compile::generate_feature_source(&font, &font.features, &settings);
    assert!(
        generated.diagnostics.is_empty(),
        "{:?}",
        generated.diagnostics
    );
    font.features = generated.source;
    font.feature_settings = generated.settings;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("generated.glyphs");
    write_glyphs(&font, &path).unwrap();
    let reopened = read_glyphs(&path).unwrap();
    assert_eq!(font.feature_settings, reopened.feature_settings);
    assert_eq!(
        font.features.features[0].id,
        reopened.features.features[0].id
    );
    let regenerated = fc_compile::generate_feature_source(
        &reopened,
        &reopened.features,
        &reopened.feature_settings,
    );
    assert!(
        regenerated.diagnostics.is_empty(),
        "{:?}",
        regenerated.diagnostics
    );
    let text = std::fs::read_to_string(&path).unwrap();
    let mut raw = glyphs_reader::Plist::parse(&text).unwrap();
    if let glyphs_reader::Plist::Dictionary(root) = &mut raw {
        if let Some(glyphs_reader::Plist::Array(features)) = root.get_mut("features") {
            if let glyphs_reader::Plist::Dictionary(feature) = &mut features[0] {
                feature.insert(
                    "code".into(),
                    glyphs_reader::Plist::String("sub A A by B;".into()),
                );
            }
        }
    }
    std::fs::write(&path, raw.to_string()).unwrap();
    let edited = read_glyphs(&path).unwrap();
    assert!(!edited.feature_settings.ligatures);
    assert!(edited.feature_settings.generated.is_empty());
    assert_eq!(edited.features.features[0].code, "sub A A by B;");
    assert!(fc_compile::generate_feature_source(
        &edited,
        &edited.features,
        &edited.feature_settings
    )
    .diagnostics
    .is_empty());
}

#[test]
fn singular_transforms_are_preserved_or_rejected_before_output() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("singular.glyphs");
    for coeffs in [
        [0., 0., 1., 2., 0., 0.],
        [1., 2., 0., 0., 0., 0.],
        [0., 1., 2., 0., 0., 0.],
    ] {
        let mut font = sample();
        font.glyph_mut("B").unwrap().layers[0].components[0].transform = kurbo::Affine::new(coeffs);
        write_glyphs(&font, &path).unwrap();
        let native = glyphs_reader::Font::load(&path).unwrap();
        let glyphs_reader::Shape::Component(component) = &native.glyphs["B"].layers[0].shapes[0]
        else {
            panic!("missing native component")
        };
        assert!(component
            .transform
            .as_coeffs()
            .iter()
            .zip(coeffs)
            .all(|(a, b)| (a - b).abs() < 1e-9));
    }
    let mut font = sample();
    font.glyph_mut("B").unwrap().layers[0].components[0].transform =
        kurbo::Affine::new([1., 0., 1., 0., 0., 0.]);
    std::fs::write(&path, "keep this").unwrap();
    assert!(write_glyphs(&font, &path).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "keep this");
}

#[test]
fn preserves_guide_userdata_and_localized_description_properties() {
    let input = NATIVE_V2.replace("ascender = 800;", "guideLines = ({ position = \"{10, 20}\"; angle = 90; userData = { vendor = kept; }; }); ascender = 800;")
        .replace("familyName = Native;", "familyName = Native; properties = ({ key = descriptions; vendorDescription = kept; values = ({ language = dflt; value = English; }, { language = ZHS; value = Chinese; }); });");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("metadata.glyphs");
    std::fs::write(&path, input).unwrap();
    let font = read_glyphs(&path).unwrap();
    write_glyphs(&font, &path).unwrap();
    let raw = glyphs_reader::Plist::parse(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let guide = &raw.get("fontMaster").unwrap().as_array().unwrap()[0]
        .get("guides")
        .unwrap()
        .as_array()
        .unwrap()[0];
    assert_eq!(
        guide
            .get("userData")
            .unwrap()
            .get("vendor")
            .unwrap()
            .as_str(),
        Some("kept")
    );
    let property = raw
        .get("properties")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p.get("key").and_then(glyphs_reader::Plist::as_str) == Some("descriptions"))
        .unwrap();
    assert_eq!(
        property.get("vendorDescription").unwrap().as_str(),
        Some("kept")
    );
    assert!(property
        .get("values")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v.get("language").and_then(glyphs_reader::Plist::as_str) == Some("ZHS")));
}

#[test]
fn converts_v2_brace_layers_and_rejects_partial_component_alignment() {
    let input = NATIVE_V2.replace("name = Regular;", "name = Regular; weightValue = 100;")
        .replace("fontMaster = ({", "fontMaster = ({ id = M2; name = Bold; weightValue = 900; },{")
        .replace("layers = ({ layerId = M1;", "layers = ({ layerId = M2; width = 700; }, { layerId = BRACE; associatedMasterId = M1; name = \"{500}\"; width = 650; },{ layerId = M1;");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("brace.glyphs");
    std::fs::write(&path, input).unwrap();
    let font = read_glyphs(&path).unwrap();
    let layer = font
        .glyph("A")
        .unwrap()
        .layers
        .iter()
        .find(|l| l.name == "{500}")
        .unwrap();
    assert!(
        matches!(&layer.kind, fc_model::LayerKind::Brace {location, ..} if location.values().copied().collect::<Vec<_>>() == [500.])
    );
    write_glyphs(&font, &path).unwrap();
    let reopened = read_glyphs(&path).unwrap();
    assert_eq!(
        reopened
            .glyph("A")
            .unwrap()
            .layers
            .iter()
            .find(|l| l.name == "{500}")
            .unwrap()
            .kind,
        layer.kind
    );
    let input = NATIVE_V2.replace("name = A; transform", "name = A; alignment = 3; transform");
    std::fs::write(&path, input).unwrap();
    assert!(read_glyphs(&path)
        .unwrap_err()
        .to_string()
        .contains("alignment"));
}

#[test]
fn mapped_axes_use_user_bounds_and_external_mapping_edits_win() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mapped.glyphs");
    let mut font = Font::new_empty("Mapped", 1000);
    font.axes.push(Axis {
        tag: Tag(*b"wght"),
        name: "Weight".into(),
        min: 900.,
        default: 900.,
        max: 1000.,
        map: vec![(900., 0.), (1000., 100.)],
        ..Default::default()
    });
    font.masters[0].location.insert(Tag(*b"wght"), 0.);
    let mut second = Master {
        name: "Bold".into(),
        ..Default::default()
    };
    second.location.insert(Tag(*b"wght"), 100.);
    font.add_master(second);
    write_glyphs(&font, &path).unwrap();
    let reopened = read_glyphs(&path).unwrap();
    assert_eq!(reopened.axes, font.axes);
    assert_eq!(reopened.masters[1].location[&Tag(*b"wght")], 100.);
    let source = std::fs::read_to_string(&path)
        .unwrap()
        .replace("900 = 0;", "1000 = 0;")
        .replace("1000 = 100;", "2000 = 100;")
        .replace("default = 900;", "default = 1000;");
    std::fs::write(&path, source).unwrap();
    let edited = read_glyphs(&path).unwrap();
    assert_eq!(
        (
            edited.axes[0].min,
            edited.axes[0].default,
            edited.axes[0].max
        ),
        (1000., 1000., 2000.)
    );
}

#[test]
fn rejects_unrepresented_native_paint_and_component_placement() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("unsupported.glyphs");
    for input in [
        NATIVE_V2.replace(
            "layerId = M1; width = 600;",
            "layerId = M1; width = 600; attr = { svg = 1; };",
        ),
        NATIVE_V2.replace(
            "layerId = M1; width = 600;",
            "layerId = M1; width = 600; attr = { sbixSize = 16; };",
        ),
        NATIVE_V2.replace("closed = 1;", "closed = 1; attr = { reversePaths = 1; };"),
        NATIVE_V2.replace(
            "name = A; transform",
            "name = A; orientation = 2; transform",
        ),
    ] {
        std::fs::write(&path, input).unwrap();
        assert!(read_glyphs(&path).is_err());
    }
}

#[test]
fn preserves_metric_definition_and_value_unknowns() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("metrics.glyphs");
    write_glyphs(&sample(), &path).unwrap();
    let text = std::fs::read_to_string(&path)
        .unwrap()
        .replace("type = ascender;", "type = ascender; vendorMetric = kept;")
        .replace("pos = 800;", "pos = 800; over = 17; vendorValue = kept;");
    std::fs::write(&path, text).unwrap();
    let font = read_glyphs(&path).unwrap();
    write_glyphs(&font, &path).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("vendorMetric = kept;"));
    assert!(text.contains("vendorValue = kept;"));
    assert!(text.contains("over = 17;"));
}
