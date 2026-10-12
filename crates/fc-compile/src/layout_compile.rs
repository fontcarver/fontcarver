//! Rebuild layout through fontc's writers without compiling any glyph outlines.

use std::collections::{BTreeMap, HashMap, HashSet};

use fc_model::{Font, GlyphName, MasterId};
use fontbe::{
    features::{
        create_gather_ir_kerning_work, create_kern_segment_work, create_kerns_work,
        create_mark_work, FeatureCompilationWork, FeatureFirstPassWork,
    },
    orchestration::{BeWork, Context as BeContext},
};
use fontdrasil::{
    coords::NormalizedLocation, orchestration::Access, types::GlyphName as IrGlyphName,
};
use fontir::{
    ir::{Glyph as IrGlyph, GlyphInstance, GlyphOrder},
    orchestration::{Context as FeContext, Flags, IrWork},
    source::Source,
};
use write_fonts::{types::Tag, validate::Validate, FontWrite};

use crate::{Error, FcSource};

/// Compile layout against the exact glyph IDs of an already compiled font.
///
/// The source constructor strips geometry before producing IR. This graph runs
/// only fontc's feature writers, retaining their source ownership and automatic
/// kerning, mark, cursive and GDEF behavior. Absent tables must be removed from
/// the previous font so deleting feature source cannot leave stale lookups.
pub(crate) fn compile_layout(
    font: &Font,
    master: &MasterId,
    glyph_order: &[GlyphName],
) -> Result<BTreeMap<Tag, Vec<u8>>, Error> {
    validate_order(font, glyph_order)?;
    let source = FcSource::from_layout_font(font, master)?;
    let flags = Flags::PREFER_SIMPLE_GLYPHS;
    let fe = FeContext::new_root(flags);
    run_ir(&fe, source.create_static_metadata_work().map_err(ir_error)?)?;
    run_ir(&fe, source.create_feature_ir_work().map_err(ir_error)?)?;
    run_ir(
        &fe,
        source
            .create_kerning_locations_ir_work()
            .map_err(ir_error)?,
    )?;
    for location in &fe.kerning_locations.get().locations {
        run_ir(
            &fe,
            source
                .create_kerning_instance_ir_work(location.clone())
                .map_err(ir_error)?,
        )?;
    }
    for work in source.create_glyph_ir_work().map_err(ir_error)? {
        run_ir(&fe, work)?;
    }

    // GlyphOrderWork also decomposes outlines and can introduce glyphs. The
    // cached font already fixed this order; seed it directly instead.
    let writer = fe.copy_for_work(Access::All, Access::All);
    let mut order = GlyphOrder::new();
    for name in glyph_order {
        order.insert(IrGlyphName::new(name.as_ref()));
    }
    writer.glyph_order.set(order);
    // FcSource supplies no explicit categories and does not request inference.
    writer.gdef_categories.set(Default::default());
    if writer
        .try_get_glyph(IrGlyphName::NOTDEF)
        .is_none_or(|glyph| !glyph.emit_to_binary)
    {
        let notdef = IrGlyph::new(
            IrGlyphName::NOTDEF,
            true,
            HashSet::new(),
            HashMap::from([(
                NormalizedLocation::new(),
                GlyphInstance {
                    width: (f64::from(font.info.units_per_em) * 0.5).round(),
                    ..Default::default()
                },
            )]),
        )
        .map_err(|error| Error::InvalidSource(error.to_string()))?;
        writer.glyphs.set(notdef);
    }

    let be = BeContext::new_root(flags, None, None, false, &fe);
    run_be(&be, FeatureFirstPassWork::create())?;
    run_be(&be, create_gather_ir_kerning_work())?;
    for work in create_kern_segment_work(&be.all_kerning_pairs.get()) {
        run_be(&be, work)?;
    }
    run_be(&be, create_kerns_work())?;
    run_be(&be, create_mark_work())?;
    run_be(&be, FeatureCompilationWork::create())?;

    let mut tables = BTreeMap::new();
    if let Some(table) = be.gsub.try_get() {
        insert_table(&mut tables, Tag::new(b"GSUB"), table.as_ref())?;
    }
    if let Some(table) = be.gpos.try_get() {
        insert_table(&mut tables, Tag::new(b"GPOS"), table.as_ref())?;
    }
    if let Some(table) = be.gdef.try_get() {
        insert_table(&mut tables, Tag::new(b"GDEF"), table.as_ref())?;
    }
    if let Some(extra) = be.extra_fea_tables.try_get() {
        // These are partial overrides, or name IDs referenced by layout. They
        // require the full table merge rather than copying cached table bytes.
        if extra.name.is_some()
            || extra.head.is_some()
            || extra.hhea.is_some()
            || extra.vhea.is_some()
            || extra.os2.is_some()
            || extra.os2_builder.is_some()
            || extra.stat.is_some()
            || extra.debg.is_some()
        {
            return Err(Error::InvalidSource(
                "feature source produces non-layout tables requiring full compilation".into(),
            ));
        }
        if let Some(table) = &extra.base {
            insert_table(&mut tables, Tag::new(b"BASE"), table)?;
        }
    }
    Ok(tables)
}

