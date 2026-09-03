use std::fs;

use fw_model::{NamePlatform, PointKind, Tag, TtIndex};
use fw_sfnt::{read_bytes, read_path, ReadOptions};
use read_fonts::{FontRef, TableProvider};
use write_fonts::{
    tables::name::{Name, NameRecord},
    types::NameId,
    FontBuilder,
};

#[test]
fn reads_dejavu_into_the_document_model() {
    let path = fw_fixtures::path("DejaVuSans.ttf");
    let bytes = fs::read(&path).expect("read fixture");
    let source = FontRef::new(&bytes).expect("parse fixture directly");
    let maxp = source.maxp().expect("maxp");
    let hmtx = source.hmtx().expect("hmtx");

    let outcome = read_path(&path, &ReadOptions::default()).expect("read DejaVuSans");
    assert_eq!(outcome.fonts.len(), 1);
    let font = &outcome.fonts[0];
    assert_eq!(font.glyphs.len(), usize::from(maxp.num_glyphs()));
    assert_eq!(font.info.family_name, "DejaVu Sans");
    assert_eq!(
        font.info.open_type_head_created.as_deref(),
        Some("2023/08/10 17:39:05")
    );

    let glyph_a = font.glyph("A").expect("A glyph");
    assert!(glyph_a.unicodes.contains(&0x41));
    let layer_a = glyph_a
        .master_layer(&font.default_master().id)
        .expect("A master layer");
    assert_eq!(
        layer_a.width,
        f64::from(
            hmtx.advance(read_fonts::types::GlyphId::new(36))
                .expect("A advance")
        )
    );
    assert!(!layer_a.contours.is_empty());
    assert!(layer_a
        .contours
        .iter()
        .flat_map(|c| &c.points)
        .all(|point| {
            matches!(point.tt_index, TtIndex::Index(_))
                && matches!(
                    point.kind,
                    PointKind::Line | PointKind::QCurve | PointKind::OffCurve
                )
        }));

    let aacute = font.glyph("Aacute").expect("Aacute glyph");
    let aacute_layer = aacute
        .master_layer(&font.default_master().id)
        .expect("Aacute master layer");
    assert_eq!(aacute_layer.components.len(), 2);
    assert!(aacute_layer
        .components
        .iter()
        .any(|component| component.transform.as_coeffs()[4] != 0.0
            || component.transform.as_coeffs()[5] != 0.0));

    assert!(font.extra.raw_tables.contains_key(&Tag(*b"GSUB")));
    assert!(font.extra.raw_tables.contains_key(&Tag(*b"GPOS")));
    assert!(!font.extra.raw_tables.contains_key(&Tag(*b"glyf")));
    assert!(!font.extra.raw_tables.contains_key(&Tag(*b"hmtx")));
    assert!(!font.extra.raw_tables.contains_key(&Tag(*b"DSIG")));
    assert!(!font.default_master().tt.prep.is_empty());

    let provenance = font.extra.source.as_ref().expect("source provenance");
    assert_eq!(provenance.path.as_deref(), path.to_str());
    assert_eq!(provenance.format, "ttf");
    assert_eq!(
        provenance.sha256,
        "ae7b7855e115a5966d8b1b3f80f254ccc117ec86f9965e202ee2940453837280"
    );
    assert!(provenance
        .sha256
        .bytes()
        .all(|byte| byte.is_ascii_hexdigit()));
    let expected_tags: Vec<_> = source
        .table_directory()
        .table_records()
        .iter()
        .map(|record| Tag(record.tag().to_be_bytes()))
        .collect();
    assert_eq!(provenance.table_tags, expected_tags);
    assert_eq!(
        provenance.glyph_order,
        font.glyph_order().cloned().collect::<Vec<_>>()
    );
    assert_eq!(provenance.original_bytes.as_deref(), Some(&bytes));

    for bogus in [0x0000, 0x0008, 0x0080] {
        assert!(
            font.glyphs
                .values()
                .all(|glyph| !glyph.unicodes.contains(&bogus)),
            "Mac Roman byte U+{bogus:04X} leaked into Unicode mappings"
        );
    }
}

#[test]
fn reads_or_derives_x_height_and_cap_height() {
    let dejavu = read_path(
        &fw_fixtures::path("DejaVuSans.ttf"),
        &ReadOptions::default(),
    )
    .expect("read DejaVu Sans");
    let dejavu_metrics = &dejavu.fonts[0].default_master().metrics;
    assert_eq!(dejavu_metrics.x_height, 1_120.0);
    assert_eq!(dejavu_metrics.cap_height, 1_493.0);

    let noto_path = fw_fixtures::path("NotoSans-Regular.ttf");
    let noto_bytes = fs::read(&noto_path).expect("read Noto Sans fixture");
    let noto_source = FontRef::new(&noto_bytes).expect("parse Noto Sans directly");
    let noto_os2 = noto_source.os2().expect("Noto Sans OS/2 table");
    assert!(noto_os2.version() >= 2);
    assert_eq!(noto_os2.sx_height(), Some(536));
    assert_eq!(noto_os2.s_cap_height(), Some(714));

    let noto = read_path(&noto_path, &ReadOptions::default()).expect("read Noto Sans");
    let noto_metrics = &noto.fonts[0].default_master().metrics;
    assert_eq!(noto_metrics.x_height, 536.0);
    assert_eq!(noto_metrics.cap_height, 714.0);
}

