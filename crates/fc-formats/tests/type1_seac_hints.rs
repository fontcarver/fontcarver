//! Independently encoded Type 1 composition fixtures; no font editor is used.

use fc_formats::type1::{read_type1, read_warnings, write_type1};
use fc_model::{HintMask, StemHint};

fn op(program: &mut Vec<u8>, operands: &[i32], opcode: u16) {
    for &value in operands {
        match value {
            -107..=107 => program.push((value + 139) as u8),
            108..=1131 => {
                program.extend([
                    247 + ((value - 108) / 256) as u8,
                    ((value - 108) % 256) as u8,
                ]);
            }
            -1131..=-108 => {
                let value = -value - 108;
                program.extend([251 + (value / 256) as u8, (value % 256) as u8]);
            }
            _ => {
                program.push(255);
                program.extend(value.to_be_bytes());
            }
        }
    }
    if opcode > 255 {
        program.extend([12, opcode as u8]);
    } else {
        program.push(opcode as u8);
    }
}

fn rectangle(sidebearing: i32, stem: Option<i32>, ghost: bool) -> Vec<u8> {
    let mut program = Vec::new();
    op(&mut program, &[sidebearing, 600], 13);
    if let Some(width) = stem {
        op(&mut program, &[0, width], 3);
    }
    if ghost {
        op(&mut program, &[21, -21], 1);
    }
    op(&mut program, &[0, 0], 21);
    op(&mut program, &[0, 100], 5);
    op(&mut program, &[40, 0], 5);
    op(&mut program, &[0, -100], 5);
    op(&mut program, &[], 9);
    op(&mut program, &[], 14);
    program
}

fn composite(asb: i32) -> Vec<u8> {
    let mut program = Vec::new();
    op(&mut program, &[50, 600], 13);
    op(&mut program, &[asb, 0, 200, 65, 194], 0x0c06);
    program
}

fn encrypted(program: &[u8], mut state: u16) -> Vec<u8> {
    program
        .iter()
        .map(|byte| {
            let cipher = *byte ^ (state >> 8) as u8;
            state = state
                .wrapping_add(u16::from(cipher))
                .wrapping_mul(52845)
                .wrapping_add(22719);
            cipher
        })
        .collect()
}

fn charstring(program: &[u8]) -> Vec<u8> {
    // Use the ordinary four-byte prefix so the independent FontTools decoder
    // can inspect this fixture; its T1Font parser rejects legal lenIV = -1.
    let mut plaintext = vec![0, 1, 2, 3];
    plaintext.extend(program);
    encrypted(&plaintext, 4330)
}

fn native(
    dir: &std::path::Path,
    base: &[u8],
    accent: &[u8],
    composite: &[u8],
    subrs: &[Vec<u8>],
    matrix: &str,
) -> std::path::PathBuf {
    let mut private = format!(
        "TESTdup /Private 20 dict dup begin\n/RD {{string currentfile exch readstring pop}} executeonly def\n/ND {{noaccess def}} executeonly def\n/NP {{noaccess put}} executeonly def\n/lenIV 4 def\n/BlueValues [-10 0 100 110] def\n/Subrs {} array\n",
        subrs.len()
    ).into_bytes();
    for (index, program) in subrs.iter().enumerate() {
        let program = charstring(program);
        private.extend(format!("dup {index} {} RD ", program.len()).bytes());
        private.extend(&program);
        private.extend(b" NP\n");
    }
    private.extend(b" ND\n2 index /CharStrings 4 dict dup begin\n");
    for (name, program) in [
        (".notdef", &[139, 248, 236, 13, 14][..]),
        ("A", base),
        ("acute", accent),
        ("Aacute", composite),
    ] {
        let program = charstring(program);
        private.extend(format!("/{name} {} RD ", program.len()).bytes());
        private.extend(&program);
        private.extend(b" ND\n");
    }
    private.extend(b"end end readonly put noaccess put dup /FontName get exch definefont pop mark currentfile closefile\n");
    let encrypted = encrypted(&private, 55665);
    let clear = format!("%!PS-AdobeFont-1.0: SeacHints 1.0\n11 dict begin\n/FontName /SeacHints def\n/FontType 1 def\n/PaintType 0 def\n/FontInfo 1 dict dup begin /FamilyName (SeacHints) def end def\n/FontBBox [0 0 600 700] def\n/FontMatrix [{matrix}] def\n/Encoding StandardEncoding def\ncurrentdict end\ncurrentfile eexec\n");
    let mut bytes = Vec::new();
    for (kind, data) in [
        (1, clear.as_bytes()),
        (2, encrypted.as_slice()),
        (1, b"cleartomark\n".as_slice()),
    ] {
        bytes.extend([0x80, kind]);
        bytes.extend((data.len() as u32).to_le_bytes());
        bytes.extend(data);
    }
    bytes.extend([0x80, 3]);
    let path = dir.join("source.pfb");
    std::fs::write(&path, bytes).unwrap();
    path
}

