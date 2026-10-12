use std::{fmt::Write, process::Command, sync::OnceLock};

use fc_formats::type42::{read_bytes, write_bytes};
use fc_model::{Contour, Font, Glyph, Layer, Point, PointKind};
use read_fonts::{FontRef, TableProvider};

fn sample_sfnt() -> &'static [u8] {
    static BYTES: OnceLock<Vec<u8>> = OnceLock::new();
    BYTES.get_or_init(|| {
        let mut font = Font::new_empty("Type 42 Test", 1000);
        font.info.postscript_font_name = Some("Type42Test-Regular".into());
        let master = font.default_master().id.clone();
        for (name, unicode, width) in [
            (".notdef", None, 500.0),
            ("space", Some(32), 250.0),
            ("A", Some(65), 610.0),
            ("Snowman", Some(0x2603), 700.0),
        ] {
            let mut glyph = Glyph::new(name);
            glyph.unicodes.extend(unicode);
            let mut layer = Layer::new_master(master.clone());
            layer.width = width;
            if name == "A" || name == "Snowman" {
                layer.contours.push(Contour {
                    closed: true,
                    points: [(10.0, 20.0), (300.0, 20.0), (300.0, 700.0), (10.0, 700.0)]
                        .map(|(x, y)| Point {
                            x,
                            y,
                            kind: PointKind::Line,
                            ..Point::default()
                        })
                        .to_vec(),
                    ..Contour::default()
                });
            }
            glyph.layers.push(layer);
            font.insert_glyph(glyph).unwrap();
        }
        fc_compile::compile_static(&font, &master, &Default::default()).unwrap()
    })
}

fn wrapped_sample() -> String {
    String::from_utf8(write_bytes(sample_sfnt()).unwrap()).unwrap()
}

fn declaration_prefix(source: &str, declaration: &str) -> usize {
    source.find(declaration).unwrap()
}

fn before_font_type(source: &str, extra: &str) -> String {
    let offset = declaration_prefix(source, "/FontType");
    format!("{}{}\n{}", &source[..offset], extra, &source[offset..])
}

fn replace_sfnts(source: &str, sfnt: &[u8]) -> String {
    let sfnts = declaration_prefix(source, "/sfnts");
    let open = sfnts + source[sfnts..].find('[').unwrap();
    let close = open + source[open..].find(']').unwrap() + 1;
    let mut replacement = String::from("[<");
    for byte in sfnt {
        write!(&mut replacement, "{byte:02x}").unwrap();
    }
    replacement.push_str(">]");
    format!("{}{}{}", &source[..open], replacement, &source[close..])
}

fn hex_chunks(source: &str) -> Vec<Vec<u8>> {
    let sfnts = &source[declaration_prefix(source, "/sfnts") + "/sfnts".len()..];
    let array = sfnts.split_once('[').unwrap().1.split_once(']').unwrap().0;
    let mut remaining = array;
    let mut chunks = Vec::new();
    while let Some((_, after_open)) = remaining.split_once('<') {
        let (hex, after_close) = after_open.split_once('>').unwrap();
        let compact: String = hex.chars().filter(|ch| !ch.is_whitespace()).collect();
        assert_eq!(compact.len() % 2, 0);
        chunks.push(
            compact
                .as_bytes()
                .chunks_exact(2)
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect(),
        );
        remaining = after_close;
    }
    chunks
}

#[test]
fn wrapper_preserves_truetype_bytes_and_glyph_identity() {
    let started = std::time::Instant::now();
    let source = wrapped_sample();
    assert!(source.starts_with("%!PS-TrueTypeFont-"));
    assert!(source.contains("/FontType 42 def"));
    assert!(source.contains("/FontName /Type42Test-Regular def"));
    assert_eq!(read_bytes(source.as_bytes()).unwrap(), sample_sfnt());

    let encoding = &source[declaration_prefix(&source, "/Encoding")..];
    let encoding = encoding
        .split_once('[')
        .unwrap()
        .1
        .split_once(']')
        .unwrap()
        .0;
    let names: Vec<_> = encoding.split_whitespace().collect();
    assert_eq!(names.len(), 256);
    assert_eq!(names[0], "/.notdef");
    assert_eq!(names[32], "/space");
    assert_eq!(names[65], "/A");

    let face = FontRef::new(sample_sfnt()).unwrap();
    let post = face.post().unwrap();
    let charstrings = &source[declaration_prefix(&source, "/CharStrings")..];
    for gid in 0..face.maxp().unwrap().num_glyphs() {
        let name = post
            .glyph_name(read_fonts::types::GlyphId16::new(gid))
            .unwrap();
        assert!(
            charstrings.contains(&format!("/{name} {gid} def")),
            "missing CharStrings glyph {name} at GID {gid}"
        );
    }
    assert!(started.elapsed() < std::time::Duration::from_secs(30));
}

