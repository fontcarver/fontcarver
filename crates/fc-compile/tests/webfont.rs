use fc_compile::{wrap_woff1, wrap_woff2};
use read_fonts::{FontRef, TableProvider};

#[test]
fn woff1_round_trips_every_table() {
    let sfnt = std::fs::read(fc_fixtures::path("DejaVuSans.ttf")).expect("read fixture");
    let wrapped = wrap_woff1(&sfnt).expect("encode WOFF1");
    assert_eq!(&wrapped[..4], b"wOFF");
    let decoded = wuff::decompress_woff1(&wrapped).expect("decode WOFF1");
    assert_tables_equal(&sfnt, &decoded);
}

#[test]
fn woff2_round_trips_to_a_valid_font() {
    let sfnt = std::fs::read(fc_fixtures::path("NotoSans-Regular.ttf")).expect("read fixture");
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

#[test]
fn external_color_font_webfont_work_budget() {
    let Ok(path) = std::env::var("FONTCARVER_COLOR_FONT") else {
        return;
    };
    let sfnt = std::fs::read(path).expect("read external color font");
    let started = std::time::Instant::now();
    let woff = wrap_woff1(&sfnt).expect("encode color WOFF");
    let elapsed = started.elapsed();
    eprintln!("Color WOFF: {} bytes in {elapsed:?}", woff.len());
    assert_tables_equal(&sfnt, &wuff::decompress_woff1(&woff).unwrap());
    assert!(
        elapsed < std::time::Duration::from_secs(30),
        "color WOFF compression exceeded 30s: {elapsed:?}"
    );
}
