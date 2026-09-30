//! Row 46 acceptance: FEA parses into the typed lookup IR, serialises back, and
//! the round trip is semantically stable on fixtures and on real fonts.

use fc_layout::fea_ir::{
    parse_fea, AnchorPoint, ClassItem, FeaIr, GlyphSet, LanguagePolicy, LookupFlag, LookupTable,
    LookupType, MarkAttachment, PairValues, Rule, Statement, ValueRecord,
};
use fc_layout::LayoutImport;
use fc_model::GlyphName;

/// Glyph names used by the hand-written fixtures.
fn order() -> Vec<GlyphName> {
    [
        ".notdef",
        "a",
        "b",
        "c",
        "d",
        "e",
        "f",
        "i",
        "f_i",
        "one",
        "two",
        "acutecomb",
        "gravecomb",
        "A",
        "B",
        "Z",
    ]
    .into_iter()
    .map(GlyphName::from)
    .collect()
}

/// Parse, serialise, reparse, and require the two IRs to be equal.
fn roundtrip(source: &str) -> FeaIr {
    let order = order();
    let first = parse_fea(source, &order).expect("first parse");
    let rendered = first.to_fea();
    let second = parse_fea(&rendered, &order)
        .unwrap_or_else(|error| panic!("reparse of\n{rendered}\nfailed: {error}"));
    assert_eq!(first, second, "round trip changed the IR:\n{rendered}");
    first
}

