//! Actual compiler round trips for the imported layout authority and graph.
use fc_layout::{equivalent_layout, LayoutImport};
use fc_model::{FeatureBlock, FeaturesSource, Font, Glyph, Layer, Tag};

fn source_font(source: &str) -> Font {
    let mut font = Font::new_empty("Layout conversion", 1000);
    let master = font.default_master().id.clone();
    for (name, codepoint) in [
        (".notdef", None),
        ("a", Some(0x61)),
        ("b", Some(0x62)),
        ("c", Some(0x63)),
        ("a.alt", None),
        ("a_b", None),
    ] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes.extend(codepoint);
        let mut layer = Layer::new_master(master.clone());
        layer.width = 500.0;
        glyph.layers.push(layer);
        font.insert_glyph(glyph).unwrap();
    }
    font.features.prefixes.push(FeatureBlock {
        code: source.into(),
        ..Default::default()
    });
    font
}
fn import(font: &Font) -> Font {
    let bytes = fc_compile::export(font, &|_| 0, &Default::default())
        .unwrap()
        .bytes;
    fc_sfnt::read_bytes(&bytes, &Default::default())
        .unwrap()
        .fonts
        .remove(0)
}
fn roundtrip(source: &str) -> (LayoutImport, Font) {
    let original = import(&source_font(source));
    let layout = LayoutImport::from_font(&original);
    assert!(layout.can_convert(), "{:?}", layout.blockers);
    let fea = layout.decompile().unwrap();
    let mut candidate = original.clone();
    candidate.features.source_of_truth = FeaturesSource::Fea;
    candidate.features.prefixes = vec![FeatureBlock {
        code: fea.clone(),
        ..Default::default()
    }];
    let compiled = fc_compile::compile_static_with_metadata(
        &candidate,
        &candidate.default_master().id,
        &Default::default(),
    )
    .unwrap_or_else(|e| panic!("{e}\n{fea}"));
    equivalent_layout(&layout, &compiled.bytes, &compiled.glyph_order)
        .unwrap_or_else(|e| panic!("{e}\n{fea}"));
    (layout, original)
}
#[test]
fn decompiled_simple_layout_compiles_and_preserves_feature_graph() {
    roundtrip("languagesystem DFLT dflt; languagesystem latn dflt; lookup Single { sub a by a.alt; } Single; feature salt { lookup Single; } salt; feature liga { sub a b by a_b; } liga; feature kern { pos a b -40; } kern;");
}
#[test]
fn required_and_script_language_registrations_roundtrip() {
    roundtrip("languagesystem latn dflt; languagesystem latn TRK; feature rlig { script latn; language dflt required; sub a b by a_b; language TRK exclude_dflt required; sub a by a.alt; } rlig;");
}
#[test]
fn inventory_blocks_competing_authorities_and_old_contradictions_without_mutation() {
    let mut font = import(&source_font("feature liga { sub a b by a_b; } liga;"));
    for tag in [*b"kern", *b"kerx", *b"morx", *b"mort", *b"JSTF"] {
        let mut candidate = font.clone();
        candidate.extra.raw_tables.insert(Tag(tag), vec![0; 4]);
        let snapshot = candidate.clone();
        let layout = LayoutImport::from_font(&candidate);
        assert!(!layout.can_convert());
        assert!(layout
            .blockers
            .iter()
            .any(|block| block.table == Tag(tag).to_string()));
        assert_eq!(candidate, snapshot);
    }
    font.features.source_of_truth = FeaturesSource::Fea;
    assert!(LayoutImport::from_font(&font)
        .blockers
        .iter()
        .any(|b| b.table == "authority"));
}
#[test]
fn every_truncated_table_is_rejected_and_still_inspectable() {
    let font = import(&source_font("feature liga { sub a b by a_b; } liga;"));
    let bytes = font.extra.raw_tables.get(&Tag(*b"GSUB")).unwrap();
    for length in 0..bytes.len() {
        let mut candidate = font.clone();
        candidate
            .extra
            .raw_tables
            .insert(Tag(*b"GSUB"), bytes[..length].to_vec());
        let layout = LayoutImport::from_font(&candidate);
        assert!(!layout.can_convert(), "prefix {length}");
        assert!(layout.tables.iter().any(|table| table.tag == "GSUB"));
    }
}
#[test]
fn fingerprint_includes_tables_and_original_glyph_provenance() {
    let (_, mut font) = roundtrip("feature liga { sub a b by a_b; } liga;");
    let hash = LayoutImport::fingerprint_for_font(&font);
    assert_eq!(hash, LayoutImport::from_font(&font).fingerprint);
    font.extra.source.as_mut().unwrap().glyph_order.swap(1, 2);
    assert_ne!(hash, LayoutImport::fingerprint_for_font(&font));
    assert!(!LayoutImport::from_font(&font).can_convert());
}
#[test]
fn changed_rule_is_not_accepted_by_structural_equivalence() {
    let (layout, mut font) = roundtrip("feature kern { pos a b -40; } kern;");
    font.features.source_of_truth = FeaturesSource::Fea;
    font.features.prefixes = vec![FeatureBlock {
        code: "feature kern { pos a b -41; } kern;".into(),
        ..Default::default()
    }];
    let compiled = fc_compile::compile_static_with_metadata(
        &font,
        &font.default_master().id,
        &Default::default(),
    )
    .unwrap();
    assert!(equivalent_layout(&layout, &compiled.bytes, &compiled.glyph_order).is_err());
}
#[test]
fn base_only_and_legacy_only_binary_import_have_original_table_authority() {
    let original = source_font(
        "table BASE { HorizAxis.BaseTagList romn; HorizAxis.BaseScriptList latn romn 0; } BASE;",
    );
    let font = import(&original);
    assert_eq!(
        font.features.source_of_truth,
        FeaturesSource::ImportedTables
    );
    assert!(LayoutImport::from_font(&font)
        .tables
        .iter()
        .any(|t| t.tag == "BASE"));
}

