//! CFF2 private-dictionary authoring and its variation across masters.

mod common;

use fc_model::{Axis, LayerId, LayerKind, MasterId, PsPrivate, Tag};
use read_fonts::{types::Tag as RawTag, FontRef};

/// Two weight masters whose PostScript private values differ.
fn two_master_font(vary: bool) -> fc_model::Font {
    let mut font = common::scratch_font();
    font.axes.push(Axis {
        tag: Tag(*b"wght"),
        name: "Weight".into(),
        min: 100.0,
        default: 100.0,
        max: 900.0,
        ..Default::default()
    });
    font.masters[0].location.insert(Tag(*b"wght"), 100.0);
    font.masters[0].private_dict = PsPrivate {
        blue_values: vec![-12.0, 0.0, 700.0, 712.0],
        std_hw: Some(60.0),
        std_vw: Some(80.0),
        stem_snap_v: vec![80.0, 96.0],
        language_group: Some(0),
        ..PsPrivate::default()
    };
    let mut heavy = font.masters[0].clone();
    heavy.id = MasterId::new();
    heavy.name = "Heavy".into();
    heavy.location.insert(Tag(*b"wght"), 900.0);
    if vary {
        heavy.private_dict = PsPrivate {
            blue_values: vec![-18.0, 0.0, 700.0, 724.0],
            std_hw: Some(110.0),
            std_vw: Some(150.0),
            stem_snap_v: vec![150.0, 180.0],
            language_group: Some(0),
            ..PsPrivate::default()
        };
    }
    for glyph in font.glyphs.values_mut() {
        let mut layer = glyph.layers[0].clone();
        layer.id = LayerId::new();
        layer.kind = LayerKind::Master(heavy.id.clone());
        layer.width += 200.0;
        for contour in &mut layer.contours {
            for point in &mut contour.points {
                point.x *= 1.5;
            }
        }
        glyph.layers.push(layer);
    }
    font.masters.push(heavy);
    font
}

fn export(font: &fc_model::Font) -> fc_compile::ExportReport {
    fc_compile::export(
        font,
        &|_| 0,
        &fc_compile::ExportOptions {
            variable: true,
            format: fc_compile::ExportFormat::Otf,
            ..Default::default()
        },
    )
    .expect("variable CFF2 export")
}

#[test]
fn the_private_dictionary_is_written_and_varies_with_the_masters() {
    for vary in [false, true] {
        let font = two_master_font(vary);
        let report = export(&font);
        let face = FontRef::new(&report.bytes).expect("face");
        assert!(face.table_data(RawTag::new(b"CFF2")).is_some());
        let mut outcome =
            fc_sfnt::read_bytes(&report.bytes, &fc_sfnt::ReadOptions::default()).expect("reimport");
        // The importer discloses a blended private dictionary, so its warning is
        // exactly the signal that the writer emitted DICT `blend` operators.
        assert_eq!(
            outcome
                .warnings
                .iter()
                .any(|warning| warning.code == "cff2-private-variation-flattened"),
            vary,
            "vary {vary}: {:?}",
            outcome.warnings
        );
        let imported = outcome.fonts.remove(0);
        let private = &imported.default_master().private_dict;
        // The default instance keeps the design default master's values.
        assert_eq!(private.blue_values, vec![-12.0, 0.0, 700.0, 712.0]);
        assert_eq!(private.std_hw, Some(60.0));
        assert_eq!(private.std_vw, Some(80.0));
        assert_eq!(private.stem_snap_v, vec![80.0, 96.0]);
        assert_eq!(private.language_group, Some(0));
    }
}

