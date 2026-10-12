use fc_model::{CidInfo, Font, Glyph, GlyphName, Layer, PsPrivate};
use fc_sfnt::write::write_cff;
use read_fonts::{ps::cff::CffFontRef, types::GlyphId};

fn source_serif_cid() -> (Font, Vec<u8>) {
    let source = std::fs::read(fc_fixtures::path("SourceSerif4Min.otf")).expect("fixture");
    let mut font = fc_sfnt::read_bytes(&source, &fc_sfnt::ReadOptions::default())
        .expect("source")
        .fonts
        .remove(0);
    let order = font.glyph_order().cloned().collect::<Vec<_>>();
    font.cid = Some(CidInfo {
        registry: "Adobe".into(),
        ordering: "Identity".into(),
        supplement: 0,
        cid_map: order
            .iter()
            .enumerate()
            .map(|(index, name)| (index as u32, name.clone()))
            .collect(),
    });
    font.masters[0].private_dict = PsPrivate {
        blue_values: vec![-10.0, 0.0, 700.0, 710.0],
        std_vw: Some(70.0),
        ..PsPrivate::default()
    };
    font.masters[0].cff_private_dicts = vec![PsPrivate {
        blue_values: vec![-20.0, 0.0, 800.0, 820.0],
        std_vw: Some(95.0),
        ..PsPrivate::default()
    }];
    let master = font.default_master().id.clone();
    font.glyph_mut(&order[1])
        .expect("glyph")
        .master_layer_mut(&master)
        .expect("layer")
        .cff_private_dict_index = Some(1);
    (font, source)
}

fn wrap_cff(source: &[u8], cff: Vec<u8>) -> Vec<u8> {
    use read_fonts::{types::Tag, FontRef};
    let source = FontRef::new(source).expect("source face");
    let mut builder = write_fonts::FontBuilder::new();
    for record in source.table_directory().table_records() {
        if record.tag() != Tag::new(b"CFF ") {
            builder.add_raw(
                record.tag(),
                source.table_data(record.tag()).expect("table").as_bytes(),
            );
        }
    }
    builder.add_raw(Tag::new(b"CFF "), cff);
    builder.build()
}

fn cid_font() -> Font {
    let mut font = Font::new_empty("CID Private", 1000);
    let master = font.default_master().id.clone();
    for (name, fd) in [(".notdef", None), ("A", Some(1)), ("B", Some(0))] {
        let mut glyph = Glyph::new(name);
        let mut layer = Layer::new_master(master.clone());
        layer.width = 500.0;
        layer.cff_private_dict_index = fd;
        glyph.layers.push(layer);
        font.insert_glyph(glyph).expect("insert");
    }
    font.cid = Some(CidInfo {
        registry: "Adobe".into(),
        ordering: "Identity".into(),
        supplement: 0,
        cid_map: [(0, ".notdef".into()), (17, "A".into()), (42, "B".into())].into(),
    });
    font.masters[0].private_dict.blue_values = vec![-10.0, 0.0, 700.0, 710.0];
    font.masters[0].cff_private_dicts.push(PsPrivate {
        blue_values: vec![-20.0, 0.0, 800.0, 820.0],
        std_vw: Some(95.0),
        ..PsPrivate::default()
    });
    font
}

#[test]
fn cid_writer_retains_each_private_dict_and_selection_in_export_order() {
    let font = cid_font();
    let order: Vec<GlyphName> = [".notdef", "B", "A"].map(Into::into).into();
    let output = write_cff(&font, &font.default_master().id, &order, false).expect("write");
    let cff = CffFontRef::new_cff(&output.bytes, 0, Some(1000)).expect("read CFF");
    assert_eq!(cff.num_subfonts(), 2);
    assert_eq!(cff.subfont_index(GlyphId::new(0)), Some(0));
    assert_eq!(cff.subfont_index(GlyphId::new(1)), Some(0));
    assert_eq!(cff.subfont_index(GlyphId::new(2)), Some(1));
    for (fd, expected) in [
        (0, vec![(-10.0, 0.0), (700.0, 710.0)]),
        (1, vec![(-20.0, 0.0), (800.0, 820.0)]),
    ] {
        let (_, hints) = cff.subfont_hinted(fd, &[]).expect("private");
        let blues: Vec<_> = hints
            .blues
            .values()
            .iter()
            .map(|(a, b)| (a.to_f64(), b.to_f64()))
            .collect();
        assert_eq!(blues, expected);
    }
}

