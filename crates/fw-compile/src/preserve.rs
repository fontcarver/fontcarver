//! Preservation merge for raw tables and TrueType programs.

use std::collections::HashSet;

use fw_model::{FeaturesSource, Font, GlyphName, MasterId, Tag};
use read_fonts::{types::GlyphId, FontRef, TableProvider};
use sha2::{Digest, Sha256};
use skrifa::{GlyphNameSource, GlyphNames};
use write_fonts::{
    from_obj::ToOwnedTable,
    tables::glyf::{GlyfLocaBuilder, Glyph as WriteGlyph, SimpleGlyph},
    FontBuilder,
};

use crate::{Error, ExportOptions, ExportWarning};

const LAYOUT_TAGS: [Tag; 4] = [Tag(*b"GSUB"), Tag(*b"GPOS"), Tag(*b"GDEF"), Tag(*b"BASE")];
const DSIG: Tag = Tag(*b"DSIG");

/// Merges compiled tables with eligible imported data and TrueType instructions.
pub fn preserve_merge(
    compiled: &[u8],
    font: &Font,
    touched: &HashSet<GlyphName>,
    options: &ExportOptions,
) -> Result<(Vec<u8>, Vec<ExportWarning>), Error> {
    let compiled_face =
        FontRef::new(compiled).map_err(|error| Error::parse("compiled sfnt", error))?;
    let mut warnings = Vec::new();
    let verified_order = provenance_order(font, &mut warnings);
    let mut builder = FontBuilder::new();
    for record in compiled_face.table_directory().table_records() {
        if record.tag().to_be_bytes() == DSIG.0 {
            continue;
        }
        let data = compiled_face
            .table_data(record.tag())
            .ok_or_else(|| Error::parse("compiled table", record.tag()))?;
        builder.add_raw(record.tag(), data.as_bytes());
    }

    if options.preserve_untouched_tables {
        merge_raw_tables(&mut builder, font, verified_order, &mut warnings);
    }

    if options.keep_tt_instructions {
        if touched.is_empty() && verified_order != Some(false) {
            let master = selected_master(font, options)?;
            if has_hinting(font, master) {
                reattach_hinting(&mut builder, &compiled_face, font, master)?;
            }
        } else if !touched.is_empty() || has_hinting_in_font(font, options)? {
            warnings.push(hinting_dropped_warning(touched, verified_order));
        }
    }

    Ok((builder.build(), warnings))
}

fn merge_raw_tables(
    builder: &mut FontBuilder<'_>,
    font: &Font,
    verified_order: Option<bool>,
    warnings: &mut Vec<ExportWarning>,
) {
    let imported_layout = font.features.source_of_truth == FeaturesSource::ImportedTables;
    let mut layout_dropped = false;
    for (tag, data) in &font.extra.raw_tables {
        if *tag == DSIG {
            continue;
        }
        if LAYOUT_TAGS.contains(tag) {
            if imported_layout && verified_order == Some(true) {
                builder.add_raw(read_tag(*tag), data.clone());
            } else if imported_layout {
                layout_dropped = true;
            }
            continue;
        }
        let tag = read_tag(*tag);
        if !builder.contains(tag) {
            builder.add_raw(tag, data.clone());
        }
    }
    if layout_dropped {
        warnings.push(ExportWarning {
            code: "layout-dropped-glyph-order-changed",
            message: "Imported GSUB/GPOS/GDEF/BASE tables were not copied because the original glyph order could not be verified unchanged".to_owned(),
        });
    }
}

fn provenance_order(font: &Font, warnings: &mut Vec<ExportWarning>) -> Option<bool> {
    let source = font.extra.source.as_ref()?;
    let bytes = source.original_bytes.as_deref()?;
    if !source.sha256.is_empty() {
        let actual = format!("{:x}", Sha256::digest(bytes.as_slice()));
        if actual != source.sha256 {
            warnings.push(ExportWarning {
                code: "provenance-hash-mismatch",
                message: "Original font bytes did not match their import digest; provenance-dependent preservation was disabled".to_owned(),
            });
            return Some(false);
        }
    }
    Some(original_face(bytes, font).is_some_and(|face| glyph_order_matches(&face, font)))
}

