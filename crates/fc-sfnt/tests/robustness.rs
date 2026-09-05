use std::{fs, panic};

use fc_sfnt::{read_bytes, Error, ReadOptions};

const FIXTURES: &[&str] = &[
    "DejaVuSans.ttf",
    "DejaVuSans.woff",
    "DejaVuSansMin.woff2",
    "NotoSans-Regular.ttf",
    "NotoSerifTibetan-Regular.ttf",
    "SourceSans3Min.otf",
    "SourceSerif4Min.otf",
];

#[test]
fn truncated_fixtures_never_panic() {
    for fixture in FIXTURES {
        let bytes = fs::read(fc_fixtures::path(fixture)).expect("read fixture");
        for step in 0..10 {
            let cut = bytes.len() * step / 10;
            let result = panic::catch_unwind(|| {
                let _ = read_bytes(&bytes[..cut], &ReadOptions::default());
            });
            assert!(result.is_ok(), "{fixture} panicked when truncated at {cut}");
        }
    }
}

#[test]
fn malformed_supported_containers_report_parse_errors() {
    for bytes in [
        b"\0\x01\0\0".as_slice(),
        b"wOFF".as_slice(),
        b"wOF2".as_slice(),
    ] {
        assert!(matches!(
            read_bytes(bytes, &ReadOptions::default()),
            Err(Error::Parse { .. })
        ));
    }
}
