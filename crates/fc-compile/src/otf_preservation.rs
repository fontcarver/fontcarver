//! Guards for retained layout whose semantics depend on replaced outlines.

use read_fonts::{
    tables::{
        base::{BaseCoord, MinMax},
        gdef::CaretValue,
        gpos::{
            AnchorTable, CursivePosFormat1, ExtensionSubtable, MarkArray, MarkBasePosFormat1,
            MarkLigPosFormat1, MarkMarkPosFormat1, PairPos, PositionLookup, SinglePos, ValueRecord,
        },
        layout::DeviceOrVariationIndex,
    },
    types::Tag,
    FontData, FontRef, MinByteRange, ReadError, TableProvider,
};

use crate::Error;

/// Refuse layout that cannot survive the conversion to static CFF outlines.
///
/// Point numbers are not stable when quadratic contours become cubic contours
/// or components are flattened. Retained variation adjustments also cannot be
/// interpreted after their font axes are removed without first instantiating
/// them. Coordinate-based layout, including ordinary Device tables, is safe.
/// MATH has only glyph IDs and coordinate/device values, not contour indices;
/// its glyph-order requirements are enforced by the existing preservation merge.
pub(crate) fn check_geometry_independent_layout(sfnt: &[u8]) -> Result<(), Error> {
    let face = checked("OTF source sfnt", FontRef::new(sfnt))?;
    if face.table_data(Tag::new(b"GPOS")).is_some() {
        let gpos = checked_table("GPOS", face.gpos())?;
        check_header("GPOS", gpos.offset_data(), &[(0x10000, 10), (0x10001, 14)])?;
        if let Some(variations) = gpos.feature_variations() {
            checked("GPOS", variations)?;
            return Err(variable_layout("GPOS", "FeatureVariations"));
        }
        for lookup in checked_table("GPOS", gpos.lookup_list())?.lookups().iter() {
            check_lookup(checked("GPOS", lookup)?)?;
        }
    }
    if face.table_data(Tag::new(b"GSUB")).is_some() {
        let gsub = checked_table("GSUB", face.gsub())?;
        check_header("GSUB", gsub.offset_data(), &[(0x10000, 10), (0x10001, 14)])?;
        if let Some(variations) = gsub.feature_variations() {
            checked("GSUB", variations)?;
            return Err(variable_layout("GSUB", "FeatureVariations"));
        }
    }
    if face.table_data(Tag::new(b"GDEF")).is_some() {
        let gdef = checked_table("GDEF", face.gdef())?;
        check_header(
            "GDEF",
            gdef.offset_data(),
            &[(0x10000, 12), (0x10002, 14), (0x10003, 18)],
        )?;
        if let Some(store) = gdef.item_var_store() {
            checked("GDEF", store)?;
            return Err(variable_layout("GDEF", "ItemVariationStore"));
        }
        if let Some(attach_list) = gdef.attach_list() {
            for points in checked_table("GDEF", attach_list)?.attach_points().iter() {
                if checked_table("GDEF", points)?.point_count() != 0 {
                    return Err(point_layout("GDEF", "attachment points"));
                }
            }
        }
        if let Some(carets) = gdef.lig_caret_list() {
            for glyph in checked_table("GDEF", carets)?.lig_glyphs().iter() {
                for caret in checked_table("GDEF", glyph)?.caret_values().iter() {
                    match checked_table("GDEF", caret)? {
                        CaretValue::Format2(_) => {
                            return Err(point_layout("GDEF", "CaretValue format 2"));
                        }
                        CaretValue::Format3(caret) => {
                            check_device("GDEF", Some(caret.device()))?;
                        }
                        CaretValue::Format1(_) => {}
                    }
                }
            }
        }
    }
    if face.table_data(Tag::new(b"BASE")).is_some() {
        let base = checked_table("BASE", face.base())?;
        check_header("BASE", base.offset_data(), &[(0x10000, 8), (0x10001, 12)])?;
        if let Some(store) = base.item_var_store() {
            checked("BASE", store)?;
            return Err(variable_layout("BASE", "ItemVariationStore"));
        }
        for axis in [base.horiz_axis(), base.vert_axis()].into_iter().flatten() {
            let axis = checked_table("BASE", axis)?;
            let scripts = checked_table("BASE", axis.base_script_list())?;
            for record in scripts.base_script_records() {
                let script = checked_table("BASE", record.base_script(scripts.offset_data()))?;
                if let Some(values) = script.base_values() {
                    for coord in checked_table("BASE", values)?.base_coords().iter() {
                        check_base_coord(Some(coord))?;
                    }
                }
                if let Some(min_max) = script.default_min_max() {
                    check_min_max(checked("BASE", min_max)?)?;
                }
                for lang in script.base_lang_sys_records() {
                    check_min_max(checked("BASE", lang.min_max(script.offset_data()))?)?;
                }
            }
        }
    }
    Ok(())
}

