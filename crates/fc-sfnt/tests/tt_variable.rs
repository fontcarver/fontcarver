#[path = "support/tt.rs"]
mod support;

use fc_model::TtMasterData;
use fc_sfnt::tt::{point_basis, with_instructions, GlyphProgram};
use read_fonts::{tables::glyf::Glyph, types::GlyphId, FontRef, TableProvider};
use write_fonts::{
    dump_table,
    tables::{
        cvar::{Cvar, CvtDeltas},
        gvar::{GlyphDelta, GlyphDeltas, GlyphVariations, Gvar, Tent},
    },
    types::{F2Dot14, Tag},
};

fn globals() -> TtMasterData {
    TtMasterData {
        cvt: vec![40, -20],
        max_stack_elements: Some(32),
        max_storage: Some(8),
        max_function_defs: Some(0),
        max_instruction_defs: Some(0),
        max_twilight_points: Some(0),
        max_zones: Some(1),
        ..Default::default()
    }
}

fn fvar() -> Vec<u8> {
    let mut bytes = vec![0, 1, 0, 0, 0, 16, 0, 2, 0, 1, 0, 20, 0, 0, 0, 8];
    bytes.extend(*b"wght");
    for value in [100i32, 400, 900] {
        bytes.extend((value << 16).to_be_bytes());
    }
    bytes.extend([0, 0, 1, 0]);
    bytes
}

fn tent() -> Vec<Tent> {
    vec![Tent::new(F2Dot14::ONE, None)]
}

fn variable() -> Vec<u8> {
    let child = support::simple(&[(10, 20, true), (30, 50, false)], &[0x00]);
    let parent = support::composite(&[(2, 0, 0, 0, &[])]);
    let mut bytes = support::font(&[child.clone(), child, parent]);
    let gvar = Gvar::new(
        (0..3)
            .map(|id| {
                let count = if id == 2 { 5 } else { 6 };
                GlyphVariations::new(
                    GlyphId::new(id),
                    vec![GlyphDeltas::new(
                        tent(),
                        vec![GlyphDelta::new((id + 1) as i16 * 10, 0, true); count],
                    )],
                )
            })
            .collect(),
        1,
    )
    .unwrap();
    let cvar = Cvar::new(vec![CvtDeltas::new(tent(), vec![12, -4])], 1).unwrap();
    for (tag, data) in [
        (*b"fvar", fvar()),
        (*b"gvar", dump_table(&gvar).unwrap()),
        (*b"cvar", dump_table(&cvar).unwrap()),
        (
            *b"cvt ",
            globals().cvt.iter().flat_map(|v| v.to_be_bytes()).collect(),
        ),
    ] {
        bytes = support::replace(&bytes, Tag::new(&tag), data);
    }
    bytes
}

fn bound(bytes: &[u8], glyph_id: u16) -> GlyphProgram {
    GlyphProgram {
        glyph_id,
        fingerprint: point_basis(bytes, glyph_id).unwrap().fingerprint,
        bytes: vec![0x01, 0xb0, 0, 0x2f],
    }
}

fn table(bytes: &[u8], tag: &[u8; 4]) -> Vec<u8> {
    FontRef::new(bytes)
        .unwrap()
        .table_data(Tag::new(tag))
        .unwrap()
        .as_bytes()
        .to_vec()
}

#[test]
fn variable_program_replacement_preserves_points_cvt_and_all_variation_tables() {
    let bytes = variable();
    let programs = [bound(&bytes, 0), bound(&bytes, 2)];
    let result = with_instructions(&bytes, &programs, &globals(), &[]).unwrap();
    for tag in [b"fvar", b"gvar", b"cvar", b"cvt "] {
        assert_eq!(table(&bytes, tag), table(&result, tag));
    }
    let face = FontRef::new(&result).unwrap();
    let loca = face.loca(true).unwrap();
    let glyf = face.glyf().unwrap();
    for id in 0..3 {
        let original = point_basis(&bytes, id).unwrap();
        assert!(original.fingerprint.starts_with("tt-variable-points-v1:"));
        assert_eq!(original, point_basis(&result, id).unwrap());
        let stored = match loca.get_glyf(GlyphId::new(u32::from(id)), &glyf).unwrap() {
            Some(Glyph::Simple(g)) => g.instructions(),
            Some(Glyph::Composite(g)) => g.instructions().unwrap_or_default(),
            None => &[],
        };
        assert_eq!(stored, if id == 1 { &[] } else { &programs[0].bytes[..] });
    }
    let mut deltas = [0; 2];
    face.cvar()
        .unwrap()
        .deltas(1, &[F2Dot14::ONE], &mut deltas)
        .unwrap();
    assert_eq!(deltas, [12 << 16, -4 << 16]);
}

