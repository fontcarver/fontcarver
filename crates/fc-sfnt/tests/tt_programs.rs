#[path = "support/tt.rs"]
mod support;

use fc_model::TtMasterData;
use fc_sfnt::tt::{point_basis, with_instructions, GlyphProgram};
use read_fonts::{
    tables::glyf::Glyph,
    types::{GlyphId, Tag},
    FontRef, TableProvider,
};
use support::{composite, font, simple};

fn globals() -> TtMasterData {
    TtMasterData {
        cvt: vec![40, -20, 90],
        fpgm: vec![0xb0, 0, 0x2c, 0x00, 0x2d],
        prep: vec![0xb0, 0, 0x2b],
        max_stack_elements: Some(32),
        max_storage: Some(8),
        max_function_defs: Some(1),
        max_instruction_defs: Some(0),
        max_twilight_points: Some(2),
        max_zones: Some(2),
        max_size_of_instructions: None,
    }
}

fn bound(bytes: &[u8], gid: u16, program: &[u8]) -> GlyphProgram {
    GlyphProgram {
        glyph_id: gid,
        fingerprint: point_basis(bytes, gid).unwrap().fingerprint,
        bytes: program.to_vec(),
    }
}

fn program(bytes: &[u8], gid: u16) -> Vec<u8> {
    let face = FontRef::new(bytes).unwrap();
    let loca = face
        .loca(face.head().unwrap().index_to_loc_format() == 1)
        .unwrap();
    match loca
        .get_glyf(GlyphId::new(u32::from(gid)), &face.glyf().unwrap())
        .unwrap()
    {
        Some(Glyph::Simple(g)) => g.instructions().to_vec(),
        Some(Glyph::Composite(g)) => g.instructions().unwrap_or_default().to_vec(),
        None => Vec::new(),
    }
}

#[test]
fn attaches_exact_programs_to_simple_composite_and_empty_glyphs() {
    let child = simple(&[(10, 20, true), (30, 50, false)], &[0x00]);
    let parent = composite(&[(2, 0, 0, 0, &[])]);
    let bytes = font(&[child, parent, vec![], vec![0; 10]]);
    let programs = [
        bound(&bytes, 0, &[0x00, 0xb0, 0, 0x2f]),
        bound(&bytes, 1, &[0x01, 0xb0, 0, 0x2f]),
        bound(&bytes, 2, &[0x01, 0xb0, 1, 0x2f]),
        bound(&bytes, 3, &[0x00]),
    ];
    let result = with_instructions(&bytes, &programs, &globals()).unwrap();
    for entry in &programs {
        assert_eq!(program(&result, entry.glyph_id), entry.bytes);
        assert_eq!(
            point_basis(&bytes, entry.glyph_id).unwrap(),
            point_basis(&result, entry.glyph_id).unwrap()
        );
    }
    let face = FontRef::new(&result).unwrap();
    assert_eq!(
        face.cvt()
            .unwrap()
            .iter()
            .map(|v| v.get())
            .collect::<Vec<_>>(),
        globals().cvt
    );
    assert_eq!(
        face.table_data(Tag::new(b"fpgm")).unwrap().as_bytes(),
        globals().fpgm
    );
    assert_eq!(
        face.table_data(Tag::new(b"prep")).unwrap().as_bytes(),
        globals().prep
    );
    let maxp = face.maxp().unwrap();
    assert_eq!(maxp.max_zones(), Some(2));
    assert_eq!(maxp.max_storage(), Some(8));
    assert_eq!(maxp.max_stack_elements(), Some(32));
    assert_eq!(maxp.max_size_of_instructions(), Some(4));
}

#[test]
fn replacement_preserves_wrapped_coordinate_encoding_and_removes_unlisted_programs() {
    // A valid wrapping i16 delta cannot safely be regenerated with ordinary i16 subtraction.
    let child = simple(&[(-30000, 0, true), (30000, 0, true)], &[0x00]);
    let mut parent = composite(&[(2 | 0x100, 0, 0, 0, &[]), (2, 0, 0, 0, &[])]);
    parent.extend([0, 1, 0x00]); // The instruction flag is valid on an earlier component.
    let bytes = font(&[child, parent]);
    let entry = bound(&bytes, 1, &[0xb0, 0, 0x2f]);
    let result = with_instructions(&bytes, &[entry], &globals()).unwrap();
    assert_eq!(program(&result, 0), Vec::<u8>::new());
    assert_eq!(program(&result, 1), vec![0xb0, 0, 0x2f]);
    assert_eq!(
        point_basis(&bytes, 0).unwrap(),
        point_basis(&result, 0).unwrap()
    );
    assert_eq!(
        point_basis(&bytes, 1).unwrap(),
        point_basis(&result, 1).unwrap()
    );
}