fn check_lookup(lookup: PositionLookup<'_>) -> Result<(), Error> {
    macro_rules! visit {
        ($lookup:expr, $check:expr) => {{
            validate_table("GPOS", &$lookup)?;
            for subtable in $lookup.subtables().iter() {
                $check(checked("GPOS", subtable)?)?;
            }
            Ok(())
        }};
    }
    match lookup {
        PositionLookup::Single(lookup) => visit!(lookup, check_single),
        PositionLookup::Pair(lookup) => visit!(lookup, check_pair),
        PositionLookup::Cursive(lookup) => visit!(lookup, check_cursive),
        PositionLookup::MarkToBase(lookup) => visit!(lookup, check_mark_base),
        PositionLookup::MarkToLig(lookup) => visit!(lookup, check_mark_ligature),
        PositionLookup::MarkToMark(lookup) => visit!(lookup, check_mark_mark),
        PositionLookup::Extension(lookup) => visit!(lookup, check_extension),
        // Context subtables contain lookup indices, not anchors. Every lookup
        // in the list is independently checked, including unreferenced ones.
        PositionLookup::Contextual(lookup) => visit!(lookup, |_| Ok::<_, Error>(())),
        PositionLookup::ChainContextual(lookup) => visit!(lookup, |_| Ok::<_, Error>(())),
    }
}

fn check_extension(extension: ExtensionSubtable<'_>) -> Result<(), Error> {
    // Inspect each extension's own discriminant. The convenience subtables()
    // API uses the first valid extension type and can hide malformed offsets.
    match extension {
        ExtensionSubtable::Single(ext) => check_single(checked("GPOS", ext.extension())?),
        ExtensionSubtable::Pair(ext) => check_pair(checked("GPOS", ext.extension())?),
        ExtensionSubtable::Cursive(ext) => check_cursive(checked("GPOS", ext.extension())?),
        ExtensionSubtable::MarkToBase(ext) => check_mark_base(checked("GPOS", ext.extension())?),
        ExtensionSubtable::MarkToLig(ext) => check_mark_ligature(checked("GPOS", ext.extension())?),
        ExtensionSubtable::MarkToMark(ext) => check_mark_mark(checked("GPOS", ext.extension())?),
        ExtensionSubtable::Contextual(ext) => checked("GPOS", ext.extension()).map(|_| ()),
        ExtensionSubtable::ChainContextual(ext) => checked("GPOS", ext.extension()).map(|_| ()),
    }
}

fn check_single(subtable: SinglePos<'_>) -> Result<(), Error> {
    validate_table("GPOS", &subtable)?;
    match subtable {
        SinglePos::Format1(single) => check_value(&single.value_record(), single.offset_data()),
        SinglePos::Format2(single) => {
            for value in single.value_records().iter() {
                check_value(&checked("GPOS", value)?, single.offset_data())?;
            }
            Ok(())
        }
    }
}

fn check_pair(subtable: PairPos<'_>) -> Result<(), Error> {
    validate_table("GPOS", &subtable)?;
    match subtable {
        PairPos::Format1(pair) => {
            for set in pair.pair_sets().iter() {
                let set = checked_table("GPOS", set)?;
                for record in set.pair_value_records().iter() {
                    let record = checked("GPOS", record)?;
                    check_value(record.value_record1(), set.offset_data())?;
                    check_value(record.value_record2(), set.offset_data())?;
                }
            }
        }
        PairPos::Format2(pair) => {
            for class1 in pair.class1_records().iter() {
                for class2 in checked("GPOS", class1)?.class2_records().iter() {
                    let class2 = checked("GPOS", class2)?;
                    check_value(class2.value_record1(), pair.offset_data())?;
                    check_value(class2.value_record2(), pair.offset_data())?;
                }
            }
        }
    }
    Ok(())
}

fn check_value(value: &ValueRecord, data: FontData<'_>) -> Result<(), Error> {
    for device in [
        value.x_placement_device(data),
        value.y_placement_device(data),
        value.x_advance_device(data),
        value.y_advance_device(data),
    ] {
        check_device("GPOS", device)?;
    }
    Ok(())
}

fn check_cursive(subtable: CursivePosFormat1<'_>) -> Result<(), Error> {
    validate_table("GPOS", &subtable)?;
    for record in subtable.entry_exit_record() {
        check_anchor(record.entry_anchor(subtable.offset_data()))?;
        check_anchor(record.exit_anchor(subtable.offset_data()))?;
    }
    Ok(())
}

fn check_marks(marks: MarkArray<'_>) -> Result<(), Error> {
    validate_table("GPOS", &marks)?;
    for mark in marks.mark_records() {
        check_anchor(Some(mark.mark_anchor(marks.offset_data())))?;
    }
    Ok(())
}

fn check_mark_base(subtable: MarkBasePosFormat1<'_>) -> Result<(), Error> {
    validate_table("GPOS", &subtable)?;
    check_marks(checked("GPOS", subtable.mark_array())?)?;
    let bases = checked_table("GPOS", subtable.base_array())?;
    for base in bases.base_records().iter() {
        for anchor in checked("GPOS", base)?
            .base_anchors(bases.offset_data())
            .iter()
        {
            check_anchor(anchor)?;
        }
    }
    Ok(())
}