#[test]
fn reads_independent_literal_sfnts_and_conventional_encoding_loop() {
    let mut source = String::from(
        "%!PS-TrueTypeFont-1.0-1.0\n\
         12 dict begin\n\
         /FontName /Type42Test-Regular def\n\
         /FontType 42 def\n\
         /PaintType 0 def\n\
         /FontMatrix [1 0 0 1 0 0] def\n\
         /FontBBox [0.01 0.02 0.3 0.7] def\n\
         /Encoding 256 array\n\
         0 1 255 {1 index exch /.notdef put} for\n\
         dup 32 /space put\n\
         dup 65 /A put\n\
         readonly def\n\
         /CharStrings 4 dict dup begin\n",
    );
    let face = FontRef::new(sample_sfnt()).unwrap();
    let post = face.post().unwrap();
    for gid in 0..face.maxp().unwrap().num_glyphs() {
        let name = post
            .glyph_name(read_fonts::types::GlyphId16::new(gid))
            .unwrap();
        writeln!(&mut source, "/{name} {gid} def").unwrap();
    }
    source.push_str("end readonly def\n/sfnts [\n<\n");
    for (index, byte) in sample_sfnt().iter().enumerate() {
        write!(&mut source, "{byte:02x}").unwrap();
        if index % 23 == 22 {
            source.push('\n');
        }
    }
    source.push_str("\n>\n] def\nFontName currentdict end definefont pop\n");
    assert_eq!(read_bytes(source.as_bytes()).unwrap(), sample_sfnt());
}

#[test]
fn large_sfnt_uses_bounded_strings_and_roundtrips_exactly() {
    let sfnt = std::fs::read(fc_fixtures::path("DejaVuSans.ttf")).unwrap();
    assert!(sfnt.len() > 65535);
    let source = String::from_utf8(write_bytes(&sfnt).unwrap()).unwrap();
    let chunks = hex_chunks(&source);
    assert!(chunks.len() > 1);
    for chunk in &chunks {
        assert!(chunk.len() <= 65535, "oversized PostScript string");
        assert!(!chunk.is_empty());
    }
    assert_eq!(read_bytes(source.as_bytes()).unwrap(), sfnt);
}

#[test]
fn rejects_invalid_sfnt_truncated_tables_collections_and_cff() {
    for invalid in [b"".as_slice(), b"not a font", b"ttcf\0\x01\0\0"] {
        assert!(write_bytes(invalid).is_err());
    }
    let sfnt = sample_sfnt();
    let table_count = usize::from(u16::from_be_bytes(sfnt[4..6].try_into().unwrap()));
    let last_table_end = sfnt[12..12 + table_count * 16]
        .chunks_exact(16)
        .map(|entry| {
            let offset = u32::from_be_bytes(entry[8..12].try_into().unwrap());
            let length = u32::from_be_bytes(entry[12..16].try_into().unwrap());
            (offset + length) as usize
        })
        .max()
        .unwrap();
    for length in [4, 12, sfnt.len() / 2, last_table_end - 1] {
        assert!(write_bytes(&sfnt[..length]).is_err());
        assert!(read_bytes(replace_sfnts(&wrapped_sample(), &sfnt[..length]).as_bytes()).is_err());
    }
    let mut invalid_range = sfnt.to_vec();
    invalid_range[20..24].copy_from_slice(&u32::MAX.to_be_bytes());
    assert!(write_bytes(&invalid_range).is_err());
    assert!(read_bytes(replace_sfnts(&wrapped_sample(), &invalid_range).as_bytes()).is_err());

    let cff = std::fs::read(fc_fixtures::path("SourceSans3Min.otf")).unwrap();
    assert!(write_bytes(&cff).is_err());
    let wrapped_cff = replace_sfnts(&wrapped_sample(), &cff);
    assert!(read_bytes(wrapped_cff.as_bytes()).is_err());
}