/// The rules of the one feature block in a fixture.
fn rules(ir: &FeaIr) -> Vec<(LookupType, Rule)> {
    fn collect(statements: &[Statement], out: &mut Vec<(LookupType, Rule)>) {
        for statement in statements {
            match statement {
                Statement::Rule(rule) => out.push((rule.lookup, rule.rule.clone())),
                Statement::Feature { statements, .. } | Statement::Lookup { statements, .. } => {
                    collect(statements, out)
                }
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    collect(&ir.statements, &mut out);
    out
}

#[test]
fn the_nine_authored_lookup_types_parse_into_typed_rules() {
    let ir = roundtrip(
        "feature test {\n\
           sub a by b;\n\
           sub a by b c;\n\
           sub a from [b c];\n\
           sub f i by f_i;\n\
           sub a b' c by d;\n\
           pos a -10;\n\
           pos a <1 2 3 4> b <5 6 7 8>;\n\
           pos base a <anchor 10 20> mark @TOP;\n\
           pos mark acutecomb <anchor 1 2> mark @TOP;\n\
         } test;",
    );
    let parsed = rules(&ir);
    let types: Vec<LookupType> = parsed.iter().map(|(lookup, _)| *lookup).collect();
    assert_eq!(
        types,
        vec![
            LookupType::gsub(1),
            LookupType::gsub(2),
            LookupType::gsub(3),
            LookupType::gsub(4),
            LookupType::gsub(6),
            LookupType::gpos(1),
            LookupType::gpos(2),
            LookupType::gpos(4),
            LookupType::gpos(6),
        ]
    );
    assert_eq!(
        parsed[0].1,
        Rule::SingleSub {
            target: GlyphSet::Glyph("a".into()),
            replacement: GlyphSet::Glyph("b".into()),
        }
    );
    assert_eq!(
        parsed[2].1,
        Rule::AlternateSub {
            target: GlyphSet::Glyph("a".into()),
            alternates: GlyphSet::Literal(vec![
                ClassItem::Glyph("b".into()),
                ClassItem::Glyph("c".into())
            ]),
        }
    );
    let Rule::PairPos { values, .. } = &parsed[6].1 else {
        panic!("expected a pair rule, got {:?}", parsed[6].1);
    };
    assert_eq!(
        values,
        &PairValues::Both(
            ValueRecord::Placement {
                x_placement: 1,
                y_placement: 2,
                x_advance: 3,
                y_advance: 4
            },
            ValueRecord::Placement {
                x_placement: 5,
                y_placement: 6,
                x_advance: 7,
                y_advance: 8
            }
        )
    );
    assert_eq!(
        parsed[7].1,
        Rule::MarkToBase {
            target: GlyphSet::Glyph("a".into()),
            marks: vec![MarkAttachment {
                anchor: AnchorPoint::At { x: 10, y: 20 },
                class: Some("TOP".into()),
            }],
        }
    );
    assert_eq!(ir.typed_rules(), 9);
    assert_eq!(ir.verbatim_statements(), 0);
}

#[test]
fn cursive_ligature_reverse_and_contextual_positioning_are_typed_too() {
    let ir = roundtrip(
        "feature test {\n\
           pos cursive a <anchor 1 2> <anchor 3 4>;\n\
           pos ligature f_i <anchor 1 2> mark @TOP ligComponent <anchor 3 4> mark @TOP;\n\
           rsub a b' c by d;\n\
           pos a b' <1 2 3 4> c;\n\
           pos a b' lookup Adjust c;\n\
           ignore sub a b' c, d e' f;\n\
           ignore pos a b' c;\n\
           enum pos a b -20;\n\
         } test;",
    );
    let types: Vec<LookupType> = rules(&ir).iter().map(|(lookup, _)| *lookup).collect();
    assert_eq!(
        types,
        vec![
            LookupType::gpos(3),
            LookupType::gpos(5),
            LookupType::gsub(8),
            LookupType::gpos(8),
            LookupType::gpos(8),
            LookupType::gsub(6),
            LookupType::gpos(8),
            LookupType::gpos(2),
        ]
    );
    assert_eq!(ir.verbatim_statements(), 0);
    let parsed = rules(&ir);
    let Rule::IgnoreSub(patterns) = &parsed[5].1 else {
        panic!("expected ignore sub");
    };
    assert_eq!(patterns.len(), 2);
    let Rule::PairPos {
        enumerated, values, ..
    } = &parsed[7].1
    else {
        panic!("expected pair pos");
    };
    assert!(enumerated);
    assert_eq!(values, &PairValues::AfterPair(ValueRecord::Advance(-20)));
}

#[test]
fn classes_language_systems_flags_and_named_lookups_round_trip() {
    let ir = roundtrip(
        "languagesystem DFLT dflt;\n\
         languagesystem latn TRK;\n\
         @Lower = [a b A - Z @Marks];\n\
         @Marks = [acutecomb gravecomb];\n\
         markClass [acutecomb gravecomb] <anchor 0 500> @TOP;\n\
         lookup Adjust useExtension {\n\
           lookupflag RightToLeft IgnoreMarks MarkAttachmentType @Marks;\n\
           pos a 10;\n\
         } Adjust;\n\
         feature test {\n\
           script latn;\n\
           language TRK exclude_dflt;\n\
           lookup Adjust;\n\
           subtable;\n\
           lookupflag 0;\n\
         } test;\n\
         feature aalt {\n\
           feature test;\n\
         } aalt;",
    );
    assert_eq!(
        ir.statements[0],
        Statement::LanguageSystem {
            script: "DFLT".into(),
            language: "dflt".into()
        }
    );
    assert_eq!(
        ir.statements[2],
        Statement::GlyphClassDef {
            name: "Lower".into(),
            members: vec![
                ClassItem::Glyph("a".into()),
                ClassItem::Glyph("b".into()),
                ClassItem::Range("A".into(), "Z".into()),
                ClassItem::Named("Marks".into()),
            ],
        }
    );
    let Statement::MarkClassDef { class, anchor, .. } = &ir.statements[4] else {
        panic!("expected markClass");
    };
    assert_eq!(class, "TOP");
    assert_eq!(anchor, &AnchorPoint::At { x: 0, y: 500 });
    let Statement::Lookup {
        name,
        use_extension,
        statements,
    } = &ir.statements[5]
    else {
        panic!("expected a lookup block");
    };
    assert_eq!(name, "Adjust");
    assert!(use_extension);
    assert_eq!(
        statements[0],
        Statement::LookupFlag(LookupFlag::Named {
            right_to_left: true,
            ignore_base_glyphs: false,
            ignore_ligatures: false,
            ignore_marks: true,
            mark_attachment_type: Some("Marks".into()),
            use_mark_filtering_set: None,
        })
    );
    let Statement::Feature { statements, .. } = &ir.statements[6] else {
        panic!("expected a feature block");
    };
    assert_eq!(statements[0], Statement::Script("latn".into()));
    assert_eq!(
        statements[1],
        Statement::Language {
            tag: "TRK".into(),
            policy: LanguagePolicy {
                exclude_dflt: true,
                include_dflt: false,
                required: false
            },
        }
    );
    assert_eq!(statements[2], Statement::LookupCall("Adjust".into()));
    assert_eq!(statements[3], Statement::Subtable);
    assert_eq!(statements[4], Statement::LookupFlag(LookupFlag::Value(0)));
    let Statement::Feature { statements, .. } = &ir.statements[7] else {
        panic!("expected aalt");
    };
    assert_eq!(statements[0], Statement::FeatureCall("test".into()));
}

#[test]
fn unsupported_constructs_and_comments_are_preserved_verbatim() {
    let source = "# a leading comment\n\
        table head { FontRevision 1.5; } head;\n\
        anchorDef 120 -20 TOP_ANCHOR;\n\
        valueRecordDef 40 SHIFT;\n\
        feature test {\n\
          # inside the block\n\
          featureNames { name \"Test\"; };\n\
          pos a <1 2 3 4 <device 11 -1> <device NULL> <device NULL> <device NULL>>;\n\
        } test;";
    let ir = roundtrip(source);
    assert_eq!(
        ir.statements[0],
        Statement::Comment("# a leading comment".into())
    );
    assert_eq!(
        ir.statements[1],
        Statement::Verbatim("table head { FontRevision 1.5; } head;".into())
    );
    assert_eq!(
        ir.statements[2],
        Statement::Verbatim("anchorDef 120 -20 TOP_ANCHOR;".into())
    );
    assert_eq!(
        ir.statements[3],
        Statement::Verbatim("valueRecordDef 40 SHIFT;".into())
    );
    let Statement::Feature { statements, .. } = &ir.statements[4] else {
        panic!("expected a feature block");
    };
    assert_eq!(
        statements[0],
        Statement::Comment("# inside the block".into())
    );
    assert!(matches!(statements[1], Statement::Verbatim(_)));
    // A device-table value record stays typed as GPOS 1 with a verbatim record.
    let Statement::Rule(rule) = &statements[2] else {
        panic!("expected a positioning rule");
    };
    assert_eq!(rule.lookup, LookupType::gpos(1));
    let Rule::SinglePos { value, .. } = &rule.rule else {
        panic!("expected single positioning");
    };
    assert!(matches!(value, ValueRecord::Other(text) if text.contains("device")));
    // Nothing is dropped: every original statement survives the render.
    let rendered = ir.to_fea();
    for fragment in [
        "FontRevision 1.5",
        "anchorDef 120 -20 TOP_ANCHOR",
        "valueRecordDef 40 SHIFT",
        "featureNames",
        "device 11 -1",
        "# a leading comment",
        "# inside the block",
    ] {
        assert!(
            rendered.contains(fragment),
            "{fragment} missing from\n{rendered}"
        );
    }
}

#[test]
fn keyword_like_glyph_names_and_cids_survive_the_round_trip() {
    let order: Vec<GlyphName> = ["sub", "by", "mark", "pos"]
        .into_iter()
        .map(GlyphName::from)
        .collect();
    let ir = parse_fea(
        "feature test { sub \\sub by \\by; pos \\mark \\pos -5; } test;",
        &order,
    )
    .expect("parse escaped keywords");
    let rendered = ir.to_fea();
    assert!(rendered.contains("sub \\sub by \\by;"), "{rendered}");
    assert_eq!(
        parse_fea(&rendered, &order).expect("reparse escaped keywords"),
        ir
    );
    // CIDs keep their numeric identity rather than becoming glyph names.
    let cids = parse_fea("@Cids = [\\1 - \\5 \\7];", &[]).expect("parse CIDs");
    assert_eq!(
        cids.statements[0],
        Statement::GlyphClassDef {
            name: "Cids".into(),
            members: vec![ClassItem::CidRange(1, 5), ClassItem::Cid(7)],
        }
    );
    assert_eq!(parse_fea(&cids.to_fea(), &[]).expect("reparse CIDs"), cids);
}

#[test]
fn malformed_source_is_reported_instead_of_guessed() {
    let error = parse_fea("feature test { sub a by; } test;", &order()).expect_err("parse error");
    assert!(!error.messages.is_empty());
    assert!(parse_fea("@Class = [a", &order()).is_err());
}

#[test]
fn an_include_statement_is_refused_with_an_actionable_message() {
    // The editor and the compiler both disable external includes, so the IR
    // reports the statement instead of silently reading nothing.
    let error = parse_fea(
        "include(other.fea);\nfeature test { sub a by b; } test;",
        &order(),
    )
    .expect_err("include is refused");
    let message = error.to_string();
    assert!(message.contains("include"), "{message}");
    assert!(message.contains("Inline the included file"), "{message}");
}

/// The corpus both the visual lookup editor and this IR are checked against.
fn shared_corpus() -> serde_json::Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/visual-lookup-corpus.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    serde_json::from_str(&text).expect("corpus is valid JSON")
}

#[test]
fn the_generated_and_visual_editor_corpus_round_trips() {
    // One shared fixture holds the exact text the visual lookup editor emits
    // and the recipes generate, so the editor and the IR cannot drift apart.
    let corpus = shared_corpus();
    let names: Vec<GlyphName> = corpus["glyphOrder"]
        .as_array()
        .expect("glyphOrder")
        .iter()
        .map(|name| GlyphName::from(name.as_str().expect("glyph name")))
        .collect();
    let sources: Vec<String> = corpus["visual"]
        .as_array()
        .expect("visual")
        .iter()
        .map(|entry| {
            // The editor stores a feature body; the IR reads whole source.
            let tag = entry["tag"].as_str().expect("tag");
            let body = entry["fea"].as_str().expect("fea");
            format!("feature {tag} {{\n{body}\n}} {tag};")
        })
        .chain(
            corpus["generated"]
                .as_array()
                .expect("generated")
                .iter()
                .map(|entry| entry.as_str().expect("generated entry").to_owned()),
        )
        .collect();
    assert!(sources.len() >= 15, "corpus shrank to {}", sources.len());
    for source in sources {
        let first = parse_fea(&source, &names)
            .unwrap_or_else(|error| panic!("could not parse corpus entry\n{source}\n{error}"));
        assert_eq!(first.verbatim_statements(), 0, "not typed:\n{source}");
        let rendered = first.to_fea();
        let second = parse_fea(&rendered, &names)
            .unwrap_or_else(|error| panic!("reparse failed\n{rendered}\n{error}"));
        assert_eq!(first, second, "round trip changed:\n{source}\n{rendered}");
    }
}

/// Build a document whose only layout is `source`, then compile it.
fn compiled_font(source: &str) -> fc_model::Font {
    let mut font = fc_model::Font::new_empty("Decompiled layout", 1000);
    let master = font.default_master().id.clone();
    for (name, codepoint) in [
        (".notdef", None),
        ("a", Some(0x61_u32)),
        ("b", Some(0x62)),
        ("c", Some(0x63)),
        ("d", Some(0x64)),
        ("a.alt", None),
        ("a_b", None),
        ("acutecomb", Some(0x301)),
        ("gravecomb", Some(0x300)),
    ] {
        let mut glyph = fc_model::Glyph::new(name);
        glyph.unicodes.extend(codepoint);
        let mut layer = fc_model::Layer::new_master(master.clone());
        layer.width = if name == "acutecomb" || name == "gravecomb" {
            0.0
        } else {
            500.0
        };
        glyph.layers.push(layer);
        font.insert_glyph(glyph).expect("insert glyph");
    }
    font.features.prefixes.push(fc_model::FeatureBlock {
        code: source.into(),
        ..Default::default()
    });
    let bytes = fc_compile::export(&font, &|_| 0, &Default::default())
        .expect("export layout font")
        .bytes;
    fc_sfnt::read_bytes(&bytes, &Default::default())
        .expect("reread exported font")
        .fonts
        .remove(0)
}

#[test]
fn real_compiled_layout_decompiles_and_round_trips_through_the_ir() {
    // Every statement here compiles to real GSUB/GPOS binary, is read back out
    // of the binary by the layout decompiler, and must survive the typed IR.
    let sources = [
        "languagesystem DFLT dflt;\nlanguagesystem latn dflt;\nlookup Single { sub a by a.alt; } Single;\nfeature salt { lookup Single; } salt;\nfeature liga { sub a b by a_b; } liga;\nfeature kern { pos a b -40; } kern;",
        "languagesystem latn dflt;\nlanguagesystem latn TRK;\nfeature rlig { script latn; language dflt required; sub a b by a_b; language TRK exclude_dflt required; sub a by a.alt; } rlig;",
        "@marks = [acutecomb gravecomb];\nmarkClass acutecomb <anchor 0 500> @TOP;\nmarkClass gravecomb <anchor 0 500> @TOP;\nfeature mark { pos base a <anchor 250 700> mark @TOP; } mark;\nfeature mkmk { pos mark acutecomb <anchor 0 900> mark @TOP; } mkmk;",
        "feature calt { sub a b' c by d; sub [a b] c' by d; ignore sub a b' c; } calt;",
        "feature kern { pos cursive a <anchor 0 0> <anchor 500 0>; enum pos [a b] c -30; pos a <0 0 -10 0> b <0 0 -20 0>; } kern;",
        "feature test { sub a from [b c]; sub a by b c; } test;",
    ];
    let mut typed_total = 0;
    for source in sources {
        let font = compiled_font(source);
        let import = LayoutImport::from_font(&font);
        assert!(
            import.can_convert(),
            "fixture layout is not convertible: {:?}",
            import.blockers
        );
        let decompiled = import.decompile().expect("decompile compiled layout");
        let order: Vec<GlyphName> = font.glyph_order().cloned().collect();
        let first = parse_fea(&decompiled, &order).unwrap_or_else(|error| {
            panic!("decompiled source did not parse: {error}\n{decompiled}")
        });
        let rendered = first.to_fea();
        let second = parse_fea(&rendered, &order)
            .unwrap_or_else(|error| panic!("rendered source did not parse: {error}\n{rendered}"));
        assert_eq!(first, second, "round trip changed the IR:\n{rendered}");
        assert!(
            first.typed_rules() > 0,
            "no typed rule in decompiled layout:\n{decompiled}"
        );
        // Only whole-table blocks stay verbatim; every lookup is typed.
        for statement in &first.statements {
            if let Statement::Verbatim(text) = statement {
                assert!(
                    text.starts_with("table "),
                    "untyped lookup statement {text:?} in:\n{decompiled}"
                );
            }
        }
        typed_total += first.typed_rules();
    }
    assert!(typed_total >= 15, "only {typed_total} typed rules");
}

#[test]
fn licensed_fixture_fonts_either_round_trip_or_report_their_blockers() {
    // The licensed corpus keeps the documented P2b conversion blockers, so most
    // of it cannot be decompiled at all. Whatever can be is round tripped here;
    // the rest is reported so the boundary stays visible rather than implied.
    let mut decompiled = 0;
    let mut blocked = Vec::new();
    for name in [
        "SourceSans3Min.otf",
        "SourceSerif4Min.otf",
        "NotoSans-Regular.ttf",
        "NotoSerifTibetan-Regular.ttf",
        "DejaVuSans.ttf",
    ] {
        let bytes = std::fs::read(fc_fixtures::path(name)).expect("read fixture");
        let font = fc_sfnt::read_bytes(&bytes, &Default::default())
            .expect("read font")
            .fonts
            .remove(0);
        let import = LayoutImport::from_font(&font);
        match import.decompile() {
            Ok(source) => {
                let order: Vec<GlyphName> = font.glyph_order().cloned().collect();
                let first = parse_fea(&source, &order).unwrap_or_else(|error| {
                    panic!("{name}: decompiled source did not parse: {error}")
                });
                let rendered = first.to_fea();
                let second = parse_fea(&rendered, &order).unwrap_or_else(|error| {
                    panic!("{name}: rendered source did not parse: {error}")
                });
                assert_eq!(first, second, "{name}: round trip changed the IR");
                eprintln!(
                    "{name}: {} statements, {} typed rules, {} verbatim",
                    first.statements.len(),
                    first.typed_rules(),
                    first.verbatim_statements()
                );
                decompiled += 1;
            }
            Err(_) => {
                assert!(
                    !import.blockers.is_empty(),
                    "{name}: decompilation failed without a recorded blocker"
                );
                blocked.push(name);
            }
        }
    }
    eprintln!("decompiled {decompiled}; blocked by P2b conversion limits: {blocked:?}");
    assert_eq!(decompiled + blocked.len(), 5);
}

#[test]
fn the_ir_reports_which_table_and_type_each_rule_compiles_into() {
    let ir = roundtrip("feature test { sub a by b; pos a -1; } test;");
    let parsed = rules(&ir);
    assert_eq!(parsed[0].0.table, LookupTable::Gsub);
    assert_eq!(parsed[1].0.table, LookupTable::Gpos);
    assert_eq!(parsed[0].0.to_string(), "GSUB 1");
    assert_eq!(parsed[1].0.to_string(), "GPOS 1");
}
