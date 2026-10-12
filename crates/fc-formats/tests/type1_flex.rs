//! Type 1 flex hints and dot sections, verified independently.

use fc_formats::type1::{read_type1, read_warnings, write_type1};
use fc_model::{Contour, FlexHint, Font, Glyph, Layer, Point, PointKind, PsHints};
use std::process::Command;

fn point(x: f64, y: f64, kind: PointKind) -> Point {
    Point {
        x,
        y,
        kind,
        ..Point::default()
    }
}

/// A cupped serif drawn exactly like figure 8e of the Type 1 specification,
/// translated so the two outer endpoints sit at y = 690 and the joint at 700.
fn flex_contour() -> Contour {
    Contour {
        closed: true,
        points: vec![
            point(100.0, 690.0, PointKind::Line),
            point(115.0, 690.0, PointKind::OffCurve),
            point(125.0, 700.0, PointKind::OffCurve),
            point(150.0, 700.0, PointKind::Curve),
            point(175.0, 700.0, PointKind::OffCurve),
            point(185.0, 690.0, PointKind::OffCurve),
            point(200.0, 690.0, PointKind::Curve),
            point(200.0, 0.0, PointKind::Line),
            point(100.0, 0.0, PointKind::Line),
        ],
        name: None,
        id: None,
    }
}

fn dot_contour() -> Contour {
    Contour {
        closed: true,
        points: vec![
            point(300.0, 600.0, PointKind::Line),
            point(360.0, 600.0, PointKind::Line),
            point(360.0, 660.0, PointKind::Line),
            point(300.0, 660.0, PointKind::Line),
        ],
        name: None,
        id: None,
    }
}

fn sample(flex: bool, dot: bool) -> Font {
    let mut font = Font::default();
    font.info.units_per_em = 1000;
    font.info.family_name = "Flex Checks".into();
    font.info.postscript_font_name = Some("FlexChecks".into());
    let mut glyph = Glyph::new("i");
    glyph.unicodes.push(105);
    let mut layer = Layer::new_master(font.default_master().id.clone());
    layer.width = 400.0;
    layer.contours = vec![flex_contour(), dot_contour()];
    layer.hints = PsHints {
        flex_hints: if flex {
            vec![FlexHint {
                contour: 0,
                point: 0,
                depth: 50.0,
            }]
        } else {
            Vec::new()
        },
        dot_sections: if dot { vec![1] } else { Vec::new() },
        ..PsHints::default()
    };
    layer
        .hints
        .validate(&layer.contours)
        .expect("the fixture must be a valid hint set");
    glyph.layers.push(layer);
    font.insert_glyph(glyph).unwrap();
    font
}

#[test]
fn flex_and_dot_sections_round_trip_through_both_containers() {
    let dir = tempfile::tempdir().unwrap();
    let font = sample(true, true);
    let original = font.glyph("i").unwrap().layers[0].clone();
    for binary in [false, true] {
        let path = dir
            .path()
            .join(if binary { "flex.pfb" } else { "flex.pfa" });
        write_type1(&font, &font.default_master().id, &path, binary).unwrap();
        let result = read_type1(&path).unwrap();
        assert!(
            read_warnings(&result).is_empty(),
            "{:?}",
            read_warnings(&result)
        );
        let layer = &result.glyph("i").unwrap().layers[0];
        assert_eq!(layer.hints.flex_hints, original.hints.flex_hints);
        assert_eq!(layer.hints.dot_sections, original.hints.dot_sections);
        assert_eq!(layer.contours, original.contours);
    }
}

#[test]
fn a_flex_whose_endpoints_are_not_aligned_is_refused_before_writing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("existing.pfb");
    std::fs::write(&path, b"original").unwrap();
    let mut font = sample(true, false);
    // Move the end point off the start's y: section 8.3 has no reference point.
    font.glyph_mut("i").unwrap().layers[0].contours[0].points[6].y = 680.0;
    let error = write_type1(&font, &font.default_master().id, &path, true).unwrap_err();
    assert!(error.to_string().contains("share an x or y"), "{error}");
    assert_eq!(std::fs::read(&path).unwrap(), b"original");
    // A flex deeper than 20 units is outside the mechanism altogether.
    let mut font = sample(true, false);
    font.glyph_mut("i").unwrap().layers[0].contours[0].points[3].y = 730.0;
    assert!(write_type1(&font, &font.default_master().id, &path, true).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"original");
}

