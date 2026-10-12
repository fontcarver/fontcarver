use fc_formats::metrics::pfm::{read_pfm, write_pfm, PfmKernPair};

fn word(bytes: &mut [u8], at: usize, value: u16) {
    bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
}

fn long(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

// Independently assembled Windows v1 PostScript PFM fixture, no writer involved.
fn fixture() -> Vec<u8> {
    let mut b = vec![0; 147];
    word(&mut b, 0, 0x100);
    b[6..10].copy_from_slice(b"Test");
    word(&mut b, 66, 0x81);
    word(&mut b, 68, 10);
    word(&mut b, 70, 300);
    word(&mut b, 72, 300);
    word(&mut b, 74, 750);
    word(&mut b, 83, 400);
    word(&mut b, 88, 1000);
    b[90] = 0x21;
    word(&mut b, 91, 600);
    word(&mut b, 93, 700);
    b[95] = 65;
    b[96] = 66;
    word(&mut b, 117, 30);
    long(&mut b, 119, 147);
    b.extend_from_slice(&[0; 52]);
    word(&mut b, 147, 52);
    word(&mut b, 149, 200);
    word(&mut b, 153, 300);
    word(&mut b, 155, 1);
    word(&mut b, 157, 1000);
    word(&mut b, 159, 1000);
    word(&mut b, 161, 700);
    word(&mut b, 163, 450);
    word(&mut b, 165, 730);
    word(&mut b, 167, 200);
    word(&mut b, 195, 1);
    long(&mut b, 123, 199);
    b.extend_from_slice(&500u16.to_le_bytes());
    b.extend_from_slice(&700u16.to_le_bytes());
    long(&mut b, 131, 203);
    b.extend_from_slice(&[1, 0, 65, 66]);
    b.extend_from_slice(&(-80i16).to_le_bytes());
    long(&mut b, 101, 209);
    b.extend_from_slice(b"PostScript\0");
    long(&mut b, 105, 220);
    b.extend_from_slice(b"Example\xe9\0");
    long(&mut b, 139, 229);
    b.extend_from_slice(b"Example-Regular\0");
    let size = b.len() as u32;
    long(&mut b, 2, size);
    b
}

#[test]
fn parses_independent_binary_and_roundtrips_all_semantics() {
    let metrics = read_pfm(&fixture()).unwrap();
    assert_eq!(metrics.face_name, b"Example\xe9");
    assert_eq!(metrics.device_name, b"PostScript");
    assert_eq!(metrics.first_char, 65);
    assert_eq!(metrics.widths, [500, 700]);
    assert_eq!(metrics.header.ascent, 750);
    assert_eq!(metrics.extended.lowercase_ascent, 730);
    assert_eq!(
        metrics.kerning,
        [PfmKernPair {
            left: 65,
            right: 66,
            value: -80
        }]
    );
    assert_eq!(read_pfm(&write_pfm(&metrics).unwrap()).unwrap(), metrics);
}

#[test]
fn rejects_every_truncation_without_panicking() {
    let bytes = fixture();
    for length in 0..bytes.len() {
        assert!(read_pfm(&bytes[..length]).is_err(), "length {length}");
    }
}

#[test]
fn rejects_offsets_counts_versions_and_unsupported_fields() {
    let cases: &[(usize, &[u8])] = &[
        (0, &[0, 2]),
        (2, &[0xff, 0xff, 0xff, 0xff]),
        (66, &[0x80, 0]),
        (80, &[2]),
        (95, &[67]),
        (97, &[2]),
        (99, &[1, 0]),
        (101, &[1, 0, 0, 0]),
        (105, &[0xff, 0xff, 0xff, 0xff]),
        (109, &[1, 0, 0, 0]),
        (113, &[1, 0, 0, 0]),
        (117, &[32, 0]),
        (119, &[0, 0, 0, 0]),
        (123, &[0xfe, 0xff, 0xff, 0xff]),
        (127, &[199, 0, 0, 0]),
        (131, &[0, 0, 0, 0]),
        (135, &[1, 0, 0, 0]),
        (139, &[1, 0, 0, 0]),
        (143, &[1, 0, 0, 0]),
        (147, &[54, 0]),
        (159, &[0, 0]),
        (195, &[2, 0]),
        (197, &[1, 0]),
        (203, &[255, 255]),
        (205, &[64]),
    ];
    for &(offset, replacement) in cases {
        let mut bytes = fixture();
        bytes[offset..offset + replacement.len()].copy_from_slice(replacement);
        assert!(read_pfm(&bytes).is_err(), "offset {offset}");
    }
}

#[test]
fn rejects_overlaps_unterminated_strings_and_unclaimed_payload() {
    let mut overlap = fixture();
    long(&mut overlap, 105, 209);
    assert!(read_pfm(&overlap).is_err());
    let mut missing_nul = fixture();
    *missing_nul.last_mut().unwrap() = b'X';
    assert!(read_pfm(&missing_nul).is_err());
    let mut extra = fixture();
    extra.push(7);
    let size = extra.len() as u32;
    long(&mut extra, 2, size);
    assert!(read_pfm(&extra).is_err());
}

#[test]
fn writer_rejects_lossy_or_unrepresentable_metrics() {
    let metrics = read_pfm(&fixture()).unwrap();
    let mut bad = metrics.clone();
    bad.widths.clear();
    assert!(write_pfm(&bad).is_err());
    bad = metrics.clone();
    bad.widths.resize(256, 0);
    assert!(write_pfm(&bad).is_err());
    bad = metrics.clone();
    bad.kerning.push(bad.kerning[0].clone());
    assert!(write_pfm(&bad).is_err());
    bad = metrics.clone();
    bad.face_name.push(0);
    assert!(write_pfm(&bad).is_err());
    bad = metrics.clone();
    bad.copyright.push(0);
    assert!(write_pfm(&bad).is_err());
    bad = metrics;
    bad.extended.master_units = 2048;
    assert!(write_pfm(&bad).is_err());
}

#[test]
fn mutated_input_never_panics() {
    let bytes = fixture();
    for index in 0..bytes.len() {
        for value in [0, 0x7f, 0x80, 0xff] {
            let mut mutated = bytes.clone();
            mutated[index] = value;
            let _ = read_pfm(&mutated);
        }
    }
}
