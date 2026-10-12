use assert_cmd::Command;
use fc_model::{Bitmap, BitmapGlyph, BitmapStrike, Font, Glyph, Layer};

#[test]
fn convert_opens_embedded_strikes_and_selects_psf1_explicitly() {
    let directory = tempfile::tempdir().unwrap();
    let mut font = Font::new_empty("CLI bitmap", 1000);
    let master = font.default_master().id.clone();
    let mut strike = BitmapStrike {
        ppem: 8,
        depth: 1,
        ascent: 8,
        ..Default::default()
    };
    for code in 0..256 {
        let name = format!("glyph{code:05}");
        let mut glyph = Glyph::new(name.clone());
        glyph.unicodes.push(code);
        let mut layer = Layer::new_master(master.clone());
        layer.width = 1000.0;
        glyph.layers.push(layer);
        font.insert_glyph(glyph).unwrap();
        let mut bitmap = Bitmap::new(8, 8, 1).unwrap();
        bitmap.set_pixel((code % 8) as u16, 2, 1).unwrap();
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
    let bdf = directory.path().join("source.bdf");
    let otb = directory.path().join("embedded.otb");
    let pcf = directory.path().join("roundtrip.pcf");
    let psf = directory.path().join("legacy.psf");
    fc_formats::bitmap::bdf::write(&font, &bdf, None).unwrap();
    for (input, output) in [(&bdf, &otb), (&otb, &pcf)] {
        Command::new(assert_cmd::cargo::cargo_bin!("fontcarver"))
            .arg("convert")
            .arg(input)
            .arg(output)
            .assert()
            .success();
    }
    let roundtrip = fc_formats::bitmap::read(&pcf).unwrap();
    for (name, image) in &font.bitmap_strikes[0].glyphs {
        assert_eq!(roundtrip.bitmap_strikes[0].glyphs.get(name), Some(image));
    }
    Command::new(assert_cmd::cargo::cargo_bin!("fontcarver"))
        .arg("convert")
        .arg(&bdf)
        .arg(&psf)
        .args(["--psf1", "--ppem", "8", "--depth", "1"])
        .assert()
        .success();
    assert_eq!(&std::fs::read(&psf).unwrap()[..2], &[0x36, 0x04]);
    assert_eq!(fc_formats::bitmap::read(&psf).unwrap().glyphs.len(), 256);
}
