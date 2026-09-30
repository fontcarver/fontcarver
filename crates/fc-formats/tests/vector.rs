use fc_formats::vector::{read_svg, write_svg};
use fc_model::{Contour, Font, Glyph, Layer};
use kurbo::BezPath;

fn sample() -> Font {
    let mut font = Font::default();
    font.info.family_name = "Vector & Friends".into();
    font.info.units_per_em = 1000;
    font.masters[0].metrics.ascender = 800.0;
    font.masters[0].metrics.descender = -200.0;
    let mut glyph = Glyph::new("A");
    glyph.unicodes = vec![65, 0x1d400];
    let mut layer = Layer::new_master(font.default_master().id.clone());
    layer.width = 612.25;
    layer.contours = Contour::from_bezpath(
        &BezPath::from_svg("M10.25 20.5C20 100 90.5 110.25 190.75 20.5Z").unwrap(),
    );
    glyph.layers.push(layer);
    font.insert_glyph(glyph).unwrap();
    font
}

#[test]
fn svg_roundtrip_and_independent_xml_path_decode() {
    let start = std::time::Instant::now();
    let font = sample();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("font.svg");
    write_svg(&font, &font.default_master().id, &path).unwrap();
    let source = std::fs::read_to_string(&path).unwrap();
    let xml = roxmltree::Document::parse(&source).unwrap();
    let face = xml
        .descendants()
        .find(|n| n.has_tag_name("font-face"))
        .unwrap();
    assert_eq!(face.attribute("font-family"), Some("Vector & Friends"));
    let glyph = xml.descendants().find(|n| n.has_tag_name("glyph")).unwrap();
    let decoded = BezPath::from_svg(glyph.attribute("d").unwrap()).unwrap();
    assert_eq!(
        decoded,
        font.glyph("A").unwrap().layers[0].contours[0].to_bezpath()
    );
    let restored = read_svg(&path).unwrap();
    assert_eq!(restored.info.family_name, font.info.family_name);
    assert_eq!(restored.info.units_per_em, 1000);
    assert_eq!(restored.default_master().metrics.descender, -200.0);
    let a = restored.glyph("A").unwrap();
    assert_eq!(a.unicodes, [65, 0x1d400]);
    assert_eq!(a.layers[0].width, 612.25);
    assert_eq!(
        a.layers[0].contours,
        font.glyph("A").unwrap().layers[0].contours
    );
    assert!(start.elapsed() < std::time::Duration::from_secs(10));
}

#[test]
fn svg_source_imports_default_width_names_and_missing_glyph() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("source.svg");
    std::fs::write(&path, r#"<svg xmlns="http://www.w3.org/2000/svg"><defs>
      <font horiz-adv-x="600"><font-face font-family="Test" units-per-em="2048" ascent="1500" descent="400"/>
      <missing-glyph horiz-adv-x="500" d="M0 0L100 0L100 100Z"/>
      <glyph unicode="&#x41;" d="M0 0Q100 200 200 0Z"/>
      <glyph glyph-name="space" unicode=" " horiz-adv-x="250"/>
      </font></defs></svg>"#).unwrap();
    let font = read_svg(&path).unwrap();
    assert_eq!(font.info.units_per_em, 2048);
    assert_eq!(font.default_master().metrics.descender, -400.0);
    assert_eq!(font.glyph(".notdef").unwrap().layers[0].width, 500.0);
    assert_eq!(font.glyph("uni0041").unwrap().layers[0].width, 600.0);
    assert!(font.glyph("space").unwrap().layers[0].contours.is_empty());
}

#[test]
fn svg_rejects_unsupported_untrusted_input() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.svg");
    for body in [
        "<glyph unicode='fi'/>",
        "<glyph unicode='A' transform='scale(2)'/>",
        "<glyph unicode='A' d='M0 0LNaN 0Z'/>",
        "<glyph unicode='A'><path d='M0 0Z'/></glyph>",
        "<hkern u1='A' u2='V' k='10'/>",
        "<glyph unicode='A' arabic-form='initial'/>",
        "<glyph unicode='A' vert-adv-y='1000'/>",
    ] {
        let source = format!(
            "<svg><font horiz-adv-x='600'><font-face units-per-em='1000'/>{body}</font></svg>"
        );
        std::fs::write(&path, source).unwrap();
        assert!(read_svg(&path).is_err(), "accepted {body}");
    }
    for source in [
        "<!DOCTYPE svg [<!ENTITY evil SYSTEM 'file:///etc/passwd'>]><svg>&evil;</svg>",
        "<svg><font horiz-adv-x='600'/><font horiz-adv-x='600'/></svg>",
        "<svg><path d='M0 0Z'/></svg>",
    ] {
        std::fs::write(&path, source).unwrap();
        assert!(read_svg(&path).is_err());
    }
}

