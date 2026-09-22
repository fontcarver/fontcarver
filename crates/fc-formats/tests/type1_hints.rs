use fc_formats::type1::{read_type1, read_warnings, write_type1};
use fc_model::{Contour, Font, Glyph, Layer, PsHints, StemHint};
use kurbo::{Rect, Shape};

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

fn sample() -> Font {
    let mut font = Font::default();
    font.info.units_per_em = 1000;
    font.info.family_name = "Type One Hints".into();
    font.info.style_name = "Regular".into();
    let mut glyph = Glyph::new("H");
    glyph.unicodes.push(72);
    let mut layer = Layer::new_master(font.default_master().id.clone());
    layer.width = 612.25;
    for rect in [
        Rect::new(50.25, 0.0, 130.75, 700.0),
        Rect::new(450.5, 0.0, 532.0, 700.0),
        Rect::new(130.75, 310.5, 450.5, 390.25),
    ] {
        layer
            .contours
            .extend(Contour::from_bezpath(&rect.to_path(0.1)));
    }
    layer.hints = PsHints {
        h_stems: stems(&[(21.0, -21.0), (310.5, 79.75), (700.0, -20.0)]),
        v_stems: stems(&[(50.25, 80.5), (450.5, 81.5)]),
        ..Default::default()
    };
    glyph.layers.push(layer);
    font.insert_glyph(glyph).unwrap();
    font.masters[0].private_dict.blue_values = vec![-15.0, 0.0, 700.0, 715.0, 500.0, 510.0];
    font.masters[0].private_dict.std_hw = Some(79.75);
    font
}

#[test]
fn whole_glyph_fractional_and_ghost_hints_survive_both_containers() {
    let font = sample();
    let dir = tempfile::tempdir().unwrap();
    for binary in [false, true] {
        let path = dir
            .path()
            .join(if binary { "hinted.pfb" } else { "hinted.pfa" });
        write_type1(&font, &font.default_master().id, &path, binary).unwrap();
        let result = read_type1(&path).unwrap();
        assert_eq!(
            result.glyph("H").unwrap().layers[0].hints,
            font.glyph("H").unwrap().layers[0].hints
        );
        assert_eq!(
            result.default_master().private_dict,
            font.default_master().private_dict
        );
        assert!(read_warnings(&result).is_empty());
    }
}

#[test]
fn unsupported_or_overlapping_hints_do_not_replace_output() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("existing.pfa");
    std::fs::write(&path, b"original").unwrap();
    for case in 0..4 {
        let mut font = sample();
        let layer = &mut font.glyph_mut("H").unwrap().layers[0];
        match case {
            0 => layer.hints.h_stems.push(StemHint {
                start: 320.0,
                width: 30.0,
                ..Default::default()
            }),
            1 => layer.hints.flex = true,
            2 => layer.hints.v_stems[0].ranges.push((0.0, 700.0)),
            _ => layer.hints.hint_masks.push(fc_model::HintMask {
                mask: vec![0xf8],
                ..Default::default()
            }),
        }
        assert!(write_type1(&font, &font.default_master().id, &path, false).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"original");
    }
}

// Native test data is constructed directly from the Type 1 format, including
// lenIV=-1 and a nonzero sidebearing. No font editor generates these programs.
fn native(
    dir: &std::path::Path,
    private_fields: &str,
    glyph: &[u8],
    subr: &[u8],
    matrix: &str,
) -> std::path::PathBuf {
    let mut private = format!("TESTdup /Private 20 dict dup begin\n/lenIV -1 def\n{private_fields}\n/Subrs 1 array dup 0 {} RD ", subr.len()).into_bytes();
    private.extend_from_slice(subr);
    private.extend_from_slice(
        format!(
            " NP\n2 index /CharStrings 1 dict dup begin\n/H {} RD ",
            glyph.len()
        )
        .as_bytes(),
    );
    private.extend_from_slice(glyph);
    private.extend_from_slice(b" ND\nend end readonly put noaccess put dup /FontName get exch definefont pop mark currentfile closefile\n");
    let mut state = 55665u16;
    let encrypted: Vec<_> = private
        .iter()
        .map(|byte| {
            let cipher = *byte ^ (state >> 8) as u8;
            state = state
                .wrapping_add(u16::from(cipher))
                .wrapping_mul(52845)
                .wrapping_add(22719);
            cipher
        })
        .collect();
    let clear = format!("%!PS-AdobeFont-1.0: Hints 1.0\n/FontName /Hints def\n/FontType 1 def\n/FontMatrix [{matrix}] def\n/Encoding StandardEncoding def\ncurrentfile eexec\n");
    let mut bytes = Vec::new();
    for (kind, data) in [
        (1, clear.as_bytes()),
        (2, encrypted.as_slice()),
        (1, b"cleartomark\n".as_slice()),
    ] {
        bytes.extend_from_slice(&[0x80, kind]);
        bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
        bytes.extend_from_slice(data);
    }
    bytes.extend_from_slice(&[0x80, 3]);
    let path = dir.join("native.pfb");
    std::fs::write(&path, bytes).unwrap();
    path
}