#[test]
fn independent_tools_decode_the_flex_sequence_and_render_the_same_outline() {
    if !Command::new("python3")
        .args(["-c", "import fontTools, freetype"])
        .status()
        .is_ok_and(|status| status.success())
    {
        eprintln!("skipping independent flex check: fontTools/freetype unavailable");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let flexed = dir.path().join("flexed.pfa");
    let plain = dir.path().join("plain.pfa");
    let font = sample(true, true);
    write_type1(&font, &font.default_master().id, &flexed, false).unwrap();
    let plain_font = sample(false, false);
    write_type1(&plain_font, &plain_font.default_master().id, &plain, false).unwrap();
    let script = r#"
import sys, freetype
from fontTools.t1Lib import T1Font
from fontTools.pens.recordingPen import RecordingPen

flexed, plain = sys.argv[1], sys.argv[2]
font = T1Font(flexed)
subrs = font['Private']['Subrs']
assert len(subrs) >= 4, len(subrs)
for subr in subrs:
    subr.decompile()
assert subrs[0].program == [3, 0, 'callothersubr', 'pop', 'pop', 'setcurrentpoint', 'return']
assert subrs[1].program == [0, 1, 'callothersubr', 'return']
assert subrs[2].program == [0, 2, 'callothersubr', 'return']
glyph = font['CharStrings']['i']
glyph.decompile()
program = glyph.program
start = program.index('callsubr') - 1
assert program[start:start + 42] == [
    1, 'callsubr',
    50, 0, 'rmoveto', 2, 'callsubr',
    -35, 0, 'rmoveto', 2, 'callsubr',
    10, 10, 'rmoveto', 2, 'callsubr',
    25, 0, 'rmoveto', 2, 'callsubr',
    25, 0, 'rmoveto', 2, 'callsubr',
    10, -10, 'rmoveto', 2, 'callsubr',
    15, 0, 'rmoveto', 2, 'callsubr',
    50, 200, 690, 0, 'callsubr',
], program[start:start + 42]
assert program.count('dotsection') == 2, program
# The flex form must describe the same outline as the plain curves.
def outline(path):
    source = T1Font(path)
    char = source['CharStrings']['i']
    pen = RecordingPen()
    char.draw(pen)
    return pen.value
assert outline(flexed) == outline(plain), (outline(flexed), outline(plain))

renders = 0
for path in (flexed, plain):
    face = freetype.Face(path)
    for size in (9, 12, 16, 24, 36, 48):
        face.set_pixel_sizes(0, size)
        face.load_char('i', freetype.FT_LOAD_RENDER | freetype.FT_LOAD_NO_AUTOHINT)
        assert face.glyph.bitmap.width > 0
        renders += 1
print('renders', renders)
"#;
    let file = dir.path().join("flex.py");
    std::fs::write(&file, script).unwrap();
    let output = Command::new("python3")
        .arg(&file)
        .arg(&flexed)
        .arg(&plain)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("renders 12"),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    if !Command::new("gs")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
    {
        eprintln!("skipping PostScript interpreter check: Ghostscript unavailable");
        return;
    }
    let output = Command::new("gs")
        .args([
            "-q",
            "-dBATCH",
            "-dNOPAUSE",
            "-sDEVICE=inkcov",
            "-o",
            "-",
            "-f",
        ])
        .arg(&flexed)
        .args([
            "-c",
            "/FlexChecks findfont 48 scalefont setfont 10 15 moveto (i) show showpage",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let ink = String::from_utf8_lossy(&output.stdout);
    assert!(
        ink.lines().any(|line| line
            .split_whitespace()
            .take(4)
            .filter_map(|value| value.parse::<f64>().ok())
            .any(|value| value > 0.0)),
        "{ink}"
    );
}

#[test]
fn a_mask_bound_to_the_flex_joint_round_trips_with_the_flex() {
    let dir = tempfile::tempdir().unwrap();
    let mut font = sample(true, false);
    let master = font.default_master().id.clone();
    let layer = font
        .glyph_mut("i")
        .unwrap()
        .master_layer_mut(&master)
        .unwrap();
    layer.hints.v_stems = vec![
        fc_model::StemHint {
            start: 100.0,
            width: 40.0,
            ranges: Vec::new(),
        },
        fc_model::StemHint {
            start: 160.0,
            width: 40.0,
            ranges: Vec::new(),
        },
    ];
    // Point 3 is the joint of the two curve segments the flex covers.
    layer.hints.hint_masks = vec![fc_model::HintMask {
        counter: false,
        closing: false,
        contour: 0,
        point: 3,
        mask: vec![0x40],
    }];
    layer
        .hints
        .validate(&layer.contours)
        .expect("a joint mask is representable");
    let expected = layer.hints.clone();
    let path = dir.path().join("joint.pfa");
    write_type1(&font, &master, &path, false).unwrap();
    let result = read_type1(&path).unwrap();
    assert!(
        read_warnings(&result).is_empty(),
        "{:?}",
        read_warnings(&result)
    );
    let hints = result.glyph("i").unwrap().layers[0].hints.clone();
    assert_eq!(hints.flex_hints, expected.flex_hints);
    assert_eq!(hints.v_stems, expected.v_stems);
    // The importer makes the initial active set explicit, then the joint's mask.
    assert_eq!(
        hints
            .hint_masks
            .iter()
            .map(|mask| (mask.point, mask.mask.clone()))
            .collect::<Vec<_>>(),
        [(0, vec![0xc0]), (3, vec![0x40])]
    );
    // Writing what was read gives the same hints again.
    let again = dir.path().join("joint-again.pfa");
    let mut round = result.clone();
    write_type1(&round, &round.default_master().id.clone(), &again, false).unwrap();
    round = read_type1(&again).unwrap();
    assert_eq!(round.glyph("i").unwrap().layers[0].hints, hints);

    // A mask bound to the pair's end point cannot be activated inside the one
    // operator that draws both curves, so it is refused before any output.
    let existing = dir.path().join("existing.pfa");
    std::fs::write(&existing, b"original").unwrap();
    font.glyph_mut("i")
        .unwrap()
        .master_layer_mut(&master)
        .unwrap()
        .hints
        .hint_masks[0]
        .point = 6;
    assert!(write_type1(&font, &master, &existing, false).is_err());
    assert_eq!(std::fs::read(&existing).unwrap(), b"original");
}
