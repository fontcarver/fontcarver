use std::collections::BTreeSet;
use std::fmt::Write;

use read_fonts::{types::GlyphId16, FontRef, TableProvider};

use super::{error, lexer::valid_name, sfnt};
use crate::Error;

pub(super) fn write(bytes: &[u8]) -> Result<Vec<u8>, Error> {
    let face = sfnt::validate(bytes)?;
    if !bytes.len().is_multiple_of(2) {
        return Err(error("Type 42 requires an even TrueType byte length"));
    }
    let ends = sfnt::chunk_ends(&face, bytes.len())?;
    let head = face.head().map_err(|e| error(e.to_string()))?;
    let name = font_name(&face)?;
    let names = glyph_names(&face)?;
    let mut output = String::with_capacity(bytes.len() * 2 + 8192);
    writeln!(output, "%!PS-TrueTypeFont-1.0-{}", head.font_revision())
        .map_err(|e| error(e.to_string()))?;
    output.push_str("%%Creator: FontCarver\n12 dict begin\n");
    writeln!(output, "/FontName /{name} def").map_err(|e| error(e.to_string()))?;
    output.push_str("/FontType 42 def\n/PaintType 0 def\n/FontMatrix [1 0 0 1 0 0] def\n");
    let upem = f64::from(head.units_per_em());
    writeln!(
        output,
        "/FontBBox [{:.8} {:.8} {:.8} {:.8}] def",
        f64::from(head.x_min()) / upem,
        f64::from(head.y_min()) / upem,
        f64::from(head.x_max()) / upem,
        f64::from(head.y_max()) / upem
    )
    .map_err(|e| error(e.to_string()))?;
    output.push_str("/Encoding [\n");
    let cmap = face.cmap().ok();
    let subtable = cmap
        .as_ref()
        .and_then(|cmap| cmap.best_subtable())
        .map(|(_, _, subtable)| subtable);
    for code in 0..256u32 {
        let glyph_name = subtable
            .as_ref()
            .and_then(|cmap| cmap.map_codepoint(code))
            .and_then(|gid| names.get(gid.to_u32() as usize))
            .map(String::as_str)
            .unwrap_or(".notdef");
        write!(
            output,
            "/{glyph_name}{}",
            if code % 8 == 7 { "\n" } else { " " }
        )
        .map_err(|e| error(e.to_string()))?;
    }
    output.push_str("] readonly def\n");
    writeln!(output, "/CharStrings {} dict dup begin", names.len())
        .map_err(|e| error(e.to_string()))?;
    for (gid, glyph_name) in names.iter().enumerate() {
        writeln!(output, "/{glyph_name} {gid} def").map_err(|e| error(e.to_string()))?;
    }
    output.push_str("end readonly def\n/sfnts [\n");
    let mut start = 0;
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for end in ends {
        output.push('<');
        for (offset, &byte) in bytes[start..end].iter().enumerate() {
            if offset.is_multiple_of(32) {
                output.push('\n');
            }
            output.push(char::from(HEX[(byte >> 4) as usize]));
            output.push(char::from(HEX[(byte & 15) as usize]));
        }
        output.push_str("00\n>\n");
        start = end;
    }
    output.push_str("] def\nFontName currentdict end definefont pop\n%%EOF\n");
    Ok(output.into_bytes())
}

fn font_name(face: &FontRef<'_>) -> Result<String, Error> {
    let name = face
        .name()
        .ok()
        .and_then(|table| {
            table
                .name_record()
                .iter()
                .filter(|record| record.name_id().to_u16() == 6)
                .find_map(|record| {
                    record
                        .string(table.string_data())
                        .ok()
                        .map(|value| value.to_string())
                })
        })
        .unwrap_or_else(|| "FontCarver-Type42".into());
    if !valid_name(&name) {
        return Err(error("invalid PostScript font name"));
    }
    Ok(name)
}

fn glyph_names(face: &FontRef<'_>) -> Result<Vec<String>, Error> {
    let count = face.maxp().map_err(|e| error(e.to_string()))?.num_glyphs();
    let post = face.post().ok();
    let mut used: BTreeSet<String> = post
        .as_ref()
        .into_iter()
        .flat_map(|post| {
            (0..count)
                .filter_map(move |gid| post.glyph_name(GlyphId16::new(gid)))
                .filter(|name| valid_name(name))
                .map(str::to_string)
        })
        .collect();
    used.insert(".notdef".into());
    let mut emitted = BTreeSet::new();
    let mut names = Vec::with_capacity(usize::from(count));
    for gid in 0..count {
        let name = if gid == 0 {
            ".notdef".to_string()
        } else if let Some(name) = post
            .as_ref()
            .and_then(|post| post.glyph_name(GlyphId16::new(gid)))
            .filter(|name| valid_name(name) && !emitted.contains(*name))
        {
            name.to_string()
        } else {
            let mut name = format!("gid{gid}");
            while used.contains(&name) {
                name.push('_');
            }
            name
        };
        used.insert(name.clone());
        emitted.insert(name.clone());
        names.push(name);
    }
    Ok(names)
}