#[test]
fn variable_bindings_include_glyph_id_and_recursive_component_selection() {
    let bytes = variable();
    let first = point_basis(&bytes, 0).unwrap();
    let second = point_basis(&bytes, 1).unwrap();
    assert_eq!(first.points, second.points);
    assert_ne!(first.fingerprint, second.fingerprint);
    let mut glyf = table(&bytes, b"glyf");
    let face = FontRef::new(&bytes).unwrap();
    let parent_start = face.loca(true).unwrap().get_raw(2).unwrap() as usize;
    glyf[parent_start + 12..parent_start + 14].copy_from_slice(&1u16.to_be_bytes());
    let changed = support::replace(&bytes, Tag::new(b"glyf"), glyf);
    let parent = point_basis(&bytes, 2).unwrap();
    let other = point_basis(&changed, 2).unwrap();
    assert_eq!(parent.points, other.points);
    assert_ne!(parent.fingerprint, other.fingerprint);
}

#[test]
fn changing_any_variation_dependency_invalidates_a_program_binding() {
    let bytes = variable();
    let program = bound(&bytes, 0);
    for tag in [
        b"fvar", b"gvar", b"cvar", b"avar", b"HVAR", b"VVAR", b"MVAR",
    ] {
        let mut data = FontRef::new(&bytes)
            .unwrap()
            .table_data(Tag::new(tag))
            .map(|d| d.as_bytes().to_vec())
            .unwrap_or_else(|| vec![0; 32]);
        let last = data.len() - 1;
        data[last] ^= 1;
        let changed = support::replace(&bytes, Tag::new(tag), data);
        let other = point_basis(&changed, 0).unwrap();
        assert_ne!(program.fingerprint, other.fingerprint, "{tag:?}");
        assert!(
            with_instructions(&changed, std::slice::from_ref(&program), &globals(), &[])
                .unwrap_err()
                .to_string()
                .contains("stale")
        );
    }
}

#[test]
fn cvar_requires_the_same_indexed_cvt_before_replacing_instructions() {
    let bytes = variable();
    for cvt in [
        vec![],
        vec![40],
        vec![41, -20],
        vec![-20, 40],
        vec![40, -20, 0],
    ] {
        let data = TtMasterData { cvt, ..globals() };
        assert!(with_instructions(&bytes, &[], &data, &[])
            .unwrap_err()
            .to_string()
            .contains("cvar"));
    }
    let orphan = support::replace(
        &support::font(&[support::simple(&[], &[])]),
        Tag::new(b"cvar"),
        table(&bytes, b"cvar"),
    );
    assert!(with_instructions(&orphan, &[], &globals(), &[])
        .unwrap_err()
        .to_string()
        .contains("cvar"));
}

#[test]
fn malformed_variation_headers_and_orphan_cvt_are_rejected() {
    let bytes = variable();
    for (tag, data) in [
        (*b"fvar", vec![0; 16]),
        (*b"fvar", fvar()[..35].to_vec()),
        (*b"gvar", vec![0; 20]),
        (*b"gvar", table(&bytes, b"gvar")[..23].to_vec()),
        (*b"cvar", vec![0; 8]),
        (*b"cvar", table(&bytes, b"cvar")[..13].to_vec()),
        (*b"cvt ", vec![]),
        (*b"cvt ", vec![0]),
    ] {
        let changed = support::replace(&bytes, Tag::new(&tag), data);
        assert!(point_basis(&changed, 0).is_err(), "{tag:?}");
        assert!(
            with_instructions(&changed, &[], &globals(), &[]).is_err(),
            "{tag:?}"
        );
    }
}

#[test]
fn static_fingerprints_remain_compatible_and_cannot_bind_variable_programs() {
    let static_bytes = support::font(&[support::simple(&[(10, 20, true), (30, 50, false)], &[])]);
    let original = point_basis(&static_bytes, 0).unwrap();
    assert_eq!(
        original.fingerprint,
        "tt-points-v1:7e8449f98ec444bb181bbc5eca6a8529b5e9c1098d5488653e1edad3fe294fb4"
    );
    let variable_bytes = variable();
    let changed = point_basis(&variable_bytes, 0).unwrap();
    assert_eq!(original.points, changed.points);
    assert_ne!(original.fingerprint, changed.fingerprint);
    assert!(
        with_instructions(&variable_bytes, &[bound(&static_bytes, 0)], &globals(), &[])
            .unwrap_err()
            .to_string()
            .contains("stale")
    );
}

#[test]
fn cvar_packed_indices_and_delta_runs_are_checked_without_execution() {
    // One positive-axis tuple with private point numbers: CVT[0] receives +12.
    let cvar = vec![
        0, 1, 0, 0, 0, 1, 0, 14, 0, 5, 0xa0, 0, 0x40, 0, 1, 0, 0, 0, 12,
    ];
    let bytes = variable();
    let valid = support::replace(&bytes, Tag::new(b"cvar"), cvar.clone());
    assert!(point_basis(&valid, 0).is_ok());
    assert!(with_instructions(&valid, &[], &globals(), &[]).is_ok());
    for end in 0..cvar.len() {
        let truncated = support::replace(&bytes, Tag::new(b"cvar"), cvar[..end].to_vec());
        assert!(point_basis(&truncated, 0).is_err(), "cvar length {end}");
    }
    for (at, value) in [(10, 0x20), (14, 3), (15, 1), (16, 2), (17, 1)] {
        let mut invalid = cvar.clone();
        invalid[at] = value;
        let changed = support::replace(&bytes, Tag::new(b"cvar"), invalid);
        assert!(point_basis(&changed, 0).is_err(), "cvar byte {at}");
    }
}