#[test]
fn invalid_svg_export_keeps_existing_destination() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("existing.svg");
    std::fs::write(&path, b"preserve me").unwrap();
    let mut font = sample();
    font.glyph_mut("A").unwrap().layers[0].width = f64::INFINITY;
    assert!(write_svg(&font, &font.default_master().id, &path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"preserve me");
    font.glyph_mut("A").unwrap().layers[0].width = 600.0;
    font.glyph_mut("A").unwrap().layers[0].contours[0].closed = false;
    assert!(write_svg(&font, &font.default_master().id, &path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"preserve me");
}

#[test]
fn svg_supported_path_subset_and_strict_lexing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("paths.svg");
    for data in [
        "M0 0 10 10 20 0z",
        "m1 2h30v40l-30-40z",
        "M.5 1e2Q20 50 100 0z",
    ] {
        let source = format!("<svg><font horiz-adv-x='600'><font-face/><glyph glyph-name='test' d='{data}'/></font></svg>");
        std::fs::write(&path, source).unwrap();
        let font = read_svg(&path).unwrap();
        assert!(font.glyph("test").unwrap().layers[0].contours[0].closed);
    }
    for data in [
        "M0 0L1",
        "M0 0Z 1 2",
        "M0 0,",
        "M0 0A1 2 0 0 0 3 4",
        "M0 0L1e999 2",
        "M0 0T1 2",
        "M0 0 L1 2 trailing",
    ] {
        let source =
            format!("<svg><font horiz-adv-x='600'><font-face/><glyph d='{data}'/></font></svg>");
        std::fs::write(&path, source).unwrap();
        assert!(read_svg(&path).is_err(), "accepted {data}");
    }
}

#[test]
fn svg_independent_fonttools_decoder_when_available() {
    let available = std::process::Command::new("python3")
        .args(["-c", "import fontTools.svgLib.path"])
        .output();
    if !available.is_ok_and(|output| output.status.success()) {
        eprintln!("fontTools unavailable; skipping optional independent SVG decoder");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("font.svg");
    let font = sample();
    write_svg(&font, &font.default_master().id, &path).unwrap();
    let script = r#"
import sys, xml.etree.ElementTree as ET
from fontTools.svgLib.path import parse_path
from fontTools.pens.recordingPen import RecordingPen
root = ET.parse(sys.argv[1]).getroot()
glyphs = root.findall('.//{http://www.w3.org/2000/svg}glyph')
assert len(glyphs) == 2
assert [g.attrib['unicode'] for g in glyphs] == ['A', chr(0x1d400)]
pen = RecordingPen()
parse_path(glyphs[0].attrib['d'], pen)
assert pen.value == [('moveTo', ((10.25, 20.5),)), ('curveTo', ((20.0, 100.0), (90.5, 110.25), (190.75, 20.5))), ('lineTo', ((10.25, 20.5),)), ('closePath', ())], pen.value
assert float(glyphs[0].attrib['horiz-adv-x']) == 612.25
"#;
    let output = std::process::Command::new("python3")
        .args(["-c", script])
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn svg_duplicate_conflicts_and_output_budget_fail_without_replacement() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("duplicate.svg");
    std::fs::write(&path, "<svg><font><font-face/><glyph glyph-name='A' unicode='A' d='M0 0L10 0Z'/><glyph glyph-name='A' unicode='B' d='M0 0L20 0Z'/></font></svg>").unwrap();
    assert!(read_svg(&path).is_err());
    let mut font = sample();
    font.glyph_mut("A").unwrap().unicodes = vec![65; 65536];
    std::fs::write(&path, "preserve").unwrap();
    assert!(write_svg(&font, &font.default_master().id, &path).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "preserve");
}