#[test]
fn rejects_malformed_hex_and_computed_or_truncated_sfnts() {
    let valid = wrapped_sample();
    let sfnts_offset = declaration_prefix(&valid, "/sfnts");
    let hex_offset = sfnts_offset + valid[sfnts_offset..].find('<').unwrap() + 1;
    let mut bad_hex = valid.clone();
    bad_hex.insert(hex_offset, 'G');
    assert!(read_bytes(bad_hex.as_bytes()).is_err());
    let close_offset = hex_offset + valid[hex_offset..].find('>').unwrap();
    let mut missing_close = valid.clone();
    missing_close.remove(close_offset);
    assert!(read_bytes(missing_close.as_bytes()).is_err());
    assert!(read_bytes(&valid.as_bytes()[..close_offset]).is_err());
    for replacement in ["[42]", "[(literal)]", "[]", "{<00010000>}", "1 array"] {
        let open = sfnts_offset + valid[sfnts_offset..].find('[').unwrap();
        let close = open + valid[open..].find(']').unwrap() + 1;
        let malformed = format!("{}{}{}", &valid[..open], replacement, &valid[close..]);
        assert!(read_bytes(malformed.as_bytes()).is_err(), "{replacement}");
    }
}

#[test]
fn rejects_duplicate_declarations_and_unrepresentable_overrides() {
    let valid = wrapped_sample();
    for declaration in [
        "/FontType 42 def",
        "/sfnts [] def",
        "/Metrics 1 dict dup begin /A 999 def end def",
        "/Metrics2 1 dict def",
        "/BuildGlyph { pop pop } def",
        "/BuildChar { pop pop } def",
    ] {
        let source = before_font_type(&valid, declaration);
        assert!(read_bytes(source.as_bytes()).is_err(), "{declaration}");
    }
    assert!(read_bytes(valid.replace("/FontType 42", "/FontType 1").as_bytes()).is_err());
    assert!(read_bytes(valid.replace("/PaintType 0", "/PaintType 2").as_bytes()).is_err());
    assert!(read_bytes(
        valid
            .replace("/FontMatrix [1 0 0 1 0 0]", "/FontMatrix [2 0 0 2 0 0]")
            .as_bytes()
    )
    .is_err());
}

#[test]
fn rejects_executable_postscript_without_running_it() {
    let valid = wrapped_sample();
    for executable in [
        "(untrusted.ps) run",
        "(untrusted.ps) (w) file",
        "currentfile closefile",
        "quit",
        "{ 1 2 add } exec",
        "/FontType { 42 } def",
        "/sfnts currentfile 1024 string readstring pop def",
    ] {
        let source = before_font_type(&valid, executable);
        assert!(read_bytes(source.as_bytes()).is_err(), "{executable}");
    }
    assert!(read_bytes(format!("{valid}\nquit\n").as_bytes()).is_err());
}

#[test]
fn ghostscript_loads_encoding_glyph_mapping_metrics_and_multistring_font() {
    if !Command::new("gs")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
    {
        eprintln!("skipping independent Type 42 validation: Ghostscript is unavailable");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    for (name, bytes, checks) in [
        (
            "small.t42",
            write_bytes(sample_sfnt()).unwrap(),
            "/Type42Test-Regular findfont dup /FontType get 42 ne {1 0 div} if \
             dup /Encoding get 65 get /A ne {1 0 div} if \
             dup /CharStrings get /Snowman known not {1 0 div} if \
             100 scalefont setfont (A) stringwidth pop 61 sub abs 0.001 gt {1 0 div} if \
             0 0 moveto (A) show showpage",
        ),
        (
            "large.t42",
            write_bytes(&std::fs::read(fc_fixtures::path("DejaVuSans.ttf")).unwrap()).unwrap(),
            "/DejaVuSans findfont dup /FontType get 42 ne {1 0 div} if \
             20 scalefont setfont 0 0 moveto (ABC) show showpage",
        ),
    ] {
        let path = dir.path().join(name);
        std::fs::write(&path, bytes).unwrap();
        let output = Command::new("gs")
            .args([
                "-q",
                "-dSAFER",
                "-dBATCH",
                "-dNOPAUSE",
                "-sDEVICE=nullpage",
                "-f",
            ])
            .arg(&path)
            .args(["-c", checks])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{name}: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