fn original_face<'a>(bytes: &'a [u8], font: &Font) -> Option<FontRef<'a>> {
    let mut same_count = None;
    for index in 0..u32::from(u16::MAX) {
        let Ok(face) = FontRef::from_index(bytes, index) else {
            break;
        };
        let count_matches = face
            .maxp()
            .is_ok_and(|maxp| usize::from(maxp.num_glyphs()) == font.glyphs.len());
        if count_matches && glyph_order_matches(&face, font) {
            return Some(face);
        }
        if count_matches && same_count.is_none() {
            same_count = Some(face);
        }
    }
    same_count
}

fn glyph_order_matches(face: &FontRef<'_>, font: &Font) -> bool {
    imported_glyph_order(face).is_some_and(|original| {
        original
            .iter()
            .eq(font.glyph_order().map(GlyphName::as_ref))
    })
}

fn imported_glyph_order(face: &FontRef<'_>) -> Option<Vec<String>> {
    let count = face.maxp().ok()?.num_glyphs();
    let source = GlyphNames::new(face);
    let wholly_synthesized = source.source() == GlyphNameSource::Synthesized;
    let mut used = HashSet::with_capacity(usize::from(count));
    let mut names = Vec::with_capacity(usize::from(count));
    for gid in 0..count {
        let candidate = source.get(GlyphId::new(u32::from(gid)));
        let mut value = if gid == 0 && wholly_synthesized {
            ".notdef".to_owned()
        } else if let Some(name) = candidate.filter(|name| !name.is_synthesized()) {
            name.as_str().to_owned()
        } else {
            format!("glyph{gid:05}")
        };
        if !used.insert(value.clone()) {
            value = format!("{value}.gid{gid}");
            while !used.insert(value.clone()) {
                value.push('_');
            }
        }
        names.push(value);
    }
    Some(names)
}

fn selected_master<'a>(font: &'a Font, options: &ExportOptions) -> Result<&'a MasterId, Error> {
    let id = options
        .master
        .as_ref()
        .unwrap_or_else(|| &font.default_master().id);
    font.master(id)
        .map(|master| &master.id)
        .ok_or_else(|| Error::MissingMaster(id.clone()))
}

fn has_hinting(font: &Font, master: &MasterId) -> bool {
    let master_data = font.master(master).map(|item| &item.tt);
    master_data.is_some_and(|tt| !tt.cvt.is_empty() || !tt.fpgm.is_empty() || !tt.prep.is_empty())
        || font.glyphs.values().any(|glyph| {
            glyph.master_layer(master).is_some_and(|layer| {
                layer
                    .tt_instructions
                    .as_ref()
                    .is_some_and(|instructions| !instructions.is_empty())
            })
        })
}

fn has_hinting_in_font(font: &Font, options: &ExportOptions) -> Result<bool, Error> {
    Ok(has_hinting(font, selected_master(font, options)?))
}

fn hinting_dropped_warning(
    touched: &HashSet<GlyphName>,
    verified_order: Option<bool>,
) -> ExportWarning {
    let mut glyphs: Vec<_> = touched.iter().map(ToString::to_string).collect();
    glyphs.sort();
    let message = if glyphs.is_empty() && verified_order == Some(false) {
        "TrueType instructions were dropped because the original glyph order changed".to_owned()
    } else {
        format!(
            "TrueType instructions were dropped because these glyphs changed: {}",
            glyphs.join(", ")
        )
    };
    ExportWarning {
        code: "hinting-dropped",
        message,
    }
}