#[test]
fn cid_writer_rejects_unresolved_dictionary_selection_and_missing_cid_metadata() {
    let mut font = cid_font();
    let master = font.default_master().id.clone();
    let order = font.glyph_order().cloned().collect::<Vec<_>>();
    font.glyph_mut("A")
        .expect("A")
        .master_layer_mut(&master)
        .expect("layer")
        .cff_private_dict_index = Some(2);
    let error = write_cff(&font, &master, &order, false)
        .err()
        .expect("invalid selection");
    assert!(error.to_string().contains("private dictionary"), "{error}");
    font.glyph_mut("A")
        .expect("A")
        .master_layer_mut(&master)
        .expect("layer")
        .cff_private_dict_index = Some(1);
    font.cid = None;
    let error = write_cff(&font, &master, &order, false)
        .err()
        .expect("CID required");
    assert!(error.to_string().contains("CID"), "{error}");
}

#[test]
fn cid_reader_and_writer_round_trip_all_private_values_and_assignments() {
    let (font, source) = source_serif_cid();
    let order = font.glyph_order().cloned().collect::<Vec<_>>();
    let output = write_cff(&font, &font.default_master().id, &order, false).expect("write");
    let sfnt = wrap_cff(&source, output.bytes);
    let outcome = fc_sfnt::read_bytes(&sfnt, &fc_sfnt::ReadOptions::default()).expect("read CID");
    assert!(!outcome
        .warnings
        .iter()
        .any(|warning| warning.code == "cff-private-dicts-flattened"));
    let imported = &outcome.fonts[0];
    assert_eq!(
        imported.default_master().private_dict,
        font.default_master().private_dict
    );
    assert_eq!(
        imported.default_master().cff_private_dicts,
        font.default_master().cff_private_dicts
    );
    for (index, name) in imported.glyph_order().enumerate() {
        let layer = imported
            .glyph(name)
            .expect("glyph")
            .master_layer(&imported.default_master().id)
            .expect("layer");
        assert_eq!(layer.cff_private_dict_index, Some(u8::from(index == 1)));
    }
    let imported_order = imported.glyph_order().cloned().collect::<Vec<_>>();
    let second = write_cff(
        imported,
        &imported.default_master().id,
        &imported_order,
        false,
    )
    .expect("rewrite");
    let reader = CffFontRef::new_cff(&second.bytes, 0, Some(1000)).expect("second read");
    assert_eq!(reader.num_subfonts(), 2);
    assert_eq!(reader.subfont_index(GlyphId::new(1)), Some(1));
    // Optional acceptance output is confined to the explicitly requested task path.
    if let Some(path) = std::env::var_os("FC_CFF_PRIVATE_ACCEPTANCE") {
        std::fs::write(path, sfnt).expect("acceptance font");
    }
}

#[test]
fn cid_writer_rejects_too_many_private_dicts_and_invalid_extra_values() {
    let mut font = cid_font();
    let order = font.glyph_order().cloned().collect::<Vec<_>>();
    font.masters[0].cff_private_dicts[0].std_vw = Some(-1.0);
    assert!(write_cff(&font, &font.default_master().id, &order, false).is_err());
    font.masters[0].cff_private_dicts = vec![PsPrivate::default(); 256];
    let error = write_cff(&font, &font.default_master().id, &order, false)
        .err()
        .expect("too many FDs");
    assert!(error.to_string().contains("256"), "{error}");
}
