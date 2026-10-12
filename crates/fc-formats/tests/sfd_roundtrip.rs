use fc_formats::sfd::{read_sfd, write_sfd};
use fc_model::{Anchor, Component, Contour, FeatureBlock, Font, Glyph, KernSide, Layer};
use kurbo::{Affine, BezPath};

fn sample() -> Font {
    let mut font = Font::new_empty("SFD Example", 1000);
    font.masters[0].metrics.ascender = 800.0;
    font.masters[0].metrics.descender = -200.0;
    let master = font.default_master().id.clone();
    for (name, unicode) in [
        (".notdef", None),
        ("A", Some(65)),
        ("V", Some(86)),
        ("f", Some(102)),
        ("i", Some(105)),
        ("fi", None),
    ] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes = unicode.into_iter().collect();
        let mut layer = Layer::new_master(master.clone());
        layer.width = 600.0;
        if name == "A" {
            let mut path = BezPath::new();
            path.move_to((10.0, 0.0));
            path.curve_to((100.0, 400.0), (200.0, 700.0), (300.0, 700.0));
            path.line_to((590.0, 0.0));
            path.close_path();
            layer.contours = Contour::from_bezpath(&path);
            layer.anchors.push(Anchor {
                name: "top".into(),
                x: 300.0,
                y: 720.0,
            });
        }
        if name == "V" {
            layer.components.push(Component {
                base: "A".into(),
                transform: Affine::new([1.0, 0.0, 0.0, -1.0, 0.0, 700.0]),
                ..Default::default()
            });
        }
        glyph.layers.push(layer);
        font.insert_glyph(glyph).unwrap();
    }
    font.masters[0].kerning_h.0.insert(
        (KernSide::Glyph("A".into()), KernSide::Glyph("V".into())),
        -80.0,
    );
    font.features.features.push(FeatureBlock {
        tag: "liga".into(),
        code: "sub f i by fi;".into(),
        ..Default::default()
    });
    font
}

#[test]
fn native_roundtrip_and_external_edits() {
    let font = sample();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sample.sfd");
    write_sfd(&font, &font.default_master().id, &path).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.starts_with("SplineFontDB: 3.0\n"));
    assert!(text.contains("Ligature2:"));
    assert!(text.contains("Kerns2:"));
    assert!(text.contains("Refer:"));
    assert!(text.contains("AnchorPoint:"));
    assert!(!text.contains("FontCarverFont"));
    std::fs::write(&path, text.replace("Width: 600", "Width: 620")).unwrap();
    let loaded = read_sfd(&path).unwrap();
    assert_eq!(loaded.info.family_name, font.info.family_name);
    assert_eq!(loaded.info.units_per_em, 1000);
    assert_eq!(
        loaded
            .glyph("A")
            .unwrap()
            .master_layer(&loaded.default_master().id)
            .unwrap()
            .width,
        620.0
    );
    assert_eq!(loaded.masters[0].kerning_h, font.masters[0].kerning_h);
    assert!(loaded.features.to_fea().contains("sub \\f \\i by \\fi;"));
    let a = loaded
        .glyph("A")
        .unwrap()
        .master_layer(&loaded.default_master().id)
        .unwrap();
    assert_eq!(
        a.contours[0].to_bezpath(),
        font.glyph("A")
            .unwrap()
            .master_layer(&font.default_master().id)
            .unwrap()
            .contours[0]
            .to_bezpath()
    );
    assert_eq!(a.anchors[0].name, "top");
}

#[test]
fn unsupported_layout_failure_is_atomic() {
    let mut font = sample();
    font.features.features[0].code = "sub A' V by fi;".into();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("unchanged.sfd");
    std::fs::write(&path, "keep me").unwrap();
    assert!(write_sfd(&font, &font.default_master().id, &path).is_err());
    assert_eq!(std::fs::read_to_string(path).unwrap(), "keep me");
}

