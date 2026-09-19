mod common;

use std::sync::OnceLock;

use fc_compile::{export, ExportFormat, ExportOptions};
use fc_model::{FeaturesSource, Font, Tag};
use read_fonts::{types::GlyphId16, FontRef};
use write_fonts::{
    tables::{base::*, gdef::*, gpos::*, layout::*, variations::ItemVariationStore},
    validate::Validate,
    FontWrite,
};

fn imported_font() -> Font {
    static FONT: OnceLock<Font> = OnceLock::new();
    FONT.get_or_init(|| {
        let report = export(&common::scratch_font(), &|_| 0, &ExportOptions::default())
            .expect("compile source");
        fc_sfnt::read_bytes(&report.bytes, &fc_sfnt::ReadOptions::default())
            .expect("import source")
            .fonts
            .remove(0)
    })
    .clone()
}

fn options() -> ExportOptions {
    ExportOptions {
        format: ExportFormat::Otf,
        ..ExportOptions::default()
    }
}

fn with_table(tag: [u8; 4], table: &(impl FontWrite + Validate)) -> Font {
    let mut font = imported_font();
    font.features.source_of_truth = FeaturesSource::ImportedTables;
    font.extra.raw_tables.insert(
        Tag(tag),
        write_fonts::dump_table(table).expect("encode layout table"),
    );
    font
}

fn assert_rejected(tag: [u8; 4], table: &(impl FontWrite + Validate), detail: &str) {
    let font = with_table(tag, table);
    let error = export(&font, &|_| 0, &options())
        .expect_err("incompatible layout must fail OTF export")
        .to_string();
    assert!(error.contains(detail), "{error}");
    assert!(error.contains("disable preservation"), "{error}");
    // This incompatibility is specific to conversion to CFF. Existing TTF
    // preservation remains available, and disabling raw preservation is useful.
    export(&font, &|_| 0, &ExportOptions::default()).expect("export original TTF layout");
    export(
        &font,
        &|_| 0,
        &ExportOptions {
            preserve_untouched_tables: false,
            ..options()
        },
    )
    .expect("export CFF after explicitly disabling imported table preservation");
}

fn assert_preserved(tag: [u8; 4], table: &(impl FontWrite + Validate)) {
    let font = with_table(tag, table);
    let report = export(&font, &|_| 0, &options()).expect("export coordinate layout");
    let face = FontRef::new(&report.bytes).expect("parse exported OTF");
    assert_eq!(
        face.table_data(read_fonts::types::Tag::new(&tag))
            .expect("retained layout")
            .as_bytes(),
        font.extra.raw_tables[&Tag(tag)]
    );
}

fn coverage(gid: u16) -> CoverageTable {
    CoverageFormat1::new(vec![GlyphId16::new(gid)]).into()
}

fn point_anchor() -> AnchorTable {
    AnchorFormat2::new(100, 200, 3).into()
}

fn coordinate_anchor() -> AnchorTable {
    AnchorFormat1::new(100, 200).into()
}

fn device() -> DeviceOrVariationIndex {
    Device::new(12, 12, &[1]).into()
}

fn variable_device() -> DeviceOrVariationIndex {
    VariationIndex::new(0, 0).into()
}

fn gpos(lookup: PositionLookup) -> Gpos {
    Gpos::new(
        ScriptList::default(),
        FeatureList::default(),
        PositionLookupList::new(vec![lookup]),
    )
}

fn cursive(anchor: AnchorTable) -> CursivePosFormat1 {
    CursivePosFormat1::new(coverage(2), vec![EntryExitRecord::new(Some(anchor), None)])
}

fn mark_base(mark: AnchorTable, base: AnchorTable) -> MarkBasePosFormat1 {
    MarkBasePosFormat1::new(
        coverage(3),
        coverage(2),
        MarkArray::new(vec![MarkRecord::new(0, mark)]),
        BaseArray::new(vec![BaseRecord::new(vec![Some(base)])]),
    )
}

#[test]
fn cff_export_accepts_fonts_without_imported_layout() {
    let mut font = common::scratch_font();
    font.masters[0].kerning_h.0.clear();
    export(&font, &|_| 0, &options()).expect("export without layout tables");
}

#[test]
fn gpos_cursive_point_anchors_fail_with_recovery_instructions() {
    let table = gpos(PositionLookup::Cursive(Lookup::new(
        LookupFlag::empty(),
        vec![cursive(point_anchor())],
    )));
    assert_rejected(*b"GPOS", &table, "GPOS Anchor format 2");
}

