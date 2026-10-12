use fc_formats::bitmap::{bdf, fnt, pcf, psf};
use fc_model::{Bitmap, BitmapGlyph, BitmapStrike, Font, Glyph};

fn font() -> Font {
    let mut font = Font::new_empty("Bitmap test", 1000);
    let mut strike = BitmapStrike {
        ppem: 16,
        depth: 1,
        ascent: 16,
        descent: 0,
        ..Default::default()
    };
    for (name, code) in [("space", 32), ("A", 65), ("uni4E00", 19968)] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes.push(code);
        font.insert_glyph(glyph).unwrap();
        let mut bitmap = Bitmap::new(8, 16, 1).unwrap();
        if code != 32 {
            bitmap.set_pixel(3, 5, 1).unwrap();
        }
        strike.glyphs.insert(
            name.into(),
            BitmapGlyph {
                bitmap,
                advance: 8,
                ..Default::default()
            },
        );
    }
    font.bitmap_strikes.push(strike);
    font
}

#[test]
fn pcf_preserves_bitmap_metrics_names_and_unicode() {
    let mut font = font();
    font.bitmap_strikes[0]
        .glyphs
        .get_mut("A")
        .unwrap()
        .bitmap
        .origin_x = -2;
    let encoded = pcf::encode(&font, &font.bitmap_strikes[0]).unwrap();
    let read = pcf::decode(&encoded).unwrap();
    assert_eq!(read.bitmap_strikes[0].glyphs, font.bitmap_strikes[0].glyphs);
    assert_eq!(read.glyph("uni4E00").unwrap().unicodes, vec![19968]);
    for cut in [0, 5, 20, encoded.len() - 10] {
        assert!(pcf::decode(&encoded[..cut]).is_err());
    }
}

#[test]
fn pcf_reads_x11_compiler_orders_padding_and_compressed_metrics() {
    if std::process::Command::new("bdftopcf")
        .arg("-help")
        .output()
        .is_err()
    {
        return;
    }
    let font = font();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sample.bdf");
    bdf::write(&font, &path, None).unwrap();
    for flags in [
        ["-p1", "-u1", "-m", "-M"],
        ["-p4", "-u4", "-l", "-M"],
        ["-p4", "-u2", "-m", "-L"],
        ["-p2", "-u2", "-l", "-L"],
    ] {
        let output = std::process::Command::new("bdftopcf")
            .args(flags)
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let read = pcf::decode(&output.stdout).unwrap();
        assert_eq!(
            read.bitmap_strikes[0].glyphs, font.bitmap_strikes[0].glyphs,
            "{flags:?}"
        );
    }
}

#[test]
fn psf2_unicode_aliases_and_pixels_roundtrip() {
    let mut font = font();
    font.glyph_mut("A").unwrap().unicodes.push(0x1f130);
    let bytes = psf::encode(&font, &font.bitmap_strikes[0], false).unwrap();
    let round = psf::decode(&bytes).unwrap();
    let glyph = round
        .glyphs
        .values()
        .find(|g| g.unicodes.contains(&65))
        .unwrap();
    assert_eq!(glyph.unicodes, vec![65, 0x1f130]);
    assert_eq!(
        round.bitmap_strikes[0].glyphs[&glyph.name]
            .bitmap
            .pixel(3, 5),
        1
    );
    assert!(psf::decode(&bytes[..bytes.len() - 1]).is_err());
    let mut invalid = bytes.clone();
    invalid[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(psf::decode(&invalid).is_err());
}

#[test]
fn psf1_roundtrip_and_no_implicit_clipping() {
    let mut font = Font::new_empty("PSF1", 1000);
    let mut strike = BitmapStrike {
        ppem: 8,
        depth: 1,
        ascent: 8,
        ..Default::default()
    };
    for i in 0..256 {
        let name = format!("glyph{i:05}");
        let mut glyph = Glyph::new(name.clone());
        glyph.unicodes.push(i);
        font.insert_glyph(glyph).unwrap();
        strike.glyphs.insert(
            name.into(),
            BitmapGlyph {
                bitmap: Bitmap::new(8, 8, 1).unwrap(),
                advance: 8,
                ..Default::default()
            },
        );
    }
    font.bitmap_strikes.push(strike);
    let bytes = psf::encode(&font, &font.bitmap_strikes[0], true).unwrap();
    let round = psf::decode(&bytes).unwrap();
    assert_eq!(round.glyphs.len(), 256);
    font.bitmap_strikes[0]
        .glyphs
        .get_mut("glyph00000")
        .unwrap()
        .bitmap
        .origin_x = -1;
    assert!(psf::encode(&font, &font.bitmap_strikes[0], true).is_err());
}

#[test]
fn windows_fnt_and_multistrike_fon_are_readable_by_freetype() {
    let mut font = font();
    font.glyphs.shift_remove("uni4E00");
    font.bitmap_strikes[0].glyphs.remove("uni4E00");
    let first = fnt::encode(&font, &font.bitmap_strikes[0]).unwrap();
    let read = fnt::decode(&first).unwrap();
    assert_eq!(
        read.bitmap_strikes[0].glyphs["char065"].bitmap.pixel(3, 5),
        1
    );
    let mut second = font.bitmap_strikes[0].clone();
    second.ppem = 12;
    font.bitmap_strikes.push(second);
    let fon = fnt::encode_fon(&font).unwrap();
    let read = fnt::decode_fon(&fon).unwrap();
    assert_eq!(read.bitmap_strikes.len(), 2);
    let dir = tempfile::tempdir().unwrap();
    for (name, bytes) in [("font.fnt", first), ("font.fon", fon)] {
        let path = dir.path().join(name);
        std::fs::write(&path, bytes).unwrap();
        if let Ok(result) = std::process::Command::new("fc-scan")
            .arg("--format")
            .arg("%{family}:%{pixelsize}")
            .arg(path)
            .output()
        {
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            assert!(String::from_utf8_lossy(&result.stdout).contains("Bitmap test"));
        }
    }
}

#[test]
fn windows_rejects_unrepresentable_encoding_and_corrupt_offsets() {
    let mut font = font();
    assert!(fnt::encode(&font, &font.bitmap_strikes[0]).is_err());
    font.glyphs.shift_remove("uni4E00");
    font.bitmap_strikes[0].glyphs.remove("uni4E00");
    let mut bytes = fnt::encode(&font, &font.bitmap_strikes[0]).unwrap();
    bytes[150..154].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(fnt::decode(&bytes).is_err());
}
