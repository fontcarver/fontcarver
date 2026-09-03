use std::{collections::BTreeMap, str::FromStr, sync::Arc};

use fw_model::*;
use kurbo::Affine;
use serde_json::json;

#[test]
fn populated_font_round_trips_semantically() {
    let mut font = Font::new_empty("Round Trip", 2_048);
    let master_id = font.default_master().id.clone();
    font.info = populated_font_info();
    font.axes.push(Axis {
        tag: Tag::from_str("wght").unwrap(),
        name: "Weight".into(),
        min: 100.0,
        default: 400.0,
        max: 900.0,
        hidden: false,
        map: vec![(100.0, 0.0), (900.0, 1_000.0)],
    });
    font.instances.push(Instance {
        name: "Bold".into(),
        location: BTreeMap::from([(Tag::from_str("wght").unwrap(), 700.0)]),
        style_name: "Bold".into(),
        family_name: Some("Round Trip Display".into()),
        active: true,
        is_variable_named_instance: true,
        user_data: json!({"instance": 1}),
    });
    {
        let master = &mut font.masters[0];
        master.location = BTreeMap::from([(Tag::from_str("wght").unwrap(), 400.0)]);
        master.metrics = MasterMetrics {
            ascender: 1_600.0,
            descender: -448.0,
            x_height: 1_000.0,
            cap_height: 1_450.0,
            italic_angle: -10.0,
            hhea_ascender: Some(1_700.0),
            hhea_descender: Some(-500.0),
            hhea_line_gap: Some(100.0),
            typo_ascender: Some(1_600.0),
            typo_descender: Some(-448.0),
            typo_line_gap: Some(0.0),
            win_ascent: Some(1_800.0),
            win_descent: Some(500.0),
            vhea_ascender: Some(1_000.0),
            vhea_descender: Some(-1_000.0),
            vhea_line_gap: Some(50.0),
            default_vertical_origin: Some(1_800.0),
            default_vertical_advance: Some(2_048.0),
        };
        master.alignment_zones.push(Zone {
            position: 0.0,
            size: -20.0,
        });
        master.stems.push(Stem {
            name: "H".into(),
            horizontal: true,
            width: 80.0,
        });
        master.guides.push(Guide {
            x: 10.0,
            y: 20.0,
            angle: 90.0,
            name: Some("master guide".into()),
            locked: true,
            color: Some("#ff00ff".into()),
        });
        master.kerning_h.0.insert(
            (
                KernSide::Glyph("A".into()),
                KernSide::Group("right.V".into()),
            ),
            -50.0,
        );
        master.kerning_v.0.insert(
            (KernSide::Glyph("A".into()), KernSide::Glyph("A".into())),
            20.0,
        );
        master.private_dict = PsPrivate {
            blue_values: vec![-20.0, 0.0, 1_450.0, 1_470.0],
            other_blues: vec![-250.0, -230.0],
            family_blues: vec![-20.0, 0.0],
            family_other_blues: vec![-250.0, -230.0],
            blue_scale: Some(0.039_625),
            blue_shift: Some(7.0),
            blue_fuzz: Some(1.0),
            std_hw: Some(80.0),
            std_vw: Some(90.0),
            stem_snap_h: vec![80.0, 100.0],
            stem_snap_v: vec![90.0, 110.0],
            force_bold: Some(false),
            language_group: Some(1),
            other: BTreeMap::from([("ExpansionFactor".into(), json!(0.06))]),
        };
        master.tt = TtMasterData {
            cvt: vec![1, -2],
            fpgm: vec![0xb0, 0x01],
            prep: vec![0x40],
            max_stack_elements: Some(32),
            max_storage: Some(4),
            max_function_defs: Some(2),
            max_instruction_defs: Some(1),
            max_twilight_points: Some(8),
            max_zones: Some(2),
            max_size_of_instructions: Some(64),
        };
        master.user_data = json!({"master": true});
    }

    let mut glyph = Glyph::new("A");
    glyph.unicodes = vec![0x41];
    glyph.variation_selectors = vec![(0x41, 0xfe0f)];
    glyph.production_name = Some("A.prod".into());
    glyph.category = Some("Letter".into());
    glyph.sub_category = Some("Uppercase".into());
    glyph.script = Some("latin".into());
    glyph.color_label = Some(4);
    glyph.tags.insert("featured".into());
    glyph.note = "A note".into();
    glyph.kern_group_side1 = Some("public.kern1.A".into());
    glyph.kern_group_side2 = Some("public.kern2.A".into());
    glyph.metrics_keys.left = Some("=H".into());
    glyph.metrics_keys.right = Some("=O".into());
    glyph.metrics_keys.width = Some("=H+20".into());
    glyph.metrics_keys.vertical_origin = Some("=A".into());
    glyph.metrics_keys.vertical_width = Some("=A+100".into());
    glyph.locked = true;
    glyph.user_data = json!({"glyph": "A"});
    glyph.layers.push(Layer::new_master(master_id.clone()));
    {
        let layer = glyph.master_layer_mut(&master_id).unwrap();
        layer.name = "Regular".into();
        layer.width = 1_200.0;
        layer.vertical_width = Some(2_048.0);
        layer.vertical_origin = Some(1_800.0);
        layer.contours.push(Contour {
            closed: true,
            points: vec![Point {
                x: 10.0,
                y: 20.0,
                kind: PointKind::Line,
                smooth: true,
                name: Some("origin".into()),
                tt_index: TtIndex::Index(3),
            }],
            name: Some("outer".into()),
            id: Some("contour-1".into()),
        });
        layer.components.push(Component {
            base: "acute".into(),
            transform: Affine::new([1.0, 0.1, 0.2, 1.0, 100.0, 200.0]),
            anchor: Some("top".into()),
            auto_align: true,
            use_my_metrics: true,
            round_to_grid: true,
            smart: Some(BTreeMap::from([("height".into(), 0.5)])),
            point_match: Some((1, 2)),
        });
        layer.anchors.push(Anchor {
            name: "top_2".into(),
            x: 600.0,
            y: 1_400.0,
        });
        layer.guides.push(Guide::default());
        layer.background_image = Some(Image {
            file_name: "reference.png".into(),
            transform: Affine::translate((3.0, 4.0)),
            color: Some("#fff".into()),
            data: Some(vec![137, 80, 78, 71]),
        });
        layer.hints.h_stems.push(StemHint {
            start: 0.0,
            width: 80.0,
            ranges: vec![(0.0, 500.0)],
        });
        layer.hints.v_stems.push(StemHint {
            start: 10.0,
            width: 90.0,
            ranges: vec![(20.0, 600.0)],
        });
        layer.hints.d_stems.push(DiagonalHint {
            left: (0.0, 0.0),
            right: (80.0, 0.0),
            unit: (0.707, 0.707),
        });
        layer.hints.flex = true;
        layer.hints.hint_masks.push(HintMask {
            contour: 0,
            point: 0,
            mask: vec![0x80],
        });
        layer.tt_instructions = Some(vec![0xb0, 0x00]);
        layer.color_layers = Some(vec![ColorLayer {
            palette_index: Some(2),
            contours: vec![],
            components: vec![],
            paint: Some(json!({"format": 10})),
        }]);
        layer.user_data = json!({"layer": true});
    }
    font.insert_glyph(glyph).unwrap();
    font.kerning_groups.side1.insert(
        "public.kern1.A".into(),
        [GlyphName::from("A")].into_iter().collect(),
    );
    font.kerning_groups.side2.insert(
        "public.kern2.A".into(),
        [GlyphName::from("A")].into_iter().collect(),
    );
    font.features = Features {
        prefixes: vec![FeatureBlock {
            tag: "Languagesystems".into(),
            code: "languagesystem DFLT dflt;".into(),
            automatic: false,
            disabled: false,
            notes: "prefix".into(),
        }],
        classes: vec![FeatureClass {
            name: "Uppercase".into(),
            code: "A B C".into(),
            automatic: true,
        }],
        features: vec![FeatureBlock {
            tag: "kern".into(),
            code: "pos A V -50;".into(),
            automatic: false,
            disabled: true,
            notes: "feature".into(),
        }],
        source_of_truth: FeaturesSource::ImportedTables,
    };
    font.bitmap_strikes.push(BitmapStrike {
        ppem: 16,
        depth: 1,
        ascent: 13,
        descent: 3,
        properties: BTreeMap::from([("FOUNDRY".into(), BdfValue::Str("FW".into()))]),
        glyphs: BTreeMap::from([(
            "A".into(),
            BitmapGlyph {
                bitmap: Bitmap {
                    origin_x: 0,
                    origin_y: 12,
                    width: 8,
                    height: 1,
                    depth: 1,
                    rows: vec![0xaa],
                },
                advance: 9,
                vertical_advance: Some(16),
                refs: vec![BitmapRef {
                    base: "acute".into(),
                    dx: 1,
                    dy: 2,
                }],
            },
        )]),
    });
    font.cid = Some(CidInfo {
        registry: "Adobe".into(),
        ordering: "Identity".into(),
        supplement: 0,
        cid_map: BTreeMap::from([(65, "A".into())]),
    });
    font.guides.push(Guide::default());
    font.extra
        .raw_tables
        .insert(Tag::from_str("TEST").unwrap(), vec![0, 1, 2, 253, 254, 255]);
    font.extra.source = Some(SourceProvenance {
        path: Some("source.otf".into()),
        format: "OpenType".into(),
        sha256: "abc123".into(),
        table_tags: vec![Tag::from_str("CFF ").unwrap()],
        original_bytes: Some(Arc::new(vec![1, 2, 3])),
    });
    font.extra.lib.insert("opaque".into(), json!([1, 2, 3]));
    font.user_data = json!({"font": true});

    let serialized = serde_json::to_value(&font).unwrap();
    assert_eq!(serialized["extra"]["raw_tables"]["TEST"], "AAEC/f7/");
    assert!(serialized["extra"]["source"]
        .get("original_bytes")
        .is_none());

    let mut expected = font.clone();
    expected.extra.source.as_mut().unwrap().original_bytes = None;
    let decoded: Font = serde_json::from_value(serialized).unwrap();
    assert_eq!(decoded, expected);
}

