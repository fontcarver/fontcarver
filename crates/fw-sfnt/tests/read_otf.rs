use std::path::Path;

use fw_model::{Font, PointKind};
use fw_sfnt::{read_path, ReadOptions};

#[test]
fn reads_cff_names_and_cubic_contours() {
    let path = fw_fixtures::path("test1012.otf");
    let outcome = read_path(&path, &ReadOptions::default()).expect("read CFF OTF");
    let font = &outcome.fonts[0];

    assert!(font.glyphs.contains_key(".notdef"));
    assert!(font.glyphs.contains_key("uni0627"));
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
fn reads_committed_curve_bearing_cff() {
    let outcome = read_path(
        &fw_fixtures::path("NimbusRoman-Regular-o.otf"),
        &ReadOptions::default(),
    )
    .expect("read subset Nimbus Roman");
    assert_round_o(&outcome.fonts[0]);
    assert!(!outcome
        .warnings
        .iter()
        .any(|warning| warning.code == "cff-hints-not-imported"));
}

#[test]
fn reads_system_curve_bearing_cff_when_available() {
    let path = Path::new("/usr/share/fonts/opentype/urw-base35/NimbusRoman-Regular.otf");
    if !path.is_file() {
        eprintln!("skipping test: system NimbusRoman-Regular.otf is unavailable");
        return;
    }
    let outcome = read_path(path, &ReadOptions::default()).expect("read system Nimbus Roman");
    assert_round_o(&outcome.fonts[0]);
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
        "CantarellMin.woff2",
        "DejaVuSans.ttf",
        "DejaVuSans.woff",
        "NimbusRoman-Regular-o.otf",
        "NotoSans-Regular.ttf",
        "NotoSerifTibetan-Regular.ttf",
        "test1012.otf",
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
}
