use fc_model::{Contour, Font, Glyph, Layer};
use fc_sfnt::cff_interchange::{read_bytes, read_outcome, write_bytes};
use kurbo::BezPath;
use read_fonts::{types::Tag, FontRef};

fn font(upem: u16) -> Font {
    let mut font = Font::new_empty("Bare CFF", upem);
    font.info.postscript_font_name = Some("BareCFF-Regular".into());
    let master = font.default_master().id.clone();
    font.masters[0].private_dict.blue_values = vec![-20.0, 0.0, 700.0, 720.0];
    for name in [".notdef", "A", "uni20AC", "u1F600"] {
        let mut glyph = Glyph::new(name);
        let mut layer = Layer::new_master(master.clone());
        layer.width = 640.0;
        if name == "A" {
            let mut path = BezPath::new();
            path.move_to((10.125, 0.0));
            path.curve_to((30.25, 720.5), (400.75, 720.5), (610.0, 0.0));
            path.close_path();
            layer.contours = Contour::from_bezpath(&path);
        }
        glyph.layers.push(layer);
        font.insert_glyph(glyph).unwrap();
    }
    font
}

#[test]
fn bare_cff_preserves_outlines_widths_names_hints_and_nonstandard_upem() {
    let started = std::time::Instant::now();
    for upem in [1000, 2048] {
        let source = font(upem);
        let bytes = write_bytes(&source, &source.default_master().id).unwrap();
        assert_eq!(bytes[0], 1);
        let outcome = read_outcome(&bytes).unwrap();
        let imported = &outcome.fonts[0];
        assert_eq!(imported.info.units_per_em, upem);
        assert_eq!(
            imported.info.postscript_font_name,
            source.info.postscript_font_name
        );
        assert_eq!(
            imported.default_master().private_dict.blue_values,
            source.default_master().private_dict.blue_values
        );
        assert_eq!(imported.glyph("A").unwrap().unicodes, [0x41]);
        assert_eq!(imported.glyph("uni20AC").unwrap().unicodes, [0x20ac]);
        assert_eq!(imported.glyph("u1F600").unwrap().unicodes, [0x1f600]);
        for name in source.glyph_order() {
            let original = &source.glyph(name).unwrap().layers[0];
            let actual = &imported.glyph(name).unwrap().layers[0];
            assert_eq!(original.width, actual.width);
            assert_eq!(
                original
                    .contours
                    .iter()
                    .map(Contour::to_bezpath)
                    .collect::<Vec<_>>(),
                actual
                    .contours
                    .iter()
                    .map(Contour::to_bezpath)
                    .collect::<Vec<_>>()
            );
        }
        assert!(imported.extra.raw_tables.is_empty());
        assert!(imported
            .extra
            .source
            .as_ref()
            .unwrap()
            .original_bytes
            .is_none());
        assert!(outcome
            .warnings
            .iter()
            .any(|w| w.code == "bare-cff-metadata"));
    }
    assert!(started.elapsed() < std::time::Duration::from_secs(10));
}

#[test]
fn reads_existing_cff_table_with_native_advance_widths() {
    let bytes = std::fs::read(fc_fixtures::path("SourceSans3Min.otf")).unwrap();
    let face = FontRef::new(&bytes).unwrap();
    let cff = face.table_data(Tag::new(b"CFF ")).unwrap();
    let imported = read_bytes(cff.as_bytes()).unwrap();
    let otf = fc_sfnt::read_bytes(&bytes, &Default::default()).unwrap();
    let original = &otf.fonts[0];
    assert_eq!(imported.glyphs.len(), original.glyphs.len());
    for name in original.glyph_order() {
        let expected = &original.glyph(name).unwrap().layers[0];
        let actual = &imported.glyph(name).unwrap().layers[0];
        assert_eq!(actual.width, expected.width);
        assert_eq!(
            actual
                .contours
                .iter()
                .map(Contour::to_bezpath)
                .collect::<Vec<_>>(),
            expected
                .contours
                .iter()
                .map(Contour::to_bezpath)
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn rejects_truncated_input_cff2_and_invalid_width() {
    let mut source = font(1000);
    let bytes = write_bytes(&source, &source.default_master().id).unwrap();
    for end in 0..bytes.len() {
        assert!(read_bytes(&bytes[..end]).is_err(), "accepted prefix {end}");
    }
    assert!(read_bytes(&[2, 0, 5, 0, 0, 0, 0, 0, 0]).is_err());
    source.glyph_mut("A").unwrap().layers[0].width = f64::NAN;
    assert!(write_bytes(&source, &source.default_master().id).is_err());
}

#[test]
fn cid_cff_preserves_identity_without_inventing_unicode() {
    let mut source = font(1000);
    source.cid = Some(fc_model::CidInfo {
        registry: "Adobe".into(),
        ordering: "Identity".into(),
        supplement: 0,
        cid_map: source
            .glyph_order()
            .enumerate()
            .map(|(gid, name)| (gid as u32, name.clone()))
            .collect(),
    });
    let bytes = write_bytes(&source, &source.default_master().id).unwrap();
    let imported = read_bytes(&bytes).unwrap();
    let cid = imported.cid.as_ref().unwrap();
    assert_eq!(cid.registry, "Adobe");
    assert_eq!(cid.cid_map.len(), source.glyphs.len());
    assert!(imported
        .glyphs
        .values()
        .all(|glyph| glyph.unicodes.is_empty()));
    assert!(!imported.glyph("cid00001").unwrap().layers[0]
        .contours
        .is_empty());
}