#[test]
fn infers_smooth_points_from_quadratic_tangents() {
    let outcome = read_path(
        &fw_fixtures::path("DejaVuSans.ttf"),
        &ReadOptions::default(),
    )
    .expect("read DejaVu Sans");
    let font = &outcome.fonts[0];

    let o_points: Vec<_> = font
        .glyph("o")
        .expect("o glyph")
        .layers
        .iter()
        .flat_map(|layer| &layer.contours)
        .flat_map(|contour| &contour.points)
        .filter(|point| point.kind != PointKind::OffCurve)
        .collect();
    assert!(!o_points.is_empty());
    assert!(o_points.iter().all(|point| point.smooth));

    let h_points: Vec<_> = font
        .glyph("H")
        .expect("H glyph")
        .layers
        .iter()
        .flat_map(|layer| &layer.contours)
        .flat_map(|contour| &contour.points)
        .filter(|point| point.kind != PointKind::OffCurve)
        .collect();
    assert!(!h_points.is_empty());
    assert!(h_points.iter().all(|point| !point.smooth));
}

#[test]
fn can_drop_original_bytes_without_losing_provenance() {
    let path = fw_fixtures::path("NotoSans-Regular.ttf");
    let outcome = read_path(
        &path,
        &ReadOptions {
            keep_original_bytes: false,
            ..ReadOptions::default()
        },
    )
    .expect("read Noto Sans");
    let provenance = outcome.fonts[0]
        .extra
        .source
        .as_ref()
        .expect("source provenance");
    assert!(provenance.original_bytes.is_none());
    assert_eq!(provenance.sha256.len(), 64);
    assert_eq!(
        provenance.glyph_order,
        outcome.fonts[0].glyph_order().cloned().collect::<Vec<_>>()
    );
}

#[test]
fn drops_dsig_while_preserving_other_raw_tables() {
    let bytes = fs::read(fw_fixtures::path("DejaVuSans.ttf")).expect("read fixture");
    let source = FontRef::new(&bytes).expect("parse fixture directly");
    let mut builder = FontBuilder::new();
    builder
        .add_raw(write_fonts::types::Tag::new(b"DSIG"), vec![0; 8])
        .copy_missing_tables(source);
    let with_dsig = builder.build();
    let rebuilt = FontRef::new(&with_dsig).expect("parse DSIG-bearing font");
    assert!(rebuilt
        .table_data(read_fonts::types::Tag::new(b"DSIG"))
        .is_some());

    let outcome = read_bytes(&with_dsig, &ReadOptions::default()).expect("import DSIG font");
    let raw_tables = &outcome.fonts[0].extra.raw_tables;
    assert!(!raw_tables.contains_key(&Tag(*b"DSIG")));
    assert!(raw_tables.contains_key(&Tag(*b"GSUB")));
}

#[test]
fn keeps_iso_and_custom_platform_name_records() {
    let bytes = fs::read(fw_fixtures::path("DejaVuSans.ttf")).expect("read fixture");
    let source = FontRef::new(&bytes).expect("parse fixture directly");
    let table = Name::new(vec![
        NameRecord::new(0, 1, 0, NameId::new(250), "ISO name".to_owned().into()),
        NameRecord::new(0, 1, 0, NameId::new(251), "Custom name".to_owned().into()),
    ]);
    let mut name_bytes = write_fonts::dump_table(&table).expect("build synthetic name table");
    name_bytes[6..8].copy_from_slice(&2_u16.to_be_bytes());
    name_bytes[18..20].copy_from_slice(&4_u16.to_be_bytes());
    let mut builder = FontBuilder::new();
    builder
        .add_raw(write_fonts::types::Tag::new(b"name"), name_bytes)
        .copy_missing_tables(source);

    let outcome = read_bytes(&builder.build(), &ReadOptions::default()).expect("import font");
    let names = &outcome.fonts[0].info.names;
    assert!(names.iter().any(|record| {
        record.name_id == 250
            && record.platform == NamePlatform::Other(2)
            && record.text == "ISO name"
    }));
    assert!(names.iter().any(|record| {
        record.name_id == 251
            && record.platform == NamePlatform::Other(4)
            && record.text == "Custom name"
    }));
}
