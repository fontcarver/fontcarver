//! Independent decoding and interpreter coverage for Type 1 hint substitution.

use fc_formats::type1::{read_type1, read_warnings, write_type1};
use fc_model::{Contour, Font, Glyph, HintMask, Layer, PsHints, StemHint};
use kurbo::{Rect, Shape};
use std::process::Command;

fn sample() -> Font {
    let mut font = Font::default();
    font.info.units_per_em = 1000;
    font.info.family_name = "Replacement Checks".into();
    font.info.postscript_font_name = Some("ReplacementChecks".into());
    for name in ["H", "C"] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes.push(if name == "H" { 72 } else { 67 });
        let mut layer = Layer::new_master(font.default_master().id.clone());
        layer.width = 600.0;
        for rectangle in [
            Rect::new(50.0, 0.0, 90.0, 700.0),
            Rect::new(450.0, 0.0, 490.0, 700.0),
        ] {
            layer
                .contours
                .extend(Contour::from_bezpath(&rectangle.to_path(0.1)));
        }
        let stem = |start, width| StemHint {
            start,
            width,
            ..Default::default()
        };
        layer.hints = if name == "H" {
            PsHints {
                h_stems: vec![stem(21.0, -21.0), stem(700.0, -20.0)],
                v_stems: vec![stem(50.0, 40.0), stem(55.0, 30.0)],
                hint_masks: [(0, 0, 0xe0), (0, 2, 0xd0), (1, 0, 0xe0), (1, 2, 0xd0)]
                    .into_iter()
                    .map(|(contour, point, byte)| HintMask {
                        contour,
                        point,
                        mask: vec![byte],
                        ..Default::default()
                    })
                    .collect(),
                ..Default::default()
            }
        } else {
            PsHints {
                h_stems: vec![stem(10.0, 40.0), stem(110.0, 40.0), stem(210.0, 40.0)],
                v_stems: vec![stem(50.0, 40.0)],
                hint_masks: vec![HintMask {
                    counter: true,
                    mask: vec![0xe0],
                    ..Default::default()
                }],
                ..Default::default()
            }
        };
        glyph.layers.push(layer);
        font.insert_glyph(glyph).unwrap();
    }
    font
}