#[test]
fn subroutine_hints_include_sidebearing_and_font_matrix() {
    let dir = tempfile::tempdir().unwrap();
    // 30 600 hsbw; 0 callsubr; 0 0 rmoveto; 0 100 rlineto; 40 0
    // rlineto; 0 -100 rlineto; closepath; endchar.
    let glyph = [
        169, 248, 236, 13, 139, 10, 139, 139, 21, 139, 239, 5, 179, 139, 5, 139, 39, 5, 9, 14,
    ];
    // 0 40 vstem; 21 -21 hstem; return.
    let subr = [139, 179, 3, 160, 118, 1, 11];
    let path = native(
        dir.path(),
        "/BlueValues [-10 0 100 110] def /StdVW [40] def",
        &glyph,
        &subr,
        "0.002 0 0 0.001 0.010 0.020",
    );
    let result = read_type1(&path).unwrap();
    let layer = &result.glyph("H").unwrap().layers[0];
    assert_eq!(layer.hints.v_stems, stems(&[(70.0, 80.0)]));
    assert_eq!(layer.hints.h_stems, stems(&[(41.0, -21.0)]));
    assert_eq!(
        result.default_master().private_dict.blue_values,
        [10.0, 20.0, 120.0, 130.0]
    );
    assert_eq!(result.default_master().private_dict.std_vw, Some(80.0));
    assert!(read_warnings(&result).is_empty());
}

#[test]
fn late_stems_are_disclosed_and_not_misapplied_to_the_whole_outline() {
    let dir = tempfile::tempdir().unwrap();
    let glyph = [
        139, 248, 236, 13, 139, 179, 3, 139, 139, 21, 139, 239, 5, 149, 159, 3, 179, 139, 5, 139,
        39, 5, 9, 14,
    ];
    let path = native(
        dir.path(),
        "/BlueValues [-10 0] def",
        &glyph,
        &[11],
        "0.001 0 0 0.001 0 0",
    );
    let result = read_type1(&path).unwrap();
    assert!(result.glyph("H").unwrap().layers[0].hints.is_empty());
    assert!(!result.glyph("H").unwrap().layers[0].contours.is_empty());
    assert_eq!(read_warnings(&result)[0].code, "type1-import-hints-partial");
    assert_eq!(
        result.default_master().private_dict.blue_values,
        [-10.0, 0.0]
    );
    assert_eq!(
        result.user_data["org.fontcarver.type1"]["unretained_glyph_hints"],
        serde_json::json!(["H"])
    );
}

#[test]
fn private_reflection_loss_is_disclosed_and_original_values_are_retained() {
    let dir = tempfile::tempdir().unwrap();
    let path = native(
        dir.path(),
        "/BlueValues [-10 0 100 110] def",
        &[139, 248, 236, 13, 14],
        &[11],
        "0.001 0 0 -0.001 0 0",
    );
    let result = read_type1(&path).unwrap();
    assert!(result.default_master().private_dict.blue_values.is_empty());
    assert_eq!(
        read_warnings(&result)[0].code,
        "type1-import-private-transform"
    );
    assert_eq!(
        result.user_data["org.fontcarver.type1"]["original_private"]["blue_values"],
        serde_json::json!([-10.0, 0.0, 100.0, 110.0])
    );
}

#[test]
fn malformed_private_values_are_rejected_and_runtime_values_are_disclosed() {
    let dir = tempfile::tempdir().unwrap();
    for field in [
        "/BlueValues [0 10 5 15] def",
        "/BlueValues [0 10 12] def",
        "/StdHW [72 74] def",
        "/BlueValues [0 nan] def",
        "/ForceBold 1 def",
    ] {
        let path = native(
            dir.path(),
            field,
            &[139, 248, 236, 13, 14],
            &[11],
            "0.001 0 0 0.001 0 0",
        );
        assert!(read_type1(&path).is_err(), "{field}");
    }
    let path = native(
        dir.path(),
        "/BlueScale 0.05 2 mul def /BlueValues [-10 0] def",
        &[139, 248, 236, 13, 14],
        &[11],
        "0.001 0 0 0.001 0 0",
    );
    let result = read_type1(&path).unwrap();
    assert_eq!(result.default_master().private_dict.blue_scale, None);
    assert_eq!(
        result.default_master().private_dict.blue_values,
        [-10.0, 0.0]
    );
    assert_eq!(
        read_warnings(&result)[0].code,
        "type1-import-private-computed"
    );
}