fn validate_order(font: &Font, order: &[GlyphName]) -> Result<(), Error> {
    if order.first().map(GlyphName::as_ref) != Some(".notdef")
        || order.len() > usize::from(u16::MAX)
    {
        return Err(Error::InvalidSource("invalid cached glyph order".into()));
    }
    let mut seen = HashSet::new();
    for name in order {
        if !seen.insert(name)
            || (name.as_ref() != ".notdef"
                && !font.glyphs.get(name).is_some_and(|glyph| glyph.export))
        {
            return Err(Error::InvalidSource(
                "cached glyph order no longer matches source".into(),
            ));
        }
    }
    if font
        .glyphs
        .values()
        .any(|glyph| glyph.export && !seen.contains(&glyph.name))
    {
        return Err(Error::InvalidSource(
            "cached glyph order omits an exported glyph".into(),
        ));
    }
    Ok(())
}

fn run_ir(context: &FeContext, work: Box<IrWork>) -> Result<(), Error> {
    work.exec(&context.copy_for_work(work.read_access(), work.write_access()))
        .map_err(ir_error)
}

fn ir_error(error: fontir::error::Error) -> Error {
    Error::Fontc(error.into())
}

fn run_be(context: &BeContext, work: Box<BeWork>) -> Result<(), Error> {
    // The two kerning gather units advertise Unknown until fontc's private
    // scheduler expands their dependencies. Our explicit order completes all
    // prerequisites; give those units read access without running that scheduler.
    work.exec(&context.copy_for_work(Access::All, work.write_access()))
        .map_err(|error| Error::Fontc(error.into()))
}

fn insert_table<T: FontWrite + Validate>(
    tables: &mut BTreeMap<Tag, Vec<u8>>,
    tag: Tag,
    table: &T,
) -> Result<(), Error> {
    let bytes = write_fonts::dump_table(table)
        .map_err(|error| Error::write("incremental layout table", error))?;
    tables.insert(tag, bytes);
    Ok(())
}

#[cfg(test)]
mod tests {
    use fc_model::{Anchor, FeatureBlock, Font, Glyph, GlyphName, KernSide, Layer};
    use read_fonts::{FontRead, FontRef, TableProvider};
    use write_fonts::{from_obj::ToOwnedTable, types::Tag};

    use super::compile_layout;
    use crate::{compile_static_with_metadata, CompileOptions};

    fn font() -> Font {
        let mut font = Font::new_empty("Layout Work", 1000);
        let master = font.masters[0].id.clone();
        for (name, codepoint, width) in [
            ("A", 0x41, 600.0),
            ("V", 0x56, 600.0),
            ("acutecomb", 0x301, 0.0),
        ] {
            let mut glyph = Glyph::new(name);
            glyph.unicodes.push(codepoint);
            let mut layer = Layer::new_master(master.clone());
            layer.width = width;
            if name != "V" {
                layer.anchors.push(Anchor {
                    name: if name == "A" { "top" } else { "_top" }.into(),
                    x: if name == "A" { 300.0 } else { 0.0 },
                    y: if name == "A" { 700.0 } else { 0.0 },
                });
            }
            glyph.layers.push(layer);
            font.insert_glyph(glyph).unwrap();
        }
        font.masters[0].kerning_h.0.insert(
            (
                KernSide::Glyph(GlyphName::from("A")),
                KernSide::Glyph(GlyphName::from("V")),
            ),
            -80.0,
        );
        font
    }

    #[test]
    fn writers_match_full_compile_with_a_synthetic_notdef() {
        let font = font();
        let compiled =
            compile_static_with_metadata(&font, &font.masters[0].id, &CompileOptions::default())
                .unwrap();
        let tables = compile_layout(&font, &font.masters[0].id, &compiled.glyph_order).unwrap();
        let face = FontRef::new(&compiled.bytes).unwrap();
        for tag in [Tag::new(b"GPOS"), Tag::new(b"GDEF")] {
            // Parsed structures compare semantic content, including writer output.
            assert!(tables.contains_key(&tag));
        }
        let actual_gpos = read_fonts::tables::gpos::Gpos::read(read_fonts::FontData::new(
            &tables[&Tag::new(b"GPOS")],
        ))
        .unwrap();
        let actual_gpos: write_fonts::tables::gpos::Gpos = actual_gpos.to_owned_table();
        assert_eq!(actual_gpos, face.gpos().unwrap().to_owned_table());
        let actual_gdef = read_fonts::tables::gdef::Gdef::read(read_fonts::FontData::new(
            &tables[&Tag::new(b"GDEF")],
        ))
        .unwrap();
        let actual_gdef: write_fonts::tables::gdef::Gdef = actual_gdef.to_owned_table();
        assert_eq!(actual_gdef, face.gdef().unwrap().to_owned_table());
    }

    #[test]
    fn an_unknown_cached_glyph_returns_an_error() {
        let font = font();
        let order = vec![".notdef".into(), "missing".into()];
        assert!(compile_layout(&font, &font.masters[0].id, &order).is_err());
    }

    #[test]
    fn non_layout_fea_output_requires_full_compilation() {
        let mut font = font();
        font.features.prefixes.push(FeatureBlock {
            code: "table name { nameid 1 \"Changed\"; } name;".into(),
            ..Default::default()
        });
        let order = vec![".notdef".into(), "A".into(), "V".into(), "acutecomb".into()];
        let error = compile_layout(&font, &font.masters[0].id, &order).unwrap_err();
        assert!(error.to_string().contains("non-layout"));
    }
}