/// The private values a reader computes at one normalized location.
///
/// This is the operand-stack semantics `blend` really has: the relative
/// operands are blended first, and the array operator takes their prefix sum
/// afterwards. read-fonts implements it independently of the writer under test.
fn private_at(bytes: &[u8], coordinate: f32) -> Vec<(String, Vec<f64>)> {
    use read_fonts::{
        ps::cff::{blend::BlendState, dict},
        tables::variations::ItemVariationStore,
        types::F2Dot14,
        FontData, FontRead, FontRef, TableProvider,
    };
    let face = FontRef::new(bytes).expect("face");
    let cff2 = face.cff2().expect("CFF2");
    let data = face
        .table_data(RawTag::new(b"CFF2"))
        .expect("CFF2 table")
        .as_bytes();
    let top: Vec<_> = dict::entries(cff2.top_dict_data(), None)
        .map(|entry| entry.expect("top dict"))
        .collect();
    let store = top
        .iter()
        .find_map(|entry| match entry {
            dict::Entry::VariationStoreOffset(offset) => Some(*offset),
            _ => None,
        })
        .expect("variation store");
    // The CFF2 store is preceded by its own two-byte length.
    let store = ItemVariationStore::read(FontData::new(&data[store + 2..])).expect("store");
    let fd = top
        .iter()
        .find_map(|entry| match entry {
            dict::Entry::FdArrayOffset(offset) => Some(*offset),
            _ => None,
        })
        .expect("FDArray");
    let array = read_fonts::ps::cff::index::Index::new(&data[fd..], true).expect("FDArray index");
    let range = dict::entries(array.get(0).expect("font dict"), None)
        .find_map(|entry| match entry.expect("font dict") {
            dict::Entry::PrivateDictRange(range) => Some(range),
            _ => None,
        })
        .expect("private range");
    let coordinates = [F2Dot14::from_f32(coordinate)];
    let blend = BlendState::new(store, &coordinates, 0).expect("blend state");
    dict::entries(&data[range], Some(blend))
        .map(|entry| match entry.expect("private dict") {
            dict::Entry::BlueValues(values) => (
                "BlueValues".to_owned(),
                values
                    .values()
                    .iter()
                    .flat_map(|(a, b)| [a.to_f64(), b.to_f64()])
                    .collect(),
            ),
            dict::Entry::StemSnapV(values) => (
                "StemSnapV".to_owned(),
                values.values().iter().map(|v| v.to_f64()).collect(),
            ),
            dict::Entry::StdHw(value) => ("StdHW".to_owned(), vec![value.to_f64()]),
            dict::Entry::StdVw(value) => ("StdVW".to_owned(), vec![value.to_f64()]),
            other => (format!("{other:?}"), Vec::new()),
        })
        .filter(|(_, values)| !values.is_empty())
        .collect::<Vec<_>>()
        .into_iter()
        // The DICT emits its arrays before its scalars; compare by name.
        .collect::<std::collections::BTreeMap<_, _>>()
        .into_iter()
        .collect()
}

#[test]
fn the_blended_private_values_reach_each_master_at_its_own_location() {
    let report = export(&two_master_font(true));
    let default = private_at(&report.bytes, 0.0);
    // Blending is defined on the relative operands an array operator stores, so
    // a delta added to the wrong basis would accumulate through the prefix sum
    // and miss the Heavy master's values.
    let heavy = private_at(&report.bytes, 1.0);
    assert_eq!(
        default,
        vec![
            ("BlueValues".to_owned(), vec![-12.0, 0.0, 700.0, 712.0]),
            ("StdHW".to_owned(), vec![60.0]),
            ("StdVW".to_owned(), vec![80.0]),
            ("StemSnapV".to_owned(), vec![80.0, 96.0]),
        ]
    );
    assert_eq!(
        heavy,
        vec![
            ("BlueValues".to_owned(), vec![-18.0, 0.0, 700.0, 724.0]),
            ("StdHW".to_owned(), vec![110.0]),
            ("StdVW".to_owned(), vec![150.0]),
            ("StemSnapV".to_owned(), vec![150.0, 180.0]),
        ],
        "the Heavy master's own private values"
    );
}