#[test]
fn stale_duplicate_out_of_range_and_malformed_programs_are_rejected() {
    let bytes = font(&[simple(&[(10, 20, true)], &[])]);
    let entry = bound(&bytes, 0, &[0]);
    let other = font(&[simple(&[(11, 20, true)], &[])]);
    assert!(
        with_instructions(&other, std::slice::from_ref(&entry), &globals())
            .unwrap_err()
            .to_string()
            .contains("stale")
    );
    assert!(with_instructions(&bytes, &[entry.clone(), entry.clone()], &globals()).is_err());
    let mut invalid = entry.clone();
    invalid.glyph_id = 1;
    assert!(with_instructions(&bytes, &[invalid], &globals()).is_err());
    for data in [vec![0xb0], vec![0x41, 1, 0], vec![0; 65536]] {
        let mut invalid = entry.clone();
        invalid.bytes = data;
        assert!(with_instructions(&bytes, &[invalid], &globals()).is_err());
    }
}

#[test]
fn globals_require_complete_valid_maxima_and_complete_programs() {
    let bytes = font(&[simple(&[], &[])]);
    for data in [
        TtMasterData {
            max_zones: Some(0),
            ..globals()
        },
        TtMasterData {
            max_zones: Some(3),
            ..globals()
        },
        TtMasterData {
            max_stack_elements: None,
            ..globals()
        },
        TtMasterData {
            cvt: vec![0; 65537],
            ..globals()
        },
        TtMasterData {
            prep: vec![0xb8],
            ..globals()
        },
        TtMasterData {
            fpgm: vec![0x40, 2, 0],
            ..globals()
        },
        TtMasterData {
            max_instruction_defs: Some(257),
            ..globals()
        },
    ] {
        assert!(with_instructions(&bytes, &[], &data).is_err());
    }
    let mut signed = bytes;
    for tag in [b"DSIG", b"hdmx", b"LTSH", b"VDMX"] {
        signed = support::replace(&signed, Tag::new(tag), vec![0; 8]);
    }
    let result = with_instructions(&signed, &[], &globals()).unwrap();
    for tag in [b"DSIG", b"hdmx", b"LTSH", b"VDMX"] {
        assert!(FontRef::new(&result)
            .unwrap()
            .table_data(Tag::new(tag))
            .is_none());
    }
}

#[test]
fn interpreter_executes_phantom_only_program_and_changes_empty_glyph_advance() {
    use skrifa::{
        instance::{LocationRef, Size},
        outline::{pen::NullPen, DrawSettings, Engine, HintingInstance, HintingOptions, Target},
        MetadataProvider,
    };
    let bytes = font(&[simple(&[], &[])]);
    let instruction = bound(&bytes, 0, &[0x01, 0xb0, 1, 0x2f, 0xb1, 1, 64, 0x38]);
    let with = with_instructions(&bytes, &[instruction], &globals()).unwrap();
    let without = with_instructions(&bytes, &[], &globals()).unwrap();
    let advance = |data: &[u8]| {
        let face = skrifa::FontRef::new(data).unwrap();
        let outlines = face.outline_glyphs();
        let hint = HintingInstance::new(
            &outlines,
            Size::new(16.0),
            LocationRef::default(),
            HintingOptions {
                engine: Engine::Interpreter,
                target: Target::Mono,
            },
        )
        .unwrap();
        outlines
            .get(GlyphId::new(0))
            .unwrap()
            .draw(DrawSettings::hinted(&hint, true), &mut NullPen)
            .unwrap()
            .advance_width
            .unwrap()
    };
    assert_eq!(advance(&with) - advance(&without), 1.0);
}
