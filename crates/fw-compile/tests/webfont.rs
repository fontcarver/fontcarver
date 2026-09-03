use fw_compile::{wrap_woff1, wrap_woff2};
use read_fonts::{FontRef, TableProvider};

#[test]
fn woff1_round_trips_every_table() {
    let sfnt = std::fs::read(fw_fixtures::path("DejaVuSans.ttf")).expect("read fixture");
    let wrapped = wrap_woff1(&sfnt).expect("encode WOFF1");
    assert_eq!(&wrapped[..4], b"wOFF");
    let decoded = wuff::decompress_woff1(&wrapped).expect("decode WOFF1");
    assert_tables_equal(&sfnt, &decoded);
}

#[test]
fn woff2_round_trips_to_a_valid_font() {
    let sfnt = std::fs::read(fw_fixtures::path("NotoSans-Regular.ttf")).expect("read fixture");
    let wrapped = wrap_woff2(&sfnt).expect("encode WOFF2");
    assert_eq!(&wrapped[..4], b"wOF2");
    let decoded = wuff::decompress_woff2(&wrapped).expect("decode WOFF2");
    let original = FontRef::new(&sfnt).expect("parse original");
    let roundtrip = FontRef::new(&decoded).expect("parse WOFF2 roundtrip");
    assert_eq!(
        original.maxp().expect("original maxp").num_glyphs(),
        roundtrip.maxp().expect("roundtrip maxp").num_glyphs()
    );
}

fn assert_tables_equal(left: &[u8], right: &[u8]) {
    let left = FontRef::new(left).expect("parse source");
    let right = FontRef::new(right).expect("parse roundtrip");
    for record in left.table_directory().table_records() {
        assert_eq!(
            left.table_data(record.tag()).map(|data| data.as_bytes()),
            right.table_data(record.tag()).map(|data| data.as_bytes()),
            "table {} changed",
            record.tag()
        );
    }
}