#[test]
fn external_native_sfd_corpus() {
    let started = std::time::Instant::now();
    let Some(corpus) = fc_fixtures::external("fontforge-tests") else {
        return;
    };
    for name in [
        "DirectionTest.sfd",
        "SplineOverlapBug1.sfd",
        "QuadOverlapBugs.sfd",
    ] {
        let font = read_sfd(&corpus.join(name)).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(!font.glyphs.is_empty());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(name);
        write_sfd(&font, &font.default_master().id, &path)
            .unwrap_or_else(|error| panic!("{name} write: {error}"));
        let loaded = read_sfd(&path).unwrap();
        assert_eq!(loaded.glyphs.len(), font.glyphs.len());
        for (name, glyph) in &font.glyphs {
            let before = glyph.master_layer(&font.default_master().id).unwrap();
            let after = loaded
                .glyph(name.as_ref())
                .unwrap()
                .master_layer(&loaded.default_master().id)
                .unwrap();
            assert_eq!(before.width, after.width);
            assert_eq!(
                before
                    .contours
                    .iter()
                    .map(Contour::to_bezpath)
                    .collect::<Vec<_>>(),
                after
                    .contours
                    .iter()
                    .map(Contour::to_bezpath)
                    .collect::<Vec<_>>(),
                "{name}"
            );
        }
    }
    assert!(
        started.elapsed() < std::time::Duration::from_secs(10),
        "small SFD corpus exceeded import/export budget"
    );
}

#[test]
fn native_layout_roundtrip_compiles_with_fontc() {
    let font = sample();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("layout.sfd");
    write_sfd(&font, &font.default_master().id, &path).unwrap();
    let loaded = read_sfd(&path).unwrap();
    let before = fc_compile::export(&font, &|_| 0, &Default::default()).unwrap();
    let after = fc_compile::export(&loaded, &|_| 0, &Default::default()).unwrap();
    use read_fonts::{FontRef, TableProvider};
    let before = FontRef::new(&before.bytes).unwrap();
    let after = FontRef::new(&after.bytes).unwrap();
    assert_eq!(
        before.gsub().unwrap().offset_data().as_bytes(),
        after.gsub().unwrap().offset_data().as_bytes()
    );
    assert_eq!(
        before.gpos().unwrap().offset_data().as_bytes(),
        after.gpos().unwrap().offset_data().as_bytes()
    );
}

fn shape(font: &Font, text: &str, script: &str) -> Vec<(u32, i32, i32, i32, i32)> {
    use harfrust::{Script, ShapeOptions, ShaperData, UnicodeBuffer};
    use std::str::FromStr;
    let bytes = fc_compile::export(font, &|_| 0, &Default::default())
        .unwrap()
        .bytes;
    let face = read_fonts::FontRef::new(&bytes).unwrap();
    let data = ShaperData::new(&face);
    let shaper = data.shaper(&face).build();
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.set_script(Script::from_str(script).unwrap());
    buffer.guess_segment_properties();
    let shaped = shaper.shape(buffer, ShapeOptions::default());
    shaped
        .glyph_infos()
        .iter()
        .zip(shaped.glyph_positions())
        .map(|(info, pos)| {
            (
                info.glyph_id,
                pos.x_advance,
                pos.y_advance,
                pos.x_offset,
                pos.y_offset,
            )
        })
        .collect()
}