fn stems(values: &[(f64, f64)]) -> Vec<StemHint> {
    values
        .iter()
        .map(|&(start, width)| StemHint {
            start,
            width,
            ..Default::default()
        })
        .collect()
}

#[test]
fn seac_inherits_component_hints_with_sidebearings_ghosts_and_matrix() {
    let dir = tempfile::tempdir().unwrap();
    let source = native(
        dir.path(),
        &rectangle(30, Some(40), true),
        &rectangle(10, Some(20), true),
        &composite(10),
        &[],
        "0.002 0 0 0.001 0.010 0.020",
    );
    let font = read_type1(&source).unwrap();
    let layer = &font.glyph("Aacute").unwrap().layers[0];
    assert!(read_warnings(&font).is_empty());
    assert_eq!(layer.hints.h_stems, stems(&[(41.0, -21.0), (241.0, -21.0)]));
    assert_eq!(layer.hints.v_stems, stems(&[(70.0, 80.0), (110.0, 40.0)]));
    assert_eq!(
        layer.hints.hint_masks,
        vec![
            HintMask {
                mask: vec![0xa0],
                ..Default::default()
            },
            HintMask {
                contour: 1,
                mask: vec![0x50],
                ..Default::default()
            }
        ]
    );
    assert_eq!(layer.contours[0].points[0].x, 70.0);
    assert_eq!(layer.contours[1].points[0].x, 110.0);
    assert_eq!(layer.contours[1].points[0].y, 220.0);
    assert_eq!(layer.width, 1200.0);
    for binary in [false, true] {
        let path = dir
            .path()
            .join(if binary { "result.pfb" } else { "result.pfa" });
        write_type1(&font, &font.default_master().id, &path, binary).unwrap();
        let reopened = read_type1(&path).unwrap();
        let actual = &reopened.glyph("Aacute").unwrap().layers[0];
        assert_eq!(actual.hints, layer.hints);
        assert_eq!(actual.contours, layer.contours);
    }
}

#[test]
fn seac_resets_the_active_set_for_an_unhinted_component() {
    let dir = tempfile::tempdir().unwrap();
    for (base, accent, masks) in [(Some(40), None, [0x80, 0]), (None, Some(20), [0, 0x80])] {
        let path = native(
            dir.path(),
            &rectangle(30, base, false),
            &rectangle(10, accent, false),
            &composite(10),
            &[],
            "0.001 0 0 0.001 0 0",
        );
        let font = read_type1(&path).unwrap();
        let layer = &font.glyph("Aacute").unwrap().layers[0];
        assert!(read_warnings(&font).is_empty());
        assert_eq!(
            layer.hints.hint_masks,
            vec![
                HintMask {
                    mask: vec![masks[0]],
                    ..Default::default()
                },
                HintMask {
                    contour: 1,
                    mask: vec![masks[1]],
                    ..Default::default()
                }
            ]
        );
    }
}

#[test]
fn seac_remaps_replacement_events_and_closing_edges() {
    let dir = tempfile::tempdir().unwrap();
    let mut accent = rectangle(10, Some(20), false);
    accent.truncate(accent.len() - 2);
    op(&mut accent, &[4, 1, 3], 0x0c10);
    op(&mut accent, &[], 0x0c11);
    op(&mut accent, &[], 10);
    op(&mut accent, &[], 9);
    op(&mut accent, &[], 14);
    let mut replacement = Vec::new();
    op(&mut replacement, &[10, 10], 3);
    op(&mut replacement, &[], 11);
    let path = native(
        dir.path(),
        &rectangle(30, Some(40), false),
        &accent,
        &composite(10),
        &[vec![11], vec![11], vec![11], vec![11], replacement],
        "0.001 0 0 0.001 0 0",
    );
    let font = read_type1(&path).unwrap();
    let hints = &font.glyph("Aacute").unwrap().layers[0].hints;
    assert!(read_warnings(&font).is_empty());
    assert_eq!(
        hints.v_stems,
        stems(&[(30.0, 40.0), (50.0, 20.0), (60.0, 10.0)])
    );
    assert_eq!(
        hints.hint_masks,
        vec![
            HintMask {
                mask: vec![0x80],
                ..Default::default()
            },
            HintMask {
                contour: 1,
                mask: vec![0x40],
                ..Default::default()
            },
            HintMask {
                contour: 1,
                closing: true,
                mask: vec![0x20],
                ..Default::default()
            }
        ]
    );
}