#[test]
fn licensed_real_font_inventory_reports_actual_coverage_without_mutation() {
    for name in [
        "SourceSans3Min.otf",
        "SourceSerif4Min.otf",
        "NotoSans-Regular.ttf",
        "NotoSerifTibetan-Regular.ttf",
        "DejaVuSans.ttf",
    ] {
        let path = fc_fixtures::path(name);
        let bytes = std::fs::read(path).unwrap();
        let font = fc_sfnt::read_bytes(&bytes, &Default::default())
            .unwrap()
            .fonts
            .remove(0);
        let fingerprint = LayoutImport::fingerprint_for_font(&font);
        let layout = LayoutImport::from_font(&font);
        assert_eq!(fingerprint, LayoutImport::fingerprint_for_font(&font));
        eprintln!(
            "{name}: {} lookups, {} blockers: {:?}",
            layout.lookups.len(),
            layout.blockers.len(),
            layout.blockers
        );
        if name == "NotoSerifTibetan-Regular.ttf" {
            assert!(!layout.can_convert());
            assert!(layout
                .blockers
                .iter()
                .any(|block| block.table == "source" && block.reason.contains("work budget")));
            assert!(
                !layout.lookups.is_empty(),
                "bounded inspector remains available"
            );
        }
        if layout.can_convert() {
            let source = layout.decompile().unwrap();
            let mut candidate = font.clone();
            candidate.features.source_of_truth = FeaturesSource::Fea;
            candidate.features.prefixes = vec![FeatureBlock {
                code: source,
                ..Default::default()
            }];
            candidate.feature_settings.writer_policy.kerning = false;
            candidate.feature_settings.writer_policy.anchors = false;
            match fc_compile::compile_static_with_metadata(
                &candidate,
                &candidate.default_master().id,
                &Default::default(),
            ) {
                Ok(compiled) => eprintln!(
                    "{name}: canonical equivalence {:?}",
                    equivalent_layout(&layout, &compiled.bytes, &compiled.glyph_order)
                ),
                Err(error) => {
                    let validation = fc_compile::validate_feature_candidate(
                        &candidate,
                        &candidate.features,
                        &candidate.feature_settings,
                    );
                    panic!("{name}: decompiler claimed support but source did not compile: {error}; {:?}", validation.diagnostics.iter().take(6).collect::<Vec<_>>());
                }
            }
        }
    }
}

#[test]
fn valid_conversion_backup_keeps_original_inspector_available() {
    let (_, mut font) = roundtrip("feature liga { sub a b by a_b; } liga;");
    let backup = fc_model::LayoutConversion {
        original_features: font.features.clone(),
        original_glyph_order: font.extra.source.as_ref().unwrap().glyph_order.clone(),
        baseline_fingerprint: LayoutImport::fingerprint_for_font(&font),
        ..Default::default()
    };
    font.features.source_of_truth = FeaturesSource::Fea;
    font.layout_conversion = Some(backup);
    assert!(!LayoutImport::from_font(&font)
        .blockers
        .iter()
        .any(|block| block.table == "authority"));
    font.layout_conversion
        .as_mut()
        .unwrap()
        .baseline_fingerprint
        .clear();
    assert!(LayoutImport::from_font(&font)
        .blockers
        .iter()
        .any(|block| block.table == "authority"));
}

#[test]
fn compiled_inventory_requires_complete_glyph_id_mapping() {
    let source = source_font("feature liga { sub a b by a_b; } liga;");
    let compiled = fc_compile::compile_static_with_metadata(
        &source,
        &source.default_master().id,
        &Default::default(),
    )
    .unwrap();
    assert!(LayoutImport::from_bytes(&compiled.bytes, &compiled.glyph_order[1..]).is_err());
}

#[test]
fn blocked_feature_parameters_retain_decoded_lookup_rules_and_paged_glyphs() {
    let bytes = std::fs::read(fc_fixtures::path("NotoSans-Regular.ttf")).unwrap();
    let font = fc_sfnt::read_bytes(&bytes, &Default::default())
        .unwrap()
        .fonts
        .remove(0);
    let import = LayoutImport::from_font(&font);
    assert!(import
        .blockers
        .iter()
        .any(|block| block.table == "GSUB" && block.reason.contains("FeatureParams")));
    assert!(!import.can_convert());
    let lookup = import
        .lookups
        .iter()
        .find(|lookup| lookup.table == "GSUB" && !lookup.rules.is_empty())
        .expect("known substitutions remain inspectable");
    assert!(!lookup.rules[0].glyphs.is_empty());
    assert!(!import
        .rule_glyphs("GSUB", lookup.index, 0)
        .unwrap()
        .is_empty());
}

#[test]
fn gdef_classes_carets_and_both_filter_dependencies_pass_the_full_graph_gate() {
    roundtrip("@marks = [c]; table GDEF { GlyphClassDef [a], [a_b], [c], [b]; LigatureCaretByPos a_b -20 300; } GDEF; lookup Attach { lookupflag MarkAttachmentType @marks; pos a 10; } Attach; lookup Filter { lookupflag UseMarkFilteringSet @marks; pos a 20; } Filter; feature kern { lookup Attach; lookup Filter; } kern;");
}

#[test]
fn both_base_axes_and_script_coordinates_pass_the_full_graph_gate() {
    roundtrip("table BASE { HorizAxis.BaseTagList ideo romn; HorizAxis.BaseScriptList cyrl romn -120 0, latn romn -100 0; VertAxis.BaseTagList ideo romn; VertAxis.BaseScriptList hani ideo 0 120; } BASE;");
}