#[test]
fn class_kerning_and_group_exceptions_keep_shaping() {
    let mut font = sample();
    font.features.features.clear();
    font.kerning_groups.side1.insert(
        "public.kern1.left".into(),
        ["A".into(), "f".into()].into_iter().collect(),
    );
    font.kerning_groups.side2.insert(
        "public.kern2.right".into(),
        ["V".into(), "i".into()].into_iter().collect(),
    );
    let pairs = &mut font.masters[0].kerning_h.0;
    pairs.insert(
        (
            KernSide::Group("public.kern1.left".into()),
            KernSide::Group("public.kern2.right".into()),
        ),
        -100.0,
    );
    pairs.insert(
        (
            KernSide::Glyph("A".into()),
            KernSide::Group("public.kern2.right".into()),
        ),
        -60.0,
    );
    pairs.insert(
        (
            KernSide::Group("public.kern1.left".into()),
            KernSide::Glyph("V".into()),
        ),
        -40.0,
    );
    for name in ["A", "f"] {
        font.glyph_mut(name).unwrap().kern_group_side1 = Some("public.kern1.left".into());
    }
    for name in ["V", "i"] {
        font.glyph_mut(name).unwrap().kern_group_side2 = Some("public.kern2.right".into());
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("groups.sfd");
    write_sfd(&font, &font.default_master().id, &path).unwrap();
    let loaded = read_sfd(&path).unwrap();
    assert_eq!(loaded.kerning_groups.side1.len(), 1);
    assert_eq!(loaded.kerning_groups.side2.len(), 1);
    assert!(loaded
        .glyph("A")
        .unwrap()
        .kern_group_side1
        .as_deref()
        .unwrap()
        .starts_with("public.kern1."));
    assert!(loaded
        .glyph("V")
        .unwrap()
        .kern_group_side2
        .as_deref()
        .unwrap()
        .starts_with("public.kern2."));
    for text in ["AV", "Ai", "fV", "fi"] {
        assert_eq!(
            shape(&font, text, "latn"),
            shape(&loaded, text, "latn"),
            "{text}"
        );
    }
    assert_eq!(shape(&loaded, "AV", "latn")[0].1, 520);
    assert_eq!(shape(&loaded, "Ai", "latn")[0].1, 540);
    assert_eq!(shape(&loaded, "fV", "latn")[0].1, 560);
    assert_eq!(shape(&loaded, "fi", "latn")[0].1, 500);
}

#[test]
fn language_system_scopes_survive_native_layout() {
    let mut font = sample();
    font.masters[0].kerning_h.0.clear();
    font.features.prefixes.push(FeatureBlock {
        tag: "languages".into(),
        code: "languagesystem latn dflt;".into(),
        ..Default::default()
    });
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("scripts.sfd");
    write_sfd(&font, &font.default_master().id, &path).unwrap();
    let loaded = read_sfd(&path).unwrap();
    for script in ["latn", "cyrl", "arab"] {
        assert_eq!(
            shape(&font, "fi", script),
            shape(&loaded, "fi", script),
            "{script}"
        );
    }
}

#[test]
fn selected_master_is_a_static_projection() {
    let mut font = sample();
    let mut bold = font.default_master().clone();
    bold.id = fc_model::MasterId::new();
    bold.name = "Bold".into();
    bold.metrics.ascender = 850.0;
    font.add_master(bold.clone());
    font.axes.push(fc_model::Axis {
        tag: "wght".parse().unwrap(),
        name: "Weight".into(),
        min: 400.0,
        default: 400.0,
        max: 700.0,
        ..Default::default()
    });
    font.glyph_mut("A")
        .unwrap()
        .master_layer_mut(&bold.id)
        .unwrap()
        .width = 720.0;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bold.sfd");
    write_sfd(&font, &bold.id, &path).unwrap();
    let loaded = read_sfd(&path).unwrap();
    assert_eq!(loaded.masters.len(), 1);
    assert_eq!(loaded.info.style_name, "Bold");
    assert!(loaded.axes.is_empty());
    assert_eq!(loaded.default_master().metrics.ascender, 850.0);
    assert_eq!(
        loaded
            .glyph("A")
            .unwrap()
            .master_layer(&loaded.default_master().id)
            .unwrap()
            .width,
        720.0
    );
}

#[test]
fn explicit_kern_source_keeps_inactive_editable_pairs_without_double_application() {
    let font = fc_formats::ufo::read_ufo(&fc_fixtures::ufo_path("Minimal.ufo")).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("minimal.sfd");
    write_sfd(&font, &font.default_master().id, &path).unwrap();
    let loaded = read_sfd(&path).unwrap();
    assert!(!loaded.masters[0].kerning_h.0.is_empty());
    assert!(!loaded.feature_settings.writer_policy.kerning);
    assert_eq!(shape(&font, "AB", "latn"), shape(&loaded, "AB", "latn"));
    write_sfd(&loaded, &loaded.default_master().id, &path).unwrap();
    let twice = read_sfd(&path).unwrap();
    assert_eq!(shape(&loaded, "AB", "latn"), shape(&twice, "AB", "latn"));
}

#[test]
fn feature_keyword_glyph_names_are_escaped_when_importing_native_rules() {
    let mut font = sample();
    let mut glyph = Glyph::new("sub");
    glyph.unicodes = vec![0x73];
    font.insert_glyph(glyph.clone()).unwrap();
    font.features.features = vec![FeatureBlock {
        tag: "liga".into(),
        code: "sub \\sub by A;".into(),
        ..Default::default()
    }];
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("keyword.sfd");
    write_sfd(&font, &font.default_master().id, &path).unwrap();
    let loaded = read_sfd(&path).unwrap();
    assert_eq!(shape(&font, "s", "latn"), shape(&loaded, "s", "latn"));
}