fn check_mark_ligature(subtable: MarkLigPosFormat1<'_>) -> Result<(), Error> {
    validate_table("GPOS", &subtable)?;
    check_marks(checked("GPOS", subtable.mark_array())?)?;
    let ligatures = checked_table("GPOS", subtable.ligature_array())?;
    for ligature in ligatures.ligature_attaches().iter() {
        let ligature = checked_table("GPOS", ligature)?;
        for component in ligature.component_records().iter() {
            for anchor in checked("GPOS", component)?
                .ligature_anchors(ligature.offset_data())
                .iter()
            {
                check_anchor(anchor)?;
            }
        }
    }
    Ok(())
}

fn check_mark_mark(subtable: MarkMarkPosFormat1<'_>) -> Result<(), Error> {
    validate_table("GPOS", &subtable)?;
    check_marks(checked("GPOS", subtable.mark1_array())?)?;
    let marks = checked_table("GPOS", subtable.mark2_array())?;
    for mark in marks.mark2_records().iter() {
        for anchor in checked("GPOS", mark)?
            .mark2_anchors(marks.offset_data())
            .iter()
        {
            check_anchor(anchor)?;
        }
    }
    Ok(())
}

fn check_anchor(anchor: Option<Result<AnchorTable<'_>, ReadError>>) -> Result<(), Error> {
    let Some(anchor) = anchor else { return Ok(()) };
    match checked_table("GPOS", anchor)? {
        AnchorTable::Format2(_) => Err(point_layout("GPOS", "Anchor format 2")),
        AnchorTable::Format3(anchor) => {
            check_device("GPOS", anchor.x_device())?;
            check_device("GPOS", anchor.y_device())
        }
        AnchorTable::Format1(_) => Ok(()),
    }
}

fn check_min_max(min_max: MinMax<'_>) -> Result<(), Error> {
    validate_table("BASE", &min_max)?;
    check_base_coord(min_max.min_coord())?;
    check_base_coord(min_max.max_coord())?;
    for feature in min_max.feat_min_max_records() {
        check_base_coord(feature.min_coord(min_max.offset_data()))?;
        check_base_coord(feature.max_coord(min_max.offset_data()))?;
    }
    Ok(())
}

fn check_base_coord(coord: Option<Result<BaseCoord<'_>, ReadError>>) -> Result<(), Error> {
    let Some(coord) = coord else { return Ok(()) };
    match checked_table("BASE", coord)? {
        BaseCoord::Format2(_) => Err(point_layout("BASE", "BaseCoord format 2")),
        BaseCoord::Format3(coord) => check_device("BASE", coord.device()),
        BaseCoord::Format1(_) => Ok(()),
    }
}

fn check_device(
    table: &'static str,
    device: Option<Result<DeviceOrVariationIndex<'_>, ReadError>>,
) -> Result<(), Error> {
    if let Some(device) = device {
        if matches!(
            checked_table(table, device)?,
            DeviceOrVariationIndex::VariationIndex(_)
        ) {
            return Err(variable_layout(table, "VariationIndex"));
        }
    }
    Ok(())
}

fn checked<T>(table: &'static str, result: Result<T, ReadError>) -> Result<T, Error> {
    result.map_err(|error| Error::parse(table, error))
}

fn check_header(
    context: &'static str,
    data: FontData<'_>,
    versions: &[(u32, usize)],
) -> Result<(), Error> {
    let version = checked(context, data.read_at::<u32>(0))?;
    let Some((_, length)) = versions.iter().find(|(known, _)| *known == version) else {
        return Err(Error::parse(context, "unsupported table version"));
    };
    // MinByteRange covers only unconditional fields. Optional variation
    // offsets must exist when the header version declares them, even if null.
    if data.len() < *length {
        return Err(Error::parse(context, ReadError::OutOfBounds));
    }
    Ok(())
}

fn checked_table<'a, T: MinByteRange<'a>>(
    context: &'static str,
    result: Result<T, ReadError>,
) -> Result<T, Error> {
    let table = checked(context, result)?;
    validate_table(context, &table)?;
    Ok(table)
}

fn validate_table<'a>(context: &'static str, table: &impl MinByteRange<'a>) -> Result<(), Error> {
    // Generated read-fonts accessors return empty slices when declared arrays
    // exceed the remaining data. Do not mistake malformed counts for no layout.
    if table.min_table_bytes().len() != table.min_byte_range().len() {
        return Err(Error::parse(context, ReadError::OutOfBounds));
    }
    Ok(())
}

fn point_layout(table: &str, structure: &str) -> Error {
    Error::InvalidSource(format!(
        "static OTF/CFF export cannot preserve {table} {structure} because it references outline \
         point indices that change during CFF conversion; rebuild the layout using coordinates, \
         or disable preservation of imported tables before exporting"
    ))
}

fn variable_layout(table: &str, structure: &str) -> Error {
    Error::InvalidSource(format!(
        "static OTF/CFF export cannot preserve {table} {structure} before variable layout is \
         instantiated; rebuild static layout from feature source, or disable preservation of \
         imported tables before exporting"
    ))
}
