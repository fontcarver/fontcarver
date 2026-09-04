use fw_model::{glyph_for_unicode, Font, PointKind};
use fw_sfnt::{read_path, ReadOptions};

#[test]
fn reads_cff_names_and_cubic_contours() {
    let path = fw_fixtures::path("SourceSans3Min.otf");
    let outcome = read_path(&path, &ReadOptions::default()).expect("read CFF OTF");
    let font = &outcome.fonts[0];

    assert_eq!(font.info.family_name, "FontWorkshop CFF Sans Fixture");
    assert_eq!(font.info.style_name, "Regular");
    assert_eq!(font.info.units_per_em, 1_000);
    assert_eq!(font.glyphs.len(), 5);
    assert!(font.glyphs.contains_key(".notdef"));
    assert_eq!(
        glyph_for_unicode(font, 0x41)
            .expect("U+0041 glyph")
            .name
            .as_ref(),
        "A"
    );
    assert_eq!(font.info.postscript_default_width_x, None);
    assert_eq!(font.info.postscript_nominal_width_x, Some(518.0));
    let private = &font.default_master().private_dict;
    assert_eq!(private.std_hw, Some(73.0));
    assert_eq!(private.std_vw, Some(84.0));
    assert_eq!(private.stem_snap_h, [73.0]);
    assert_eq!(private.stem_snap_v, [84.0]);
    assert_eq!(
        private.blue_values,
        [
            -12.0, 0.0, 486.0, 498.0, 518.0, 530.0, 574.0, 586.0, 638.0, 650.0, 656.0, 668.0,
            712.0, 724.0
        ]
    );
    let points: Vec<_> = font
        .glyphs
        .values()
        .flat_map(|glyph| &glyph.layers)
        .flat_map(|layer| &layer.contours)
        .flat_map(|contour| &contour.points)
        .collect();
    assert!(!points.is_empty());
    assert!(points.iter().all(|point| point.kind != PointKind::QCurve));
    assert!(outcome
        .warnings
        .iter()
        .any(|warning| warning.code == "cff-hints-not-imported"));
}

fn assert_round_o(font: &Font) {
    let glyph = font.glyph("o").expect("o glyph");
    let layer = glyph
        .master_layer(&font.default_master().id)
        .expect("o master layer");
    assert!(!layer.contours.is_empty());
    for contour in &layer.contours {
        assert_eq!(
            contour.points.first().expect("nonempty contour").kind,
            PointKind::Curve
        );
        assert!(contour.points.windows(3).any(|points| {
            points[0].kind == PointKind::OffCurve
                && points[1].kind == PointKind::OffCurve
                && points[2].kind == PointKind::Curve
        }));
        let first = contour.points.first().expect("nonempty contour");
        let last = contour.points.last().expect("nonempty contour");
        assert_ne!((first.x, first.y), (last.x, last.y));
    }
}

#[test]
fn reads_unhinted_curve_bearing_cff() {
    let outcome = read_path(
        &fw_fixtures::path("SourceSerif4Min.otf"),
        &ReadOptions::default(),
    )
    .expect("read Source Serif 4 subset");
    let font = &outcome.fonts[0];
    assert_eq!(font.info.family_name, "FontWorkshop CFF Serif Fixture");
    assert_eq!(font.glyphs.len(), 2);
    assert_eq!(font.info.postscript_default_width_x, None);
    assert_eq!(font.info.postscript_nominal_width_x, Some(603.0));
    assert_round_o(font);
    assert!(!outcome
        .warnings
        .iter()
        .any(|warning| warning.code == "cff-hints-not-imported"));
}

#[test]
fn reads_hinted_curve_bearing_cff() {
    let outcome = read_path(
        &fw_fixtures::path("SourceSans3Min.otf"),
        &ReadOptions::default(),
    )
    .expect("read Source Sans 3 subset");
    assert_round_o(&outcome.fonts[0]);
    assert!(outcome
        .warnings
        .iter()
        .any(|warning| warning.code == "cff-hints-not-imported"));
}

fn assert_contour_invariants(font: &Font, source: &str) {
    let is_cff = font
        .extra
        .source
        .as_ref()
        .is_some_and(|provenance| provenance.format == "otf");
    for glyph in font.glyphs.values() {
        for layer in &glyph.layers {
            for (index, contour) in layer.contours.iter().enumerate() {
                assert!(
                    contour.try_to_bezpath().is_ok(),
                    "{source}: glyph {} contour {index} is malformed",
                    glyph.name
                );
                if is_cff {
                    if let (Some(first), Some(last)) =
                        (contour.points.first(), contour.points.last())
                    {
                        assert!(
                            last.kind == PointKind::OffCurve
                                || (first.x, first.y) != (last.x, last.y),
                            "{source}: glyph {} contour {index} repeats its start point",
                            glyph.name
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn every_committed_fixture_has_valid_imported_contours() {
    for name in [
        "DejaVuSans.ttf",
        "DejaVuSans.woff",
        "DejaVuSansMin.woff2",
        "NotoSans-Regular.ttf",
        "NotoSerifTibetan-Regular.ttf",
        "SourceSans3Min.otf",
        "SourceSerif4Min.otf",
    ] {
        let outcome = read_path(&fw_fixtures::path(name), &ReadOptions::default())
            .unwrap_or_else(|error| panic!("read {name}: {error}"));
        for font in &outcome.fonts {
            assert_contour_invariants(font, name);
        }
    }
}

#[test]
fn source_han_has_valid_imported_contours_when_available() {
    let path = fw_fixtures::skip_unless_external!("source-han-sans-sc-otf");
    let outcome = read_path(&path, &ReadOptions::default()).expect("read Source Han Sans");
    for font in &outcome.fonts {
        assert_contour_invariants(font, "Source Han Sans SC");
    }
}

#[test]
fn reads_external_cid_cff_metadata_when_available() {
    let path = fw_fixtures::skip_unless_external!("source-han-sans-sc-otf");
    let outcome = read_path(&path, &ReadOptions::default()).expect("read Source Han Sans");
    let cid = outcome.fonts[0].cid.as_ref().expect("CID metadata");
    assert_eq!(cid.registry, "Adobe");
    assert_eq!(cid.ordering, "Identity");
    assert!(!cid.cid_map.is_empty());
    assert_eq!(
        glyph_for_unicode(&outcome.fonts[0], 0x4E00)
            .expect("U+4E00 glyph")
            .name
            .as_ref(),
        "uni4E00"
    );
}