#[test]
fn glyph_names_do_not_override_private_dictionary_parsing() {
    let mut font = sample();
    for name in ["BlueValues", "lenIV", "Metrics", "CharStrings", "ForceBold"] {
        let mut glyph = Glyph::new(name);
        glyph
            .layers
            .push(Layer::new_master(font.default_master().id.clone()));
        font.insert_glyph(glyph).unwrap();
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("names.pfb");
    write_type1(&font, &font.default_master().id, &path, true).unwrap();
    let result = read_type1(&path).unwrap();
    assert!(result.glyph("lenIV").is_some());
    assert_eq!(
        result.default_master().private_dict,
        font.default_master().private_dict
    );
}

#[test]
fn real_type1_hints_preserve_sidebearings_and_unsorted_blue_pairs() {
    let root = fc_fixtures::skip_unless_external!("fontforge-tests");
    let path = root.join("n019003l.pfb");
    if !path.is_file() {
        return;
    }
    let mut font = read_type1(&path).unwrap();
    assert_eq!(
        font.default_master().private_dict.blue_values,
        [-23.0, 0.0, 524.0, 544.0, 729.0, 749.0, 686.0, 709.0]
    );
    assert_eq!(
        font.glyph("H").unwrap().layers[0].hints.v_stems,
        stems(&[(83.0, 93.0), (551.0, 93.0)])
    );
    assert_eq!(
        font.glyph("H").unwrap().layers[0].hints.h_stems,
        stems(&[(21.0, -21.0), (332.0, 82.0), (729.0, -20.0)])
    );
    assert!(read_warnings(&font)
        .iter()
        .any(|warning| warning.code == "type1-import-hints-partial"));
    assert!(
        font.user_data["org.fontcarver.type1"]["unretained_glyph_hints"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("a"))
    );
    font.glyphs.retain(|name, _| name.as_ref() == "H");
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("H.pfb");
    write_type1(&font, &font.default_master().id, &output, true).unwrap();
    let roundtrip = read_type1(&output).unwrap();
    assert_eq!(
        font.glyph("H").unwrap().layers[0].hints,
        roundtrip.glyph("H").unwrap().layers[0].hints
    );
}

#[test]
fn fonttools_decodes_hint_operands_and_freetype_renders_the_output() {
    use std::process::Command;
    if !Command::new("python3")
        .args(["-c", "import fontTools, freetype"])
        .status()
        .is_ok_and(|status| status.success())
    {
        eprintln!("skipping independent hint check: Python fontTools/freetype unavailable");
        return;
    }
    let font = sample();
    let dir = tempfile::tempdir().unwrap();
    let script = r#"
import sys, freetype
from fontTools.t1Lib import T1Font
from fontTools.pens.recordingPen import RecordingPen
font = T1Font(sys.argv[1])
assert font['Private']['BlueValues'] == [-15, 0, 700, 715, 500, 510]
assert font['Private']['StdHW'] == [79.75]
char = font['CharStrings']['H']
char.decompile()
stack, found = [], {'hstem': [], 'vstem': []}
for item in char.program:
    if isinstance(item, (int, float)): stack.append(item)
    elif item == 'div':
        denominator, numerator = stack.pop(), stack.pop()
        stack.append(numerator / denominator)
    elif item in found: found[item].append(tuple(stack)); stack.clear()
    else: stack.clear()
assert found == {'hstem': [(21, -21), (310.5, 79.75), (700, -20)], 'vstem': [(50.25, 80.5), (450.5, 81.5)]}, found
pen = RecordingPen(); char.draw(pen)
assert char.width == 612.25
assert len(pen.value) > 10
face = freetype.Face(sys.argv[1])
changed = False
for size in [9, 12, 16, 24, 36, 48]:
    face.set_pixel_sizes(0, size)
    face.load_char('H', freetype.FT_LOAD_RENDER | freetype.FT_LOAD_NO_AUTOHINT)
    assert face.glyph.bitmap.width > 0 and face.glyph.bitmap.rows > 0
    assert any(face.glyph.bitmap.buffer)
    native = (face.glyph.bitmap.width, face.glyph.bitmap.rows, bytes(face.glyph.bitmap.buffer))
    face.load_char('H', freetype.FT_LOAD_RENDER | freetype.FT_LOAD_NO_HINTING)
    unhinted = (face.glyph.bitmap.width, face.glyph.bitmap.rows, bytes(face.glyph.bitmap.buffer))
    changed |= native != unhinted
assert changed, 'native hints did not affect any tested size'
"#;
    for binary in [false, true] {
        let path = dir.path().join(if binary {
            "independent.pfb"
        } else {
            "independent.pfa"
        });
        write_type1(&font, &font.default_master().id, &path, binary).unwrap();
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
    }
}