#[test]
fn gpos_extension_point_anchors_are_checked() {
    let extension = ExtensionSubtable::MarkToBase(ExtensionPosFormat1::new(
        4,
        mark_base(coordinate_anchor(), point_anchor()),
    ));
    let table = gpos(PositionLookup::Extension(Lookup::new(
        LookupFlag::empty(),
        vec![extension],
    )));
    assert_rejected(*b"GPOS", &table, "GPOS Anchor format 2");
}

#[test]
fn gpos_mark_and_base_point_anchors_are_checked() {
    for (mark, base) in [
        (point_anchor(), coordinate_anchor()),
        (coordinate_anchor(), point_anchor()),
    ] {
        let table = gpos(PositionLookup::MarkToBase(Lookup::new(
            LookupFlag::empty(),
            vec![mark_base(mark, base)],
        )));
        assert_rejected(*b"GPOS", &table, "GPOS Anchor format 2");
    }
}

#[test]
fn gpos_ligature_and_mark2_point_anchors_are_checked() {
    let ligature = MarkLigPosFormat1::new(
        coverage(3),
        coverage(2),
        MarkArray::new(vec![MarkRecord::new(0, coordinate_anchor())]),
        LigatureArray::new(vec![LigatureAttach::new(vec![ComponentRecord::new(vec![
            Some(point_anchor()),
        ])])]),
    );
    assert_rejected(
        *b"GPOS",
        &gpos(PositionLookup::MarkToLig(Lookup::new(
            LookupFlag::empty(),
            vec![ligature],
        ))),
        "GPOS Anchor format 2",
    );
    let mark = MarkMarkPosFormat1::new(
        coverage(3),
        coverage(2),
        MarkArray::new(vec![MarkRecord::new(0, coordinate_anchor())]),
        Mark2Array::new(vec![Mark2Record::new(vec![Some(point_anchor())])]),
    );
    assert_rejected(
        *b"GPOS",
        &gpos(PositionLookup::MarkToMark(Lookup::new(
            LookupFlag::empty(),
            vec![mark],
        ))),
        "GPOS Anchor format 2",
    );
}

#[test]
fn gpos_coordinate_and_device_anchors_are_preserved() {
    let table = gpos(PositionLookup::MarkToBase(Lookup::new(
        LookupFlag::empty(),
        vec![mark_base(
            coordinate_anchor(),
            AnchorFormat3::new(100, 200, Some(device()), None).into(),
        )],
    )));
    assert_preserved(*b"GPOS", &table);
}

#[test]
fn gdef_attachment_points_and_point_carets_are_rejected() {
    let attached = Gdef {
        attach_list: Some(AttachList::new(
            coverage(2),
            vec![AttachPoint::new(vec![0, 3])],
        ))
        .into(),
        ..Gdef::default()
    };
    assert_rejected(*b"GDEF", &attached, "GDEF attachment points");
    let carets = Gdef {
        lig_caret_list: Some(LigCaretList::new(
            coverage(2),
            vec![LigGlyph::new(vec![CaretValueFormat2::new(1).into()])],
        ))
        .into(),
        ..Gdef::default()
    };
    assert_rejected(*b"GDEF", &carets, "GDEF CaretValue format 2");
}

#[test]
fn gdef_coordinate_carets_device_adjustments_and_empty_attachment_list_survive() {
    let table = Gdef {
        attach_list: Some(AttachList::new(coverage(2), vec![AttachPoint::new(vec![])])).into(),
        lig_caret_list: Some(LigCaretList::new(
            coverage(2),
            vec![LigGlyph::new(vec![
                CaretValueFormat1::new(200).into(),
                CaretValueFormat3::new(300, device()).into(),
            ])],
        ))
        .into(),
        ..Gdef::default()
    };
    assert_preserved(*b"GDEF", &table);
}

fn base_with_script(script: BaseScript, vertical: bool) -> Base {
    let axis = Axis::new(
        Some(BaseTagList::new(vec![read_fonts::types::Tag::new(b"romn")])),
        BaseScriptList::new(vec![BaseScriptRecord::new(
            read_fonts::types::Tag::new(b"latn"),
            script,
        )]),
    );
    if vertical {
        Base::new(None, Some(axis))
    } else {
        Base::new(Some(axis), None)
    }
}

fn point_coord() -> BaseCoord {
    BaseCoordFormat2::new(100, 2, 3).into()
}

