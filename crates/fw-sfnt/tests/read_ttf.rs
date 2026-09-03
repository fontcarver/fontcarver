use std::fs;

use fw_model::{PointKind, Tag, TtIndex};
use fw_sfnt::{read_bytes, read_path, ReadOptions};
use read_fonts::{FontRef, TableProvider};
use write_fonts::FontBuilder;

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
    assert_eq!(provenance.original_bytes.as_deref(), Some(&bytes));
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