fn reattach_hinting(
    font_builder: &mut FontBuilder<'_>,
    compiled: &FontRef<'_>,
    font: &Font,
    master: &MasterId,
) -> Result<(), Error> {
    let glyf = compiled
        .glyf()
        .map_err(|error| Error::parse("glyf", error))?;
    let loca = compiled
        .loca(None)
        .map_err(|error| Error::parse("loca", error))?;
    let exported: Vec<_> = font.glyphs.values().filter(|glyph| glyph.export).collect();
    let glyph_count = compiled
        .maxp()
        .map_err(|error| Error::parse("maxp", error))?
        .num_glyphs();
    if exported.len() != usize::from(glyph_count) {
        return Err(Error::parse(
            "glyph order",
            format!(
                "fontc emitted {glyph_count} glyphs for {} exported glyphs",
                exported.len()
            ),
        ));
    }

    let mut glyf_loca = GlyfLocaBuilder::new();
    let mut max_instruction_len = 0usize;
    for (gid, model_glyph) in (0..glyph_count).zip(exported) {
        let instructions = model_glyph
            .master_layer(master)
            .and_then(|layer| layer.tt_instructions.as_deref())
            .unwrap_or_default();
        max_instruction_len = max_instruction_len.max(instructions.len());
        match loca
            .get_glyf(GlyphId::new(u32::from(gid)), &glyf)
            .map_err(|error| Error::parse("glyf", error))?
        {
            Some(glyph) => {
                let mut glyph: WriteGlyph = glyph.to_owned_table();
                match &mut glyph {
                    WriteGlyph::Simple(simple) => simple.instructions = instructions.to_vec(),
                    WriteGlyph::Composite(composite) => composite.set_instructions(instructions),
                    WriteGlyph::Empty => {}
                }
                glyf_loca
                    .add_glyph(&glyph)
                    .map_err(|error| Error::write("glyf", error))?;
            }
            None if instructions.is_empty() => {
                glyf_loca
                    .add_glyph(&WriteGlyph::Empty)
                    .map_err(|error| Error::write("glyf", error))?;
            }
            None => {
                let simple = SimpleGlyph {
                    instructions: instructions.to_vec(),
                    ..SimpleGlyph::default()
                };
                glyf_loca
                    .add_glyph(&simple)
                    .map_err(|error| Error::write("glyf", error))?;
            }
        }
    }
    let (glyf, loca, loca_format) = glyf_loca.build();
    font_builder
        .add_table(&glyf)
        .map_err(|error| Error::write("glyf", error))?;
    font_builder
        .add_table(&loca)
        .map_err(|error| Error::write("loca", error))?;

    let mut head: write_fonts::tables::head::Head = compiled
        .head()
        .map_err(|error| Error::parse("head", error))?
        .to_owned_table();
    head.index_to_loc_format = loca_format as i16;
    font_builder
        .add_table(&head)
        .map_err(|error| Error::write("head", error))?;

    let tt = &font
        .master(master)
        .ok_or_else(|| Error::MissingMaster(master.clone()))?
        .tt;
    let mut maxp: write_fonts::tables::maxp::Maxp = compiled
        .maxp()
        .map_err(|error| Error::parse("maxp", error))?
        .to_owned_table();
    maxp.max_stack_elements = tt.max_stack_elements.or(maxp.max_stack_elements);
    maxp.max_storage = tt.max_storage.or(maxp.max_storage);
    maxp.max_function_defs = tt.max_function_defs.or(maxp.max_function_defs);
    maxp.max_instruction_defs = tt.max_instruction_defs.or(maxp.max_instruction_defs);
    maxp.max_twilight_points = tt.max_twilight_points.or(maxp.max_twilight_points);
    maxp.max_zones = tt.max_zones.or(maxp.max_zones);
    maxp.max_size_of_instructions =
        Some(tt.max_size_of_instructions.unwrap_or(
            u16::try_from(max_instruction_len).map_err(|_| Error::Limit("instructions"))?,
        ));
    font_builder
        .add_table(&maxp)
        .map_err(|error| Error::write("maxp", error))?;

    add_tt_programs(font_builder, font, tt);
    Ok(())
}

fn add_tt_programs(builder: &mut FontBuilder<'_>, font: &Font, tt: &fw_model::TtMasterData) {
    let source_has = |tag: Tag| {
        font.extra
            .source
            .as_ref()
            .is_some_and(|source| source.table_tags.contains(&tag))
    };
    if !tt.fpgm.is_empty() || source_has(Tag(*b"fpgm")) {
        builder.add_raw(read_fonts::types::Tag::new(b"fpgm"), tt.fpgm.clone());
    }
    if !tt.prep.is_empty() || source_has(Tag(*b"prep")) {
        builder.add_raw(read_fonts::types::Tag::new(b"prep"), tt.prep.clone());
    }
    if !tt.cvt.is_empty() || source_has(Tag(*b"cvt ")) {
        let bytes: Vec<_> = tt
            .cvt
            .iter()
            .flat_map(|value| value.to_be_bytes())
            .collect();
        builder.add_raw(read_fonts::types::Tag::new(b"cvt "), bytes);
    }
}

fn read_tag(tag: Tag) -> read_fonts::types::Tag {
    read_fonts::types::Tag::new(&tag.0)
}
