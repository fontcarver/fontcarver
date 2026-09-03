//! Verify: (a) read-fonts -> write-fonts owned-table round trip for layout tables,
//! (b) FontBuilder mixing regenerated + verbatim tables,
//! (c) fea-rs compiling a .fea string into write-fonts GSUB/GPOS.
use read_fonts::{FontRef, TableProvider};
use write_fonts::{dump_table, from_obj::ToOwnedTable, FontBuilder};

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf".to_string()
    });
    let bytes = std::fs::read(&path).unwrap();
    let font = FontRef::new(&bytes).unwrap();
    println!("== {} ==", path);

    // ---------- (a) layout table round trip ----------
    for tag in ["GSUB", "GPOS", "GDEF"] {
        let raw = font
            .table_data(read_fonts::types::Tag::new_checked(tag.as_bytes()).unwrap())
            .map(|d| d.as_bytes().to_vec());
        let Some(raw) = raw else { println!("{tag}: absent"); continue };
        let out: Option<Vec<u8>> = match tag {
            "GSUB" => font.gsub().ok().map(|t| {
                let owned: write_fonts::tables::gsub::Gsub = t.to_owned_table();
                dump_table(&owned).unwrap()
            }),
            "GPOS" => font.gpos().ok().map(|t| {
                let owned: write_fonts::tables::gpos::Gpos = t.to_owned_table();
                dump_table(&owned).unwrap()
            }),
            "GDEF" => font.gdef().ok().map(|t| {
                let owned: write_fonts::tables::gdef::Gdef = t.to_owned_table();
                dump_table(&owned).unwrap()
            }),
            _ => None,
        };
        let Some(out) = out else { println!("{tag}: parse failed"); continue };
        println!(
            "{tag}: orig {} B -> re-dumped {} B, byte-identical: {}",
            raw.len(),
            out.len(),
            raw == out
        );
        // Re-parse the re-dumped table to prove it is still valid.
        let ok = match tag {
            "GSUB" => read_fonts::FontRead::read(read_fonts::FontData::new(&out))
                .map(|_: read_fonts::tables::gsub::Gsub| ()).is_ok(),
            "GPOS" => read_fonts::FontRead::read(read_fonts::FontData::new(&out))
                .map(|_: read_fonts::tables::gpos::Gpos| ()).is_ok(),
            _ => read_fonts::FontRead::read(read_fonts::FontData::new(&out))
                .map(|_: read_fonts::tables::gdef::Gdef| ()).is_ok(),
        };
        println!("      re-dumped {tag} re-parses: {ok}");
    }

    // ---------- (b) FontBuilder mixing ----------
    let mut head: write_fonts::tables::head::Head = font.head().unwrap().to_owned_table();
    head.font_revision = write_fonts::types::Fixed::from_f64(2.5);
    let mut builder = FontBuilder::new();
    builder.add_table(&head).unwrap();
    builder.copy_missing_tables(font.clone());
    let rebuilt = builder.build();
    let rf = FontRef::new(&rebuilt).unwrap();
    println!(
        "\nFontBuilder: {} B -> {} B; tables {} -> {}; font_revision now {:?}",
        bytes.len(),
        rebuilt.len(),
        font.table_directory().table_records().len(),
        rf.table_directory().table_records().len(),
        rf.head().unwrap().font_revision()
    );
    // shape with both to check semantic equivalence
    shape_report("original", &bytes);
    shape_report("rebuilt ", &rebuilt);

    // GSUB regenerated into the font (verbatim head + regenerated GSUB)
    let gsub_owned: write_fonts::tables::gsub::Gsub = font.gsub().unwrap().to_owned_table();
    let mut b2 = FontBuilder::new();
    b2.add_table(&gsub_owned).unwrap();
    b2.copy_missing_tables(font.clone());
    let regen = b2.build();
    shape_report("regen-GSUB", &regen);

    // ---------- (c) fea-rs from a string ----------
    use fea_rs::compile::{NopFeatureProvider, NopVariationInfo, Opts};
    use fea_rs::GlyphMap;
    let glyph_map = GlyphMap::new([".notdef", "a", "b", "c", "f", "i", "f_i"]).unwrap();
    let fea = r#"
languagesystem DFLT dflt;
languagesystem latn dflt;
feature liga {
    sub f i by f_i;
} liga;
feature kern {
    pos a b -30;
    pos b c <-10 0 -20 0>;
} kern;
table GDEF {
    GlyphClassDef [a b c f i], [], [], ;
} GDEF;
"#;
    let (tree, diags) = fea_rs::parse::parse_string(fea);
    println!("\nfea-rs parse diagnostics: {} ({})", diags.len(), if diags.has_errors() { "HAS ERRORS" } else { "clean" });
    if diags.has_errors() { println!("{}", diags.display()); }
    match fea_rs::compile::compile::<NopVariationInfo, NopFeatureProvider>(
        &tree, &glyph_map, None, None, Opts::new(),
    ) {
        Ok((compilation, warnings)) => {
            println!(
                "fea-rs compile OK; warnings={}; gsub={} gpos={} gdef={} name={} head={} os2={} stat={} base={}",
                warnings.len(),
                compilation.gsub.is_some(),
                compilation.gpos.is_some(),
                compilation.gdef.is_some(),
                compilation.name.is_some(),
                compilation.head.is_some(),
                compilation.os2.is_some(),
                compilation.stat.is_some(),
                compilation.base.is_some(),
            );
            let bin = compilation.to_binary(&glyph_map).unwrap();
            let f = FontRef::new(&bin).unwrap();
            let tags: Vec<String> = f.table_directory().table_records().iter().map(|r| r.tag().to_string()).collect();
            println!("Compilation::to_binary -> {} B, tables: {}", bin.len(), tags.join(" "));
            let gsub = compilation.gsub.as_ref().unwrap();
            println!("GSUB lookups: {}", gsub.lookup_list.lookups.len());
            let gpos = compilation.gpos.as_ref().unwrap();
            println!("GPOS lookups: {}", gpos.lookup_list.lookups.len());
        }
        Err(errs) => println!("fea-rs compile FAILED:\n{}", errs.display()),
    }

    // deliberately broken fea, to show diagnostics
    let (tree, _d) = fea_rs::parse::parse_string("feature liga { sub f i by nonexistent; } liga;\n");
    let v = fea_rs::compile::validate::<NopVariationInfo>(&tree, &glyph_map, None);
    println!("\nvalidate() on bad fea -> {} diagnostics, has_errors={}", v.len(), v.has_errors());
    println!("{}", v.display());
}

fn shape_report(label: &str, bytes: &[u8]) {
    use harfrust::{FontRef as HrFontRef, ShapeOptions, ShaperData, UnicodeBuffer};
    let f = HrFontRef::from_index(bytes, 0).unwrap();
    let d = ShaperData::new(&f);
    let s = d.shaper(&f).build();
    let mut buf = UnicodeBuffer::new();
    buf.push_str("AVATAR Wa fi");
    buf.guess_segment_properties();
    let out = s.shape(buf, ShapeOptions::new());
    let sig: Vec<String> = out
        .glyph_infos()
        .iter()
        .zip(out.glyph_positions().iter())
        .map(|(i, p)| format!("{}:{}", i.glyph_id, p.x_advance))
        .collect();
    println!("  shape[{label}] = {}", sig.join(" "));
}
