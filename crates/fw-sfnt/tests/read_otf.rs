use fw_model::PointKind;
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

#[test]
fn reads_external_cid_cff_metadata_when_available() {
    let path = fw_fixtures::skip_unless_external!("source-han-sans-sc-otf");
    let outcome = read_path(&path, &ReadOptions::default()).expect("read Source Han Sans");
    let cid = outcome.fonts[0].cid.as_ref().expect("CID metadata");
    assert_eq!(cid.registry, "Adobe");
    assert_eq!(cid.ordering, "Identity");
    assert!(!cid.cid_map.is_empty());
}