#[test]
fn masters_that_declare_different_private_values_are_reported_not_guessed() {
    let mut font = two_master_font(true);
    // One extra stem snap makes the two masters' value lists incompatible.
    let last = font.masters.len() - 1;
    font.masters[last].private_dict.stem_snap_v.push(200.0);
    let report = export(&font);
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.code == "cff2-private-not-varied"),
        "{:?}",
        report.warnings
    );
    let outcome =
        fc_sfnt::read_bytes(&report.bytes, &fc_sfnt::ReadOptions::default()).expect("reimport");
    assert!(!outcome
        .warnings
        .iter()
        .any(|warning| warning.code == "cff2-private-variation-flattened"));
}

#[test]
fn independent_tools_read_the_blended_private_dictionary() {
    use std::process::Command;
    if !Command::new("python3")
        .args(["-c", "import fontTools, freetype"])
        .status()
        .is_ok_and(|status| status.success())
    {
        eprintln!("skipping independent CFF2 private check: fontTools unavailable");
        return;
    }
    // Keep the generated font inside the task temporary directory.
    let directory = std::env::temp_dir().join(format!("fc-cff2-private-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("temporary directory");
    let path = directory.join("variable.otf");
    std::fs::write(&path, export(&two_master_font(true)).bytes).expect("write");
    let script = r#"
import sys, freetype
from fontTools.ttLib import TTFont

font = TTFont(sys.argv[1])
cff = font['CFF2'].cff
private = cff.topDictIndex[0].FDArray[0].Private
# The default-instance values come first in each blend list.
assert [value[0] for value in private.BlueValues] == [-12, 0, 700, 712], private.BlueValues
assert private.StdHW[0] == 60, private.StdHW
assert private.StdVW[0] == 80, private.StdVW
assert [value[0] for value in private.StemSnapV] == [80, 96], private.StemSnapV
# Every blend list carries one delta per region, so the values really vary.
assert all(len(value) > 1 for value in private.BlueValues), private.BlueValues
assert len(private.StdVW) > 1, private.StdVW
assert private.LanguageGroup == 0, private.LanguageGroup


def instance(blends, scalar):
    """The values `blend` yields at one location, on the operand stack.

    A DICT array operator stores each operand relative to the previous one, so
    the deltas are blended into the relative operands and the prefix sum is
    taken afterwards. FontTools reports the default operands as absolutes, so
    they are relativised again here.
    """
    absolutes = [entry[0] if isinstance(entry, list) else entry for entry in blends]
    previous = 0
    result = []
    total = 0
    for value, entry in zip(absolutes, blends):
        relative = value - previous
        previous = value
        deltas = entry[1:] if isinstance(entry, list) else []
        total += relative + sum(scalar * delta for delta in deltas)
        result.append(total)
    return result


def scalar_value(entry, scalar):
    if not isinstance(entry, list):
        return entry
    return entry[0] + sum(scalar * delta for delta in entry[1:])


assert instance(private.BlueValues, 0) == [-12, 0, 700, 712], private.BlueValues
assert instance(private.StemSnapV, 0) == [80, 96], private.StemSnapV
# The Heavy master's own values, reached through the blend operator.
assert instance(private.BlueValues, 1) == [-18, 0, 700, 724], private.BlueValues
assert instance(private.StemSnapV, 1) == [150, 180], private.StemSnapV
assert scalar_value(private.StdHW, 1) == 110, private.StdHW
assert scalar_value(private.StdVW, 1) == 150, private.StdVW

renders = 0
face = freetype.Face(sys.argv[1])
for size in (12, 24, 48):
    face.set_pixel_sizes(0, size)
    for index in range(face.num_glyphs):
        face.load_glyph(index, freetype.FT_LOAD_RENDER)
        renders += 1
print('renders', renders)
"#;
    let file = directory.join("private.py");
    std::fs::write(&file, script).expect("write script");
    let output = Command::new("python3")
        .arg(&file)
        .arg(&path)
        .output()
        .expect("python3");
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let _ = std::fs::remove_dir_all(&directory);
    assert!(stdout.contains("renders "), "{stdout}");
}