#[test]
fn missing_variation_context_and_inconsistent_gvar_ranges_are_rejected() {
    let bytes = variable();
    for tag in [b"fvar", b"gvar"] {
        let changed = support::replace(&bytes, Tag::new(tag), vec![]);
        assert!(point_basis(&changed, 0).is_err());
    }
    for (at, value) in [(4, 2u16), (12, 4), (14, 2), (22, u16::MAX)] {
        let mut data = table(&bytes, b"gvar");
        data[at..at + 2].copy_from_slice(&value.to_be_bytes());
        let changed = support::replace(&bytes, Tag::new(b"gvar"), data);
        assert!(point_basis(&changed, 0).is_err(), "gvar byte {at}");
    }
    for tag in [b"gvar", b"avar", b"HVAR", b"VVAR", b"MVAR", b"VARC"] {
        let original = support::font(&[support::simple(&[], &[])]);
        let orphan = support::replace(&original, Tag::new(tag), vec![0; 32]);
        assert!(point_basis(&orphan, 0).is_err(), "{tag:?}");
    }
}

#[test]
fn authored_control_value_variation_replaces_the_source_cvar() {
    use read_fonts::{tables::cvar::Cvar, FontData, FontRead};
    let bytes = variable();
    let regions = vec![fc_sfnt::tt::CvtRegion {
        region: vec![(0.0, 1.0, 1.0)],
        deltas: vec![7, -3],
    }];
    let result = with_instructions(&bytes, &[], &globals(), &regions).unwrap();
    assert_ne!(table(&result, b"cvar"), table(&bytes, b"cvar"));
    // Every other variation table and the control values survive unchanged.
    for tag in [b"fvar", b"gvar", b"cvt "] {
        assert_eq!(table(&bytes, tag), table(&result, tag));
    }
    let encoded = table(&result, b"cvar");
    let cvar = Cvar::read(FontData::new(&encoded)).unwrap();
    for (coordinate, expected) in [
        (0.0f32, [0, 0]),
        (1.0, [7 << 16, -3 << 16]),
        (0.5, [7 << 15, -3 << 15]),
    ] {
        let mut deltas = [0i32; 2];
        cvar.deltas(1, &[F2Dot14::from_f32(coordinate)], &mut deltas)
            .unwrap();
        assert_eq!(deltas, expected, "at {coordinate}");
    }
}

#[test]
fn an_intermediate_control_value_region_peaks_inside_the_axis() {
    use read_fonts::{tables::cvar::Cvar, FontData, FontRead};
    let bytes = variable();
    let regions = vec![fc_sfnt::tt::CvtRegion {
        region: vec![(0.0, 0.5, 1.0)],
        deltas: vec![0, 16],
    }];
    let result = with_instructions(&bytes, &[], &globals(), &regions).unwrap();
    let encoded = table(&result, b"cvar");
    let cvar = Cvar::read(FontData::new(&encoded)).unwrap();
    for (coordinate, expected) in [(0.0f32, 0), (0.5, 16 << 16), (1.0, 0)] {
        let mut deltas = [0i32; 2];
        cvar.deltas(1, &[F2Dot14::from_f32(coordinate)], &mut deltas)
            .unwrap();
        assert_eq!(deltas[1], expected, "at {coordinate}");
        assert_eq!(deltas[0], 0, "a zero delta run stays zero at {coordinate}");
    }
}

#[test]
fn control_value_variation_needs_a_variable_font_and_control_values() {
    let static_bytes = support::font(&[support::simple(&[(10, 20, true)], &[])]);
    let regions = vec![fc_sfnt::tt::CvtRegion {
        region: vec![(0.0, 1.0, 1.0)],
        deltas: vec![7, -3],
    }];
    let error = with_instructions(&static_bytes, &[], &globals(), &regions).unwrap_err();
    assert!(error.to_string().contains("variable font"), "{error}");
    let mut empty = globals();
    empty.cvt.clear();
    assert!(with_instructions(&variable(), &[], &empty, &regions).is_err());
    // A delta vector that does not describe every control value is refused.
    let wrong = vec![fc_sfnt::tt::CvtRegion {
        region: vec![(0.0, 1.0, 1.0)],
        deltas: vec![7],
    }];
    assert!(with_instructions(&variable(), &[], &globals(), &wrong).is_err());
    // So is an axis count the font does not declare.
    let extra = vec![fc_sfnt::tt::CvtRegion {
        region: vec![(0.0, 1.0, 1.0), (0.0, 1.0, 1.0)],
        deltas: vec![7, -3],
    }];
    assert!(with_instructions(&variable(), &[], &globals(), &extra).is_err());
}