#[test]
fn base_point_coordinates_are_rejected_on_both_axes() {
    for vertical in [false, true] {
        let table = base_with_script(
            BaseScript::new(Some(BaseValues::new(0, vec![point_coord()])), None, vec![]),
            vertical,
        );
        assert_rejected(*b"BASE", &table, "BASE BaseCoord format 2");
    }
}

#[test]
fn base_language_and_feature_min_max_point_coordinates_are_checked() {
    let min_max = MinMax::new(
        None,
        None,
        vec![FeatMinMaxRecord::new(
            read_fonts::types::Tag::new(b"kern"),
            None,
            Some(point_coord()),
        )],
    );
    for script in [
        BaseScript::new(None, Some(min_max.clone()), vec![]),
        BaseScript::new(
            None,
            None,
            vec![BaseLangSysRecord::new(
                read_fonts::types::Tag::new(b"ENG "),
                min_max,
            )],
        ),
    ] {
        assert_rejected(
            *b"BASE",
            &base_with_script(script, false),
            "BASE BaseCoord format 2",
        );
    }
}

#[test]
fn base_coordinate_baselines_and_device_adjustments_are_preserved() {
    let table = base_with_script(
        BaseScript::new(
            Some(BaseValues::new(0, vec![BaseCoordFormat1::new(100).into()])),
            Some(MinMax::new(
                Some(BaseCoordFormat3::new(-100, Some(device())).into()),
                None,
                vec![],
            )),
            vec![],
        ),
        false,
    );
    assert_preserved(*b"BASE", &table);
}

#[test]
fn uninstantiated_layout_variation_stores_are_rejected() {
    assert_rejected(
        *b"GDEF",
        &Gdef {
            item_var_store: Some(ItemVariationStore::default()).into(),
            ..Gdef::default()
        },
        "GDEF ItemVariationStore",
    );
    assert_rejected(
        *b"BASE",
        &Base {
            item_var_store: Some(ItemVariationStore::default()).into(),
            ..Base::default()
        },
        "BASE ItemVariationStore",
    );
}

#[test]
fn gpos_and_gsub_feature_variations_require_instantiation() {
    assert_rejected(
        *b"GPOS",
        &Gpos {
            feature_variations: Some(FeatureVariations::default()).into(),
            ..Gpos::default()
        },
        "GPOS FeatureVariations",
    );
    assert_rejected(
        *b"GSUB",
        &write_fonts::tables::gsub::Gsub {
            feature_variations: Some(FeatureVariations::default()).into(),
            ..write_fonts::tables::gsub::Gsub::default()
        },
        "GSUB FeatureVariations",
    );
}

#[test]
fn variable_anchor_adjustments_are_not_confused_with_ordinary_devices() {
    let table = gpos(PositionLookup::Cursive(Lookup::new(
        LookupFlag::empty(),
        vec![cursive(
            AnchorFormat3::new(100, 200, None, Some(variable_device())).into(),
        )],
    )));
    assert_rejected(*b"GPOS", &table, "GPOS VariationIndex");
}

#[test]
fn positioning_values_check_device_offsets_in_their_parent_table() {
    for adjustment in [device(), variable_device()] {
        let is_variable = matches!(adjustment, DeviceOrVariationIndex::VariationIndex(_));
        let mut value = ValueRecord::new().with_x_advance(-40);
        value.x_advance_device = Some(adjustment).into();
        let table = gpos(PositionLookup::Pair(Lookup::new(
            LookupFlag::empty(),
            vec![PairPosFormat1::new(
                coverage(2),
                vec![PairSet::new(vec![PairValueRecord::new(
                    GlyphId16::new(3),
                    value,
                    ValueRecord::default(),
                )])],
            )
            .into()],
        )));
        if is_variable {
            assert_rejected(*b"GPOS", &table, "GPOS VariationIndex");
        } else {
            assert_preserved(*b"GPOS", &table);
        }
    }
}

#[test]
fn malformed_gdef_attachment_offsets_fail_instead_of_bypassing_the_guard() {
    let mut font = with_table(*b"GDEF", &Gdef::default());
    font.extra.raw_tables.get_mut(&Tag(*b"GDEF")).unwrap()[6..8]
        .copy_from_slice(&u16::MAX.to_be_bytes());
    let error = export(&font, &|_| 0, &options())
        .expect_err("malformed preserved GDEF")
        .to_string();
    assert!(error.contains("failed to parse GDEF"), "{error}");
}

