use fc_compile::{compile_static_with_metadata, ShapingCompile};
use fc_model::{Anchor, Contour, FeatureBlock, Font, Glyph, KernSide, Layer};
use harfrust::{ShapeOptions, ShaperData, UnicodeBuffer};
use kurbo::BezPath;
use read_fonts::{types::Tag, FontRef};

fn fixture() -> Font {
    let mut font = Font::new_empty("Incremental shaping", 1000);
    let master = font.default_master().id.clone();
    for (name, unicode, width) in [
        (".notdef", None, 500.),
        ("a", Some(0x61), 600.),
        ("b", Some(0x62), 600.),
        ("acutecomb", Some(0x301), 0.),
        ("a_b", None, 1000.),
    ] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes.extend(unicode);
        let mut layer = Layer::new_master(master.clone());
        layer.width = width;
        let mut path = BezPath::new();
        path.move_to((10., 0.));
        path.line_to((210., 0.));
        path.line_to((210., 400.));
        path.line_to((10., 400.));
        path.close_path();
        layer.contours = Contour::from_bezpath(&path);
        glyph.layers.push(layer);
        font.insert_glyph(glyph).unwrap();
    }
    font.features.features.push(FeatureBlock {
        tag: "liga".into(),
        code: "sub a b by a_b;".into(),
        ..Default::default()
    });
    font
}

fn shape(bytes: &[u8], text: &str) -> Vec<(u32, i32, i32, i32, i32)> {
    let face = FontRef::new(bytes).unwrap();
    let data = ShaperData::new(&face);
    let shaper = data.shaper(&face).build();
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.guess_segment_properties();
    let run = shaper.shape(buffer, ShapeOptions::default());
    run.glyph_infos()
        .iter()
        .zip(run.glyph_positions())
        .map(|(i, p)| (i.glyph_id, p.x_advance, p.y_advance, p.x_offset, p.y_offset))
        .collect()
}

fn assert_full_equivalent(font: &Font, incremental: &ShapingCompile) {
    let full =
        compile_static_with_metadata(font, &font.default_master().id, &Default::default()).unwrap();
    assert_eq!(&*incremental.glyph_order, full.glyph_order);
    for text in ["ab", "ba", "a\u{301}", "aba\u{301}"] {
        assert_eq!(
            shape(&incremental.bytes, text),
            shape(&full.bytes, text),
            "{text}"
        );
    }
}

#[test]
fn layout_edits_reuse_outline_tables_and_match_full_compilation() {
    let mut font = fixture();
    let master = font.default_master().id.clone();
    let initial = ShapingCompile::build(&font, &master, None).unwrap();
    assert!(!initial.incremental);
    font.features.features[0].code = "sub b a by a_b;".into();
    font.masters[0].kerning_h.0.insert(
        (KernSide::Glyph("a".into()), KernSide::Glyph("b".into())),
        -80.,
    );
    let changed = ShapingCompile::build(&font, &master, Some(&initial)).unwrap();
    assert!(changed.incremental);
    for tag in [b"glyf", b"loca", b"cmap", b"hmtx"] {
        let before = FontRef::new(&initial.bytes).unwrap();
        let after = FontRef::new(&changed.bytes).unwrap();
        assert_eq!(
            before.table_data(Tag::new(tag)).unwrap().as_bytes(),
            after.table_data(Tag::new(tag)).unwrap().as_bytes()
        );
    }
    assert_full_equivalent(&font, &changed);

    font.features.features.clear();
    font.masters[0].kerning_h.0.clear();
    let removed = ShapingCompile::build(&font, &master, Some(&changed)).unwrap();
    assert!(removed.incremental);
    assert_eq!(shape(&removed.bytes, "ab").len(), 2);
    assert_full_equivalent(&font, &removed);
}

#[test]
fn anchor_edits_use_the_same_automatic_mark_writer() {
    let mut font = fixture();
    let master = font.default_master().id.clone();
    let initial = ShapingCompile::build(&font, &master, None).unwrap();
    for (name, anchor_name, x, y) in [("a", "top", 100., 800.), ("acutecomb", "_top", 0., 0.)] {
        font.glyph_mut(name)
            .unwrap()
            .master_layer_mut(&master)
            .unwrap()
            .anchors
            .push(Anchor {
                name: anchor_name.into(),
                x,
                y,
            });
    }
    let changed = ShapingCompile::build(&font, &master, Some(&initial)).unwrap();
    assert!(changed.incremental);
    assert_ne!(
        shape(&initial.bytes, "a\u{301}"),
        shape(&changed.bytes, "a\u{301}")
    );
    assert_full_equivalent(&font, &changed);
}

#[test]
fn non_layout_changes_invalidate_the_reusable_font() {
    let font = fixture();
    let master = font.default_master().id.clone();
    let initial = ShapingCompile::build(&font, &master, None).unwrap();
    let changes: &[fn(&mut Font)] = &[
        |f| f.glyph_mut("a").unwrap().unicodes = vec![0x63],
        |f| f.glyph_mut("a_b").unwrap().layers[0].width = 800.,
        |f| f.glyph_mut("a").unwrap().layers[0].contours[0].points[2].y = 900.,
        |f| f.glyphs.swap_indices(1, 2),
        |f| f.info.units_per_em = 2048,
        |f| f.glyph_mut("a").unwrap().layers[0].vertical_width = Some(1200.),
    ];
    for change in changes {
        let mut changed = font.clone();
        change(&mut changed);
        let rebuilt = ShapingCompile::build(&changed, &master, Some(&initial)).unwrap();
        assert!(!rebuilt.incremental);
        assert_full_equivalent(&changed, &rebuilt);
    }
}

#[test]
fn metadata_feature_tables_are_never_left_behind_on_removal() {
    let mut font = fixture();
    let master = font.default_master().id.clone();
    font.features.prefixes.push(FeatureBlock {
        code: "table hhea { Ascender 1234; } hhea;".into(),
        ..Default::default()
    });
    let initial = ShapingCompile::build(&font, &master, None).unwrap();
    font.features.prefixes.clear();
    let changed = ShapingCompile::build(&font, &master, Some(&initial)).unwrap();
    assert!(!changed.incremental);
    assert_full_equivalent(&font, &changed);
    use read_fonts::TableProvider;
    assert_ne!(
        FontRef::new(&initial.bytes)
            .unwrap()
            .hhea()
            .unwrap()
            .ascender(),
        FontRef::new(&changed.bytes)
            .unwrap()
            .hhea()
            .unwrap()
            .ascender()
    );
}

#[test]
fn repeated_feature_refresh_has_a_bounded_runtime() {
    let mut font = fixture();
    let master = font.default_master().id.clone();
    let mut compiled = ShapingCompile::build(&font, &master, None).unwrap();
    let start = std::time::Instant::now();
    for i in 0..20 {
        font.features.features[0].code = if i % 2 == 0 {
            "sub b a by a_b;"
        } else {
            "sub a b by a_b;"
        }
        .into();
        compiled = ShapingCompile::build(&font, &master, Some(&compiled)).unwrap();
        assert!(compiled.incremental);
    }
    assert!(
        start.elapsed() < std::time::Duration::from_secs(10),
        "20 tiny layout refreshes took {:?}",
        start.elapsed()
    );
}