#[test]
fn latin1_tag_round_trips_as_text_and_as_a_raw_table_key() {
    let tag = Tag([0x80, b'A', b'B', b'C']);

    assert_eq!(tag.to_string().parse::<Tag>().unwrap(), tag);

    let extra = Extra {
        raw_tables: BTreeMap::from([(tag, vec![1, 2, 3])]),
        ..Extra::default()
    };
    let json = serde_json::to_string(&extra).unwrap();
    let decoded: Extra = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, extra);
}

#[test]
fn tags_reject_non_latin1_or_non_four_character_text() {
    assert!("ABCDx".parse::<Tag>().is_err());
    assert!("ABCĀ".parse::<Tag>().is_err());
}

fn populated_font_info() -> FontInfo {
    FontInfo {
        family_name: "Round Trip".into(),
        style_name: "Regular".into(),
        units_per_em: 2_048,
        version_major: 2,
        version_minor: 7,
        copyright: Some("Copyright".into()),
        trademark: Some("Trademark".into()),
        note: Some("Font note".into()),
        open_type_name_designer: Some("Designer".into()),
        open_type_name_designer_url: Some("https://designer.example".into()),
        open_type_name_manufacturer: Some("Manufacturer".into()),
        open_type_name_manufacturer_url: Some("https://manufacturer.example".into()),
        open_type_name_license: Some("License".into()),
        open_type_name_license_url: Some("https://license.example".into()),
        open_type_name_version: Some("Version 2.007".into()),
        open_type_name_unique_id: Some("RoundTrip-Regular-2.007".into()),
        open_type_name_description: Some("Description".into()),
        open_type_name_preferred_family_name: Some("Round Trip Preferred".into()),
        open_type_name_preferred_subfamily_name: Some("Text".into()),
        open_type_name_compatible_full_name: Some("Round Trip Regular".into()),
        open_type_name_sample_text: Some("Sample text".into()),
        open_type_name_wws_family_name: Some("Round Trip WWS".into()),
        open_type_name_wws_subfamily_name: Some("Regular".into()),
        postscript_font_name: Some("RoundTrip-Regular".into()),
        postscript_full_name: Some("Round Trip Regular".into()),
        names: vec![NameRecord {
            name_id: 1,
            language: "zh-Hans".into(),
            text: "往返".into(),
            platform: NamePlatform::Unicode,
        }],
        open_type_os2_vendor_id: Some("FWKS".into()),
        open_type_os2_weight_class: Some(400),
        open_type_os2_width_class: Some(5),
        open_type_os2_selection: vec![7],
        open_type_os2_type: vec![2],
        open_type_os2_panose: Some([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]),
        open_type_os2_family_class: Some((8, 1)),
        open_type_os2_unicode_ranges: vec![0, 1, 2],
        open_type_os2_code_page_ranges: vec![0, 17],
        open_type_os2_subscript_x_size: Some(1_000.0),
        open_type_os2_subscript_y_size: Some(1_200.0),
        open_type_os2_subscript_x_offset: Some(0.0),
        open_type_os2_subscript_y_offset: Some(200.0),
        open_type_os2_superscript_x_size: Some(1_000.0),
        open_type_os2_superscript_y_size: Some(1_200.0),
        open_type_os2_superscript_x_offset: Some(0.0),
        open_type_os2_superscript_y_offset: Some(700.0),
        open_type_os2_strikeout_size: Some(100.0),
        open_type_os2_strikeout_position: Some(600.0),
        open_type_head_created: Some("2026/09/03 12:00:00".into()),
        open_type_head_flags: vec![0, 1],
        open_type_head_lowest_rec_ppem: Some(8),
        open_type_gasp_range_records: vec![GaspRecord {
            range_max_ppem: 16,
            behavior: vec![1, 3],
        }],
        open_type_hhea_caret_slope_rise: Some(1),
        open_type_hhea_caret_slope_run: Some(0),
        open_type_hhea_caret_offset: Some(12),
        open_type_vhea_caret_slope_rise: Some(0),
        open_type_vhea_caret_slope_run: Some(1),
        open_type_vhea_caret_offset: Some(14),
        postscript_is_fixed_pitch: Some(false),
        postscript_default_width_x: Some(1_000.0),
        postscript_nominal_width_x: Some(900.0),
        postscript_underline_position: Some(-200.0),
        postscript_underline_thickness: Some(100.0),
        postscript_unique_id: Some(123_456),
        macintosh_fond_name: Some("RoundTrip".into()),
        macintosh_fond_family_id: Some(42),
        user_data: json!({"info": true}),
    }
}

#[test]
fn kerning_and_sides_have_the_document_json_shape() {
    let kerning = Kerning(BTreeMap::from([(
        (
            KernSide::Glyph("A".into()),
            KernSide::Group("public.kern2.V".into()),
        ),
        -90.0,
    )]));

    assert_eq!(
        serde_json::to_value(kerning).unwrap(),
        json!([{
            "left": {"glyph": "A"},
            "right": {"group": "public.kern2.V"},
            "value": -90.0
        }])
    );
}

#[test]
fn other_name_platform_round_trips_without_changing_existing_variants() {
    let other = NamePlatform::Other(4);
    assert_eq!(
        serde_json::from_value::<NamePlatform>(serde_json::to_value(other).unwrap()).unwrap(),
        other
    );
    assert_eq!(
        serde_json::from_value::<NamePlatform>(json!("Unicode")).unwrap(),
        NamePlatform::Unicode
    );
}