#[test]
fn independent_program_decoding_and_native_rendering() {
    if !Command::new("python3")
        .args(["-c", "import fontTools, freetype"])
        .status()
        .is_ok_and(|s| s.success())
    {
        eprintln!("skipping independent replacement check: Python fontTools/freetype unavailable");
        return;
    }
    let font = sample();
    let dir = tempfile::tempdir().unwrap();
    let script = r#"
import sys, freetype
from fontTools.t1Lib import T1Font
from fontTools.pens.recordingPen import RecordingPen
font = T1Font(sys.argv[1])
subrs = font['Private']['Subrs']
assert len(subrs) == 6, len(subrs)
for subr in subrs: subr.decompile()
assert subrs[3].program == ['return']
assert subrs[4].program == [21, -21, 'hstem', 700, -20, 'hstem', 55, 30, 'vstem', 'return']
assert subrs[5].program == [21, -21, 'hstem', 700, -20, 'hstem', 50, 40, 'vstem', 'return']
h = font['CharStrings']['H']; h.decompile()
calls = [h.program[i-3:i+3] for i, token in enumerate(h.program) if token == 'callothersubr']
assert calls == [[n, 1, 3, 'callothersubr', 'pop', 'callsubr'] for n in [4, 5, 4]], calls
c = font['CharStrings']['C']; c.decompile()
pos = c.program.index('hstem3')
assert c.program[pos-6:pos] == [10, 40, 110, 40, 210, 40]
face = freetype.Face(sys.argv[1]); changed = False
for name in ['H', 'C']:
    char = font['CharStrings'][name]; pen = RecordingPen(); char.draw(pen)
    assert char.width == 600
    assert sum(op == 'closePath' for op, args in pen.value) == 2
    for size in [9, 12, 16, 24, 36, 48]:
        face.set_pixel_sizes(0, size)
        face.load_char(name, freetype.FT_LOAD_RENDER | freetype.FT_LOAD_NO_AUTOHINT)
        assert any(face.glyph.bitmap.buffer)
        native = (face.glyph.bitmap.width, face.glyph.bitmap.rows, bytes(face.glyph.bitmap.buffer))
        face.load_char(name, freetype.FT_LOAD_RENDER | freetype.FT_LOAD_NO_HINTING)
        unhinted = (face.glyph.bitmap.width, face.glyph.bitmap.rows, bytes(face.glyph.bitmap.buffer))
        changed |= native != unhinted
assert changed, 'native Type 1 hints did not affect tested sizes'
"#;
    let has_gs = Command::new("gs")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success());
    for binary in [false, true] {
        let path = dir.path().join(if binary {
            "replacement.pfb"
        } else {
            "replacement.pfa"
        });
        write_type1(&font, &font.default_master().id, &path, binary).unwrap();
        let result = read_type1(&path).unwrap();
        assert!(read_warnings(&result).is_empty());
        for name in ["H", "C"] {
            assert_eq!(
                result.glyph(name).unwrap().layers[0].hints,
                font.glyph(name).unwrap().layers[0].hints
            );
        }
        let output = Command::new("python3")
            .args(["-c", script])
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        if has_gs {
            // PFB is a transport container, not directly executable PostScript.
            // FreeType/FontTools above read it natively; feed GS its segment payloads.
            let program = dir.path().join("program.ps");
            let mut bytes = std::fs::read(&path).unwrap();
            if binary {
                let mut input = bytes.as_slice();
                let mut plain = Vec::new();
                while input[1] != 3 {
                    let size = u32::from_le_bytes(input[2..6].try_into().unwrap()) as usize;
                    plain.extend_from_slice(&input[6..6 + size]);
                    input = &input[6 + size..];
                }
                bytes = plain;
            }
            std::fs::write(&program, bytes).unwrap();
            let output = Command::new("gs")
                .args(["-q", "-dBATCH", "-dNOPAUSE", "-sDEVICE=inkcov", "-o", "-", "-f"])
                .arg(&program).args(["-c", "/ReplacementChecks findfont 48 scalefont setfont 10 15 moveto (HC) show showpage"])
                .output().unwrap();
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
                    .filter_map(|s| s.parse::<f64>().ok())
                    .any(|n| n > 0.0)),
                "{ink}"
            );
        } else {
            eprintln!("skipping PostScript interpreter check: Ghostscript unavailable");
        }
    }
}

#[test]
fn invalid_active_sets_and_counter_groups_preserve_destination() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("existing.pfb");
    std::fs::write(&path, b"original").unwrap();
    // Unequal outer widths and unequal centre spacing are no longer failures:
    // they are written as general counter control, which has its own test below.
    for case in [0, 3, 4, 5, 6] {
        let mut font = sample();
        let name = if case == 0 || case == 5 { "H" } else { "C" };
        let hints = &mut font.glyph_mut(name).unwrap().layers[0].hints;
        match case {
            0 => hints.hint_masks[1].mask[0] = 0xf0,
            3 => hints.hint_masks[0].point = 2,
            5 => hints.h_stems[0].width = 1e-12,
            // One stem alone delimits no counter.
            4 => hints.hint_masks.push(HintMask {
                mask: vec![0x90],
                ..Default::default()
            }),
            // Overlapping stems inside one counter group.
            _ => {
                hints.h_stems[1].start = 30.0;
                hints.hint_masks[0].mask[0] = 0xe0;
            }
        }
        assert!(
            write_type1(&font, &font.default_master().id, &path, true).is_err(),
            "case {case}"
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"original");
    }
}