#[test]
fn seac_rejects_mismatched_sidebearing_and_component_counter_scope() {
    let dir = tempfile::tempdir().unwrap();
    let mut counter = Vec::new();
    op(&mut counter, &[30, 600], 13);
    op(&mut counter, &[0, 20, 100, 20, 200, 20], 0x0c01);
    counter.extend(&rectangle(30, None, false)[4..]);
    for (base, composite) in [
        (rectangle(30, Some(40), false), composite(11)),
        (counter, composite(10)),
    ] {
        let path = native(
            dir.path(),
            &base,
            &rectangle(10, Some(20), false),
            &composite,
            &[],
            "0.001 0 0 0.001 0 0",
        );
        let font = read_type1(&path).unwrap();
        assert!(!font.glyph("Aacute").unwrap().layers[0].contours.is_empty());
        assert!(font.glyph("Aacute").unwrap().layers[0].hints.is_empty());
        assert!(
            font.user_data["org.fontcarver.type1"]["unretained_glyph_hints"]
                .as_array()
                .unwrap()
                .iter()
                .any(|name| name == "Aacute")
        );
    }
}

#[test]
fn fonttools_and_freetype_verify_expanded_seac_output() {
    use std::process::Command;
    if !Command::new("python3")
        .args(["-c", "import fontTools, freetype"])
        .status()
        .is_ok_and(|status| status.success())
    {
        eprintln!("skipping independent seac check: Python fontTools/freetype unavailable");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let source = native(
        dir.path(),
        &rectangle(30, Some(40), true),
        &rectangle(10, Some(20), true),
        &composite(10),
        &[],
        "0.001 0 0 0.001 0 0",
    );
    let font = read_type1(&source).unwrap();
    assert!(read_warnings(&font).is_empty());
    let script = r#"
import sys, freetype
from fontTools.t1Lib import T1Font
source, output = [T1Font(path) for path in sys.argv[1:]]
original = source['CharStrings']['Aacute']; original.decompile()
assert original.program[-6:] == [10, 0, 200, 65, 194, 'seac']
expanded = output['CharStrings']['Aacute']; expanded.decompile()
assert 'seac' not in expanded.program
found = {'hstem': [], 'vstem': []}
for program in [expanded] + list(output['Private']['Subrs']):
    program.decompile(); stack = []
    for item in program.program:
        if isinstance(item, (int, float)): stack.append(item)
        elif item == 'div':
            denominator, numerator = stack.pop(), stack.pop()
            stack.append(numerator / denominator)
        elif item in found: found[item].append(tuple(stack)); stack.clear()
        else: stack.clear()
assert set(found['hstem']) == {(21, -21), (221, -21)}, found
assert set(found['vstem']) == {(30, 40), (50, 20)}, found
faces = [freetype.Face(path) for path in sys.argv[1:]]
def outline(face, flags):
    face.load_glyph(face.get_name_index(b'Aacute'), flags)
    return (face.glyph.outline.points, face.glyph.outline.tags, face.glyph.outline.contours, face.glyph.advance.x)
flags = freetype.FT_LOAD_NO_SCALE | freetype.FT_LOAD_NO_HINTING
assert outline(faces[0], flags) == outline(faces[1], flags)
changed = False
for size in [9, 12, 16, 24, 36, 48]:
    for face in faces:
        face.set_pixel_sizes(0, size)
        hinted = outline(face, freetype.FT_LOAD_NO_AUTOHINT)
        plain = outline(face, freetype.FT_LOAD_NO_HINTING)
        changed |= hinted != plain
        face.load_glyph(face.get_name_index(b'Aacute'), freetype.FT_LOAD_RENDER | freetype.FT_LOAD_NO_AUTOHINT)
        assert face.glyph.bitmap.width > 0 and any(face.glyph.bitmap.buffer)
assert changed, 'native hinting had no effect at any tested size'
"#;
    for binary in [false, true] {
        let output = dir.path().join(if binary {
            "expanded.pfb"
        } else {
            "expanded.pfa"
        });
        write_type1(&font, &font.default_master().id, &output, binary).unwrap();
        let result = Command::new("python3")
            .args(["-c", script])
            .arg(&source)
            .arg(&output)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
}
