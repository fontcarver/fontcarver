//! Generates independently inspectable output for the instruction writer.

use std::{collections::BTreeSet, error::Error, fs, io::Write, path::PathBuf};

use fc_model::TtMasterData;
use fc_sfnt::tt::{fingerprints, point_basis, with_instructions, GlyphProgram};
use read_fonts::TableProvider;
use skrifa::MetadataProvider;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: tt_instruction_acceptance INPUT.ttf OUTPUT_DIRECTORY".into());
    }
    let bytes = fs::read(&args[0])?;
    let directory = PathBuf::from(&args[1]);
    fs::create_dir_all(&directory)?;
    let face = skrifa::FontRef::new(&bytes)?;
    let ids: Vec<_> = (0..face.maxp()?.num_glyphs()).collect();
    let mut programs = Vec::new();
    let mut samples: BTreeSet<_> = ids.iter().take(256).copied().collect();
    for character in ['A', 'é', ' '] {
        let glyph_id = face
            .charmap()
            .map(character)
            .ok_or("missing sample character")?
            .to_u32() as u16;
        samples.insert(glyph_id);
        programs.push(GlyphProgram {
            glyph_id,
            fingerprint: point_basis(&bytes, glyph_id)?.fingerprint,
            // Round and move one point by a pixel, or move the advance phantom.
            bytes: vec![
                0x01,
                0xb0,
                u8::from(character == ' '),
                0x2f,
                0xb1,
                u8::from(character == ' '),
                64,
                0x38,
                0x30,
                0x31,
            ],
        });
    }
    let globals = TtMasterData {
        authored_units_per_em: None,
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
    };
    let output = with_instructions(&bytes, &programs, &globals, &[])?;
    if fingerprints(&bytes, &ids)? != fingerprints(&output, &ids)? {
        return Err("point identity changed after writing programs".into());
    }
    fs::write(directory.join("instructed.ttf"), output)?;
    fs::write(
        directory.join("control.ttf"),
        with_instructions(&bytes, &[], &globals, &[])?,
    )?;
    let mut manifest = fs::File::create(directory.join("programs.tsv"))?;
    for program in &programs {
        let hex: String = program.bytes.iter().map(|b| format!("{b:02x}")).collect();
        writeln!(manifest, "{}\t{hex}", program.glyph_id)?;
    }
    let mut points = fs::File::create(directory.join("points.tsv"))?;
    for id in samples {
        for (index, point) in point_basis(&bytes, id)?.points.iter().enumerate() {
            writeln!(
                points,
                "{id}\t{index}\t{}\t{}\t{}\t{}",
                point.position.x,
                point.position.y,
                u8::from(point.on_curve),
                point.contour
            )?;
        }
    }
    eprintln!(
        "Verified {} glyph bindings; wrote {} programs and point samples",
        ids.len(),
        programs.len()
    );
    Ok(())
}