#[test]
fn general_counter_groups_use_othersubrs_12_and_13_and_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let mut font = sample();
    font.glyphs.retain(|name, _| name.as_ref() == "C");
    {
        let hints = &mut font.glyph_mut("C").unwrap().layers[0].hints;
        // Unequal widths and spacing: no hstem3 triple can describe this.
        hints.h_stems[2].width = 41.0;
        hints.h_stems[1].start = 115.0;
        hints.v_stems.push(StemHint {
            start: 300.0,
            width: 40.0,
            ..Default::default()
        });
        // A second group on the other axis exercises the `#V` section.
        hints.hint_masks.push(HintMask {
            counter: true,
            mask: vec![0x18],
            ..Default::default()
        });
    }
    let original = font.glyph("C").unwrap().layers[0].hints.clone();
    for binary in [false, true] {
        let path = dir
            .path()
            .join(if binary { "general.pfb" } else { "general.pfa" });
        write_type1(&font, &font.default_master().id, &path, binary).unwrap();
        let result = read_type1(&path).unwrap();
        assert!(
            read_warnings(&result).is_empty(),
            "{:?}",
            read_warnings(&result)
        );
        let hints = &result.glyph("C").unwrap().layers[0].hints;
        assert_eq!(hints.h_stems, original.h_stems);
        assert_eq!(hints.v_stems, original.v_stems);
        assert_eq!(
            hints
                .hint_masks
                .iter()
                .map(|mask| (mask.counter, mask.mask.clone()))
                .collect::<Vec<_>>(),
            [(true, vec![0xe0]), (true, vec![0x18])]
        );
    }
    if !Command::new("python3")
        .args(["-c", "import fontTools"])
        .status()
        .is_ok_and(|s| s.success())
    {
        eprintln!("skipping independent counter-control check: fontTools unavailable");
        return;
    }
    let script = r#"
import sys
from fontTools.t1Lib import T1Font
font = T1Font(sys.argv[1])
c = font['CharStrings']['C']; c.decompile()
calls = []
for index, token in enumerate(c.program):
    if token == 'callothersubr':
        calls.append(c.program[:index + 1])
assert len(calls) == 1, calls
program = calls[0]
# hsbw, then the data, the argument count and the OtherSubrs number.
assert program[2] == 'hsbw', program[:4]
assert program[-2] == 13 and program[-3] == len(program[3:-3]), program
data = program[3:-3]
# #H = 1, one group of three stems, #V = 1, one group of two stems.
assert data == [1, 10, 40, 65, 40, 96, -41, 1, 50, 40, 250, -40], data
print('ok')
"#;
    let file = dir.path().join("counters.py");
    std::fs::write(&file, script).unwrap();
    let output = Command::new("python3")
        .arg(&file)
        .arg(dir.path().join("general.pfa"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}
{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn quadratic_implied_points_keep_masks_on_the_intended_segments() {
    use fc_model::{Point, PointKind};
    let mut font = sample();
    font.glyphs.retain(|name, _| name.as_ref() == "H");
    let layer = &mut font.glyph_mut("H").unwrap().layers[0];
    let point = |x, y, kind| Point {
        x,
        y,
        kind,
        ..Default::default()
    };
    layer.contours = vec![Contour {
        closed: true,
        points: vec![
            point(0.0, 0.0, PointKind::QCurve),
            point(0.0, 100.0, PointKind::OffCurve),
            point(100.0, 100.0, PointKind::OffCurve),
            point(100.0, 0.0, PointKind::QCurve),
            point(50.0, -100.0, PointKind::OffCurve),
        ],
        ..Default::default()
    }];
    layer.hints.h_stems.clear();
    layer.hints.hint_masks = vec![
        HintMask {
            mask: vec![0x80],
            ..Default::default()
        },
        HintMask {
            point: 3,
            mask: vec![0x40],
            ..Default::default()
        },
        HintMask {
            closing: true,
            mask: vec![0x80],
            ..Default::default()
        },
    ];
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("quadratic.pfb");
    write_type1(&font, &font.default_master().id, &output, true).unwrap();
    let result = read_type1(&output).unwrap();
    let actual = &result.glyph("H").unwrap().layers[0];
    assert!(read_warnings(&result).is_empty());
    assert_eq!(actual.contours[0].points.len(), 9);
    assert_eq!(
        (
            actual.contours[0].points[6].x,
            actual.contours[0].points[6].y
        ),
        (100.0, 0.0)
    );
    assert_eq!(
        actual.hints.hint_masks,
        vec![
            HintMask {
                mask: vec![0x80],
                ..Default::default()
            },
            HintMask {
                point: 6,
                mask: vec![0x40],
                ..Default::default()
            },
            HintMask {
                closing: true,
                mask: vec![0x80],
                ..Default::default()
            },
        ]
    );
}
