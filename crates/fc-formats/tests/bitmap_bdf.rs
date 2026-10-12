use fc_formats::bitmap::bdf;

const BDF: &str = "STARTFONT 2.2\nFONT Test\nSIZE 16 72 72\nFONTBOUNDINGBOX 9 3 -1 -1\nSTARTPROPERTIES 5\nFONT_ASCENT 12\nFONT_DESCENT 4\nFAMILY_NAME \"Test \"\"quotes\"\"\"\nCHARSET_REGISTRY \"ISO10646\"\nCHARSET_ENCODING \"1\"\nENDPROPERTIES\nCHARS 2\nSTARTCHAR uni4E00\nENCODING 19968\nSWIDTH 1000 0\nDWIDTH 16 0\nBBX 9 3 -1 -1\nBITMAP\nFF80\n8080\nFF80\nENDCHAR\nSTARTCHAR space\nENCODING 32\nSWIDTH 500 0\nDWIDTH 8 0\nBBX 0 0 0 0\nBITMAP\nENDCHAR\nENDFONT\n";

#[test]
fn bdf_roundtrip_preserves_pixels_bearings_encodings_and_scalable_widths() {
    let source = BDF.replace("DWIDTH 16 0", "DWIDTH 16 0\nATTRIBUTES 00FF");
    let font = bdf::decode(source.as_bytes()).unwrap();
    assert_eq!(font.info.family_name, "Test \"quotes\"");
    assert_eq!(font.glyph("uni4E00").unwrap().unicodes, vec![19968]);
    assert_eq!(font.glyph("uni4E00").unwrap().layers[0].width, 1000.0);
    let bytes = bdf::encode(&font, &font.bitmap_strikes[0]).unwrap();
    let round = bdf::decode(&bytes).unwrap();
    assert_eq!(
        round.glyph("uni4E00").unwrap().user_data,
        font.glyph("uni4E00").unwrap().user_data
    );
    assert_eq!(
        round.bitmap_strikes[0].glyphs,
        font.bitmap_strikes[0].glyphs
    );
    assert_eq!(round.bitmap_strikes[0].ascent, 12);
    assert_eq!(round.bitmap_strikes[0].descent, 4);
}

#[test]
fn malformed_counts_pixels_encoding_and_limits_fail() {
    for malformed in [
        BDF.replace("CHARS 2", "CHARS 3"),
        BDF.replace("FF80", "FF81"),
        BDF.replace("BBX 9 3", "BBX 9000 3"),
        BDF.replace("ENCODING 19968", "ENCODING 1114112"),
        BDF.replace("ENDCHAR", ""),
        BDF.replace("ENDFONT", ""),
        BDF.replace("DWIDTH 16 0", "DWIDTH 16 1"),
    ] {
        assert!(
            bdf::decode(malformed.as_bytes()).is_err(),
            "accepted {malformed}"
        );
    }
}

#[test]
fn grayscale_and_vertical_metrics_survive() {
    let source = BDF
        .replace("SIZE 16 72 72", "SIZE 16 72 72 4")
        .replace("9 3", "2 3")
        .replace("FF80", "F0")
        .replace("8080", "0F")
        .replace(
            "SWIDTH 1000 0",
            "SWIDTH 1000 0\nSWIDTH1 0 -1000\nDWIDTH1 0 -16\nVVECTOR 8 12",
        )
        .replace(
            "SWIDTH 500 0",
            "SWIDTH 500 0\nSWIDTH1 0 -1000\nDWIDTH1 0 -16",
        );
    let font = bdf::decode(source.as_bytes()).unwrap();
    let strike = &font.bitmap_strikes[0];
    assert_eq!(strike.depth, 4);
    assert_eq!(strike.glyphs["uni4E00"].bitmap.pixel(0, 0), 15);
    assert_eq!(strike.glyphs["uni4E00"].vertical_advance, Some(16));
    let round = bdf::decode(&bdf::encode(&font, strike).unwrap()).unwrap();
    assert_eq!(round.bitmap_strikes[0].glyphs, strike.glyphs);
}

#[test]
fn gzip_and_failed_export_leave_destination_intact() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bitmap.bdf.gz");
    let mut font = bdf::decode(BDF.as_bytes()).unwrap();
    bdf::write(&font, &path, None).unwrap();
    assert_eq!(
        bdf::read(&path).unwrap().bitmap_strikes[0].glyphs,
        font.bitmap_strikes[0].glyphs
    );
    let original = std::fs::read(&path).unwrap();
    font.bitmap_strikes[0].depth = 32;
    assert!(bdf::write(&font, &path, None).is_err());
    assert_eq!(std::fs::read(path).unwrap(), original);
}