#[test]
fn malformed_gpos_extension_offsets_fail_instead_of_becoming_empty_extensions() {
    let table = gpos(PositionLookup::Extension(Lookup::new(
        LookupFlag::empty(),
        vec![ExtensionSubtable::Cursive(ExtensionPosFormat1::new(
            3,
            cursive(coordinate_anchor()),
        ))],
    )));
    let mut font = with_table(*b"GPOS", &table);
    let bytes = font.extra.raw_tables.get_mut(&Tag(*b"GPOS")).unwrap();
    let list_offset = u16::from_be_bytes([bytes[8], bytes[9]]) as usize;
    let lookup_offset =
        list_offset + u16::from_be_bytes([bytes[list_offset + 2], bytes[list_offset + 3]]) as usize;
    bytes[lookup_offset + 6..lookup_offset + 8].copy_from_slice(&u16::MAX.to_be_bytes());
    let error = export(&font, &|_| 0, &options())
        .expect_err("malformed preserved extension")
        .to_string();
    assert!(error.contains("failed to parse GPOS"), "{error}");
}

#[test]
fn truncated_declared_arrays_do_not_hide_point_dependent_layout() {
    let table = Gdef {
        attach_list: Some(AttachList::new(
            coverage(2),
            vec![AttachPoint::new(vec![1])],
        ))
        .into(),
        ..Gdef::default()
    };
    let mut font = with_table(*b"GDEF", &table);
    let bytes = font.extra.raw_tables.get_mut(&Tag(*b"GDEF")).unwrap();
    let attach_offset = u16::from_be_bytes([bytes[6], bytes[7]]) as usize;
    bytes[attach_offset + 2..attach_offset + 4].copy_from_slice(&u16::MAX.to_be_bytes());
    let error = export(&font, &|_| 0, &options()).unwrap_err().to_string();
    assert!(error.contains("failed to parse GDEF"), "{error}");

    let table = gpos(PositionLookup::Cursive(Lookup::new(
        LookupFlag::empty(),
        vec![cursive(point_anchor())],
    )));
    let mut font = with_table(*b"GPOS", &table);
    let bytes = font.extra.raw_tables.get_mut(&Tag(*b"GPOS")).unwrap();
    let list = u16::from_be_bytes([bytes[8], bytes[9]]) as usize;
    let lookup = list + u16::from_be_bytes([bytes[list + 2], bytes[list + 3]]) as usize;
    let subtable = lookup + u16::from_be_bytes([bytes[lookup + 6], bytes[lookup + 7]]) as usize;
    bytes[subtable + 4..subtable + 6].copy_from_slice(&u16::MAX.to_be_bytes());
    let error = export(&font, &|_| 0, &options()).unwrap_err().to_string();
    assert!(error.contains("failed to parse GPOS"), "{error}");
}

#[test]
fn truncated_base_value_arrays_are_errors_instead_of_empty_coordinates() {
    let table = base_with_script(
        BaseScript::new(Some(BaseValues::new(0, vec![point_coord()])), None, vec![]),
        false,
    );
    let mut font = with_table(*b"BASE", &table);
    let bytes = font.extra.raw_tables.get_mut(&Tag(*b"BASE")).unwrap();
    let axis = u16::from_be_bytes([bytes[4], bytes[5]]) as usize;
    let list = axis + u16::from_be_bytes([bytes[axis + 2], bytes[axis + 3]]) as usize;
    let script = list + u16::from_be_bytes([bytes[list + 6], bytes[list + 7]]) as usize;
    let values = script + u16::from_be_bytes([bytes[script], bytes[script + 1]]) as usize;
    bytes[values + 2..values + 4].copy_from_slice(&u16::MAX.to_be_bytes());
    let error = export(&font, &|_| 0, &options()).unwrap_err().to_string();
    assert!(error.contains("failed to parse BASE"), "{error}");
}

#[test]
fn truncated_versioned_headers_do_not_hide_variation_dependencies() {
    for (tag, version, length) in [
        (*b"GPOS", [0, 1, 0, 1], 10),
        (*b"GSUB", [0, 1, 0, 1], 10),
        (*b"GDEF", [0, 1, 0, 3], 12),
        (*b"BASE", [0, 1, 0, 1], 8),
    ] {
        let mut font = imported_font();
        let mut header = vec![0; length];
        header[..4].copy_from_slice(&version);
        font.extra.raw_tables.insert(Tag(tag), header);
        let error = export(&font, &|_| 0, &options()).unwrap_err().to_string();
        let expected = format!("failed to parse {}", Tag(tag));
        assert!(error.contains(&expected), "{error}");
    }
}
