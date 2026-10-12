//! Public inventory contracts exercised with real sfnt imports and hostile tables.

use std::collections::BTreeMap;

use fc_compile::{
    compile_static_with_metadata, shaping_fingerprint, CompileOptions, StaticCompile,
};
use fc_layout::{equivalent_layout, LayoutImport};
use fc_model::{FeatureBlock, FeaturesSource, Font, Glyph, Layer, Tag};

fn compile(source: &str) -> StaticCompile {
    let mut font = Font::new_empty("Layout inventory", 1000);
    let master = font.default_master().id.clone();
    for (name, unicode) in [
        (".notdef", None),
        ("a", Some(0x61)),
        ("b", Some(0x62)),
        ("a.alt", None),
        ("acute", Some(0x301)),
    ] {
        let mut glyph = Glyph::new(name);
        glyph.unicodes.extend(unicode);
        let mut layer = Layer::new_master(master.clone());
        layer.width = 500.0;
        glyph.layers.push(layer);
        font.insert_glyph(glyph).unwrap();
    }
    if !source.is_empty() {
        font.features.prefixes.push(FeatureBlock {
            code: source.into(),
            ..FeatureBlock::default()
        });
    }
    compile_static_with_metadata(&font, &master, &CompileOptions::default()).unwrap()
}

fn import(bytes: &[u8]) -> Font {
    fc_sfnt::read_bytes(bytes, &Default::default())
        .unwrap()
        .fonts
        .remove(0)
}

fn words(values: &[u16]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_be_bytes())
        .collect()
}

fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes(bytes[offset..offset + 2].try_into().unwrap())
}

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn set_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
}

fn with_table(template: &Font, tag: [u8; 4], bytes: Vec<u8>) -> Font {
    let mut font = template.clone();
    font.features.source_of_truth = FeaturesSource::ImportedTables;
    font.extra.raw_tables.insert(Tag(tag), bytes);
    font
}

fn assert_blocked(font: &Font, table: &str, reason: &str) {
    let original = font.clone();
    let fingerprint = LayoutImport::fingerprint_for_font(font);
    let inventory = LayoutImport::from_font(font);
    assert!(!inventory.can_convert(), "expected blocker: {reason}");
    assert!(inventory.decompile().is_err());
    assert!(
        inventory
            .blockers
            .iter()
            .any(|blocker| blocker.table == table && blocker.reason.contains(reason)),
        "{:?}",
        inventory.blockers
    );
    assert_eq!(inventory.fingerprint, fingerprint);
    assert_eq!(font, &original);
}

/// Minimal GSUB with one registered feature and independently selected lookups.
fn gsub(lookups: &[(u16, Vec<u8>)], roots: &[u16]) -> Vec<u8> {
    let mut bytes = words(&[1, 0, 10, 0, 0]);
    bytes.extend(words(&[1]));
    bytes.extend(b"DFLT");
    bytes.extend(words(&[8, 4, 0, 0, u16::MAX, 1, 0]));
    let feature_offset = bytes.len();
    set_u16(&mut bytes, 6, feature_offset as u16);
    bytes.extend(words(&[1]));
    bytes.extend(b"calt");
    bytes.extend(words(&[8, 0, roots.len() as u16]));
    bytes.extend(words(roots));
    let list_offset = bytes.len();
    set_u16(&mut bytes, 8, list_offset as u16);
    bytes.extend(words(&[lookups.len() as u16]));
    bytes.resize(bytes.len() + lookups.len() * 2, 0);
    for (index, (kind, subtable)) in lookups.iter().enumerate() {
        let offset = bytes.len() - list_offset;
        set_u16(&mut bytes, list_offset + 2 + index * 2, offset as u16);
        bytes.extend(words(&[*kind, 0, 1, 8]));
        bytes.extend(subtable);
    }
    bytes
}

fn single() -> (u16, Vec<u8>) {
    (1, words(&[2, 8, 1, 2, 1, 1, 1]))
}

fn contextual(reference: u16) -> (u16, Vec<u8>) {
    (5, words(&[3, 1, 1, 12, 0, reference, 1, 1, 1]))
}

#[test]
fn malformed_headers_limits_and_feature_variations_fail_closed_without_mutation() {
    let template = import(&compile("").bytes);
    let valid = gsub(&[single()], &[0]);
    assert!(LayoutImport::from_font(&with_table(&template, *b"GSUB", valid.clone())).can_convert());
    for (field, value, reason) in [
        (0, 2, "version"),
        (8, u16::MAX, "truncated"),
        (usize::from(u16_at(&valid, 8)), 4097, "resource limit"),
    ] {
        let mut bytes = valid.clone();
        set_u16(&mut bytes, field, value);
        assert_blocked(&with_table(&template, *b"GSUB", bytes), "GSUB", reason);
    }
    // Version 1.1 with a nonnull FeatureVariations offset is rejected before traversal.
    let bytes = words(&[1, 1, 0, 0, 0, 0, 14]);
    assert_blocked(
        &with_table(&template, *b"GSUB", bytes),
        "GSUB",
        "FeatureVariations",
    );
    let list = usize::from(u16_at(&valid, 8));
    let lookup = list + usize::from(u16_at(&valid, list + 2));
    let mut bytes = valid.clone();
    set_u16(&mut bytes, lookup + 2, 0x20);
    assert_blocked(
        &with_table(&template, *b"GSUB", bytes),
        "GSUB",
        "reserved flag",
    );
    for length in 0..valid.len() {
        let font = with_table(&template, *b"GSUB", valid[..length].to_vec());
        assert!(
            !LayoutImport::from_font(&font).can_convert(),
            "prefix {length}"
        );
    }
    let empty_subtable = gsub(&[(1, words(&[2, 6, 0, 1, 0]))], &[0]);
    assert_blocked(
        &with_table(&template, *b"GSUB", empty_subtable),
        "GSUB",
        "empty subtable",
    );
}

#[test]
fn cyclic_missing_and_unreachable_graph_records_are_not_silently_discarded() {
    let template = import(&compile("").bytes);
    for (bytes, reason) in [
        (gsub(&[contextual(1), contextual(0)], &[0]), "cyclic"),
        (gsub(&[contextual(1)], &[0]), "missing lookup"),
        (gsub(&[single(), single()], &[0]), "unreachable lookup"),
        (gsub(&[single()], &[1]), "lookup index out of range"),
    ] {
        assert_blocked(&with_table(&template, *b"GSUB", bytes), "GSUB", reason);
    }
    let mut bytes = gsub(&[single()], &[0]);
    // Add a second valid feature while the default language still references only calt.
    let lookup_list = bytes[usize::from(u16_at(&bytes, 8))..].to_vec();
    bytes.truncate(usize::from(u16_at(&bytes, 6)));
    bytes.extend(words(&[2]));
    bytes.extend(b"calt");
    bytes.extend(words(&[14]));
    bytes.extend(b"liga");
    bytes.extend(words(&[20, 0, 1, 0, 0, 1, 0]));
    let lookup_offset = bytes.len() as u16;
    set_u16(&mut bytes, 8, lookup_offset);
    bytes.extend(lookup_list);
    assert_blocked(
        &with_table(&template, *b"GSUB", bytes),
        "GSUB",
        "unreachable feature",
    );
    let mut chain: Vec<_> = (1..67).map(contextual).collect();
    chain.push(single());
    assert_blocked(
        &with_table(&template, *b"GSUB", gsub(&chain, &[0])),
        "GSUB",
        "recursion limit",
    );
}

#[test]
fn empty_or_unsorted_language_records_and_aliased_feature_expansion_are_blocked() {
    let template = import(&compile("").bytes);
    let valid = gsub(&[single()], &[0]);
    let mut empty = valid.clone();
    set_u16(&mut empty, 26, 0);
    assert_blocked(
        &with_table(&template, *b"GSUB", empty),
        "GSUB",
        "empty language system",
    );

    let mut scripts = words(&[2]);
    scripts.extend(b"latn");
    scripts.extend(words(&[14]));
    scripts.extend(b"DFLT");
    scripts.extend(words(&[14, 4, 0, 0, u16::MAX, 1, 0]));
    let unordered = replace_script_list(&valid, &scripts);
    assert_blocked(
        &with_table(&template, *b"GSUB", unordered),
        "GSUB",
        "script records are not strictly ordered",
    );

    let mut scripts = words(&[1]);
    scripts.extend(b"DFLT");
    scripts.extend(words(&[8, 0, 2]));
    scripts.extend(b"TRK ");
    scripts.extend(words(&[16]));
    scripts.extend(b"ENG ");
    scripts.extend(words(&[16, 0, u16::MAX, 1, 0]));
    let unordered = replace_script_list(&valid, &scripts);
    assert_blocked(
        &with_table(&template, *b"GSUB", unordered),
        "GSUB",
        "language records are not strictly ordered",
    );

    // 4096 feature records alias one large, valid lookup array. The encoded table
    // is small; traversing every alias would expand to over 130 million references.
    let old_lookup_list = usize::from(u16_at(&valid, 8));
    let mut alias = valid[..30].to_vec();
    alias.extend_from_slice(&valid[old_lookup_list..]);
    let feature_offset = alias.len() as u16;
    set_u16(&mut alias, 6, feature_offset);
    set_u16(&mut alias, 8, 30);
    alias.extend(words(&[4096]));
    for _ in 0..4096 {
        alias.extend(b"calt");
        alias.extend(words(&[2 + 4096 * 6]));
    }
    alias.extend(words(&[0, 32768]));
    alias.resize(alias.len() + 32768 * 2, 0);
    assert!(alias.len() < 1024 * 1024);
    assert_blocked(
        &with_table(&template, *b"GSUB", alias),
        "GSUB",
        "feature lookup references exceed resource limit",
    );
}

fn replace_script_list(bytes: &[u8], script_list: &[u8]) -> Vec<u8> {
    let old_feature_offset = usize::from(u16_at(bytes, 6));
    let old_lookup_offset = usize::from(u16_at(bytes, 8));
    let mut output = bytes[..10].to_vec();
    output.extend_from_slice(script_list);
    let feature_offset = output.len();
    output.extend_from_slice(&bytes[old_feature_offset..]);
    set_u16(&mut output, 6, feature_offset as u16);
    set_u16(
        &mut output,
        8,
        (feature_offset + old_lookup_offset - old_feature_offset) as u16,
    );
    output
}

#[test]
fn gsub_and_gpos_extension_wrappers_preserve_flags_and_shaping() {
    let original = compile(
        "lookup Substitute useExtension { lookupflag IgnoreMarks; sub a by a.alt; } Substitute;
         lookup Position useExtension { lookupflag RightToLeft IgnoreLigatures; pos a.alt b -35; } Position;
         feature calt { lookup Substitute; } calt;
         feature kern { lookup Position; } kern;",
    );
    let imported = import(&original.bytes);
    for (tag, kind) in [(*b"GSUB", 7), (*b"GPOS", 9)] {
        let data = &imported.extra.raw_tables[&Tag(tag)];
        let list = usize::from(u16_at(data, 8));
        let lookup = list + usize::from(u16_at(data, list + 2));
        assert_eq!(u16_at(data, lookup), kind, "fixture must use extensions");
    }
    let inventory = LayoutImport::from_font(&imported);
    assert!(inventory.can_convert(), "{:?}", inventory.blockers);
    assert!(inventory
        .lookups
        .iter()
        .any(|lookup| lookup.table == "GSUB" && lookup.kind == 1 && lookup.flags == 8));
    assert!(inventory
        .lookups
        .iter()
        .any(|lookup| lookup.table == "GPOS" && lookup.kind == 2 && lookup.flags == 5));
    let candidate = compile(&inventory.decompile().unwrap());
    equivalent_layout(&inventory, &candidate.bytes, &candidate.glyph_order).unwrap();
    for text in ["ab", "aab", "a\u{301}b", "ba"] {
        assert_eq!(
            shaping_fingerprint(&original.bytes, text).unwrap(),
            shaping_fingerprint(&candidate.bytes, text).unwrap()
        );
    }
}

#[test]
fn base_only_binary_import_owns_layout_and_roundtrips_without_gsub_or_gpos() {
    let original = compile(
        "table BASE { HorizAxis.BaseTagList romn; HorizAxis.BaseScriptList latn romn 0; } BASE;",
    );
    let imported = import(&original.bytes);
    assert_eq!(
        imported.features.source_of_truth,
        FeaturesSource::ImportedTables
    );
    let inventory = LayoutImport::from_font(&imported);
    assert!(inventory.can_convert(), "{:?}", inventory.blockers);
    assert_eq!(
        inventory
            .tables
            .iter()
            .map(|table| table.tag.as_str())
            .collect::<Vec<_>>(),
        ["BASE"]
    );
    assert!(inventory.lookups.is_empty());
    let candidate = compile(&inventory.decompile().unwrap());
    equivalent_layout(&inventory, &candidate.bytes, &candidate.glyph_order).unwrap();
}

#[test]
fn legacy_kern_only_actual_binary_import_owns_layout_and_blocks_conversion() {
    let original = compile("");
    let kern = words(&[0, 1, 0, 20, 1, 1, 6, 0, 0, 1, 2, (-40i16) as u16]);
    let bytes = replace_sfnt_table(&original.bytes, *b"kern", kern.clone());
    let imported = import(&bytes);
    assert_eq!(
        imported.features.source_of_truth,
        FeaturesSource::ImportedTables
    );
    assert_eq!(imported.extra.raw_tables[&Tag(*b"kern")], kern);
    let inventory = LayoutImport::from_font(&imported);
    assert_eq!(
        inventory
            .tables
            .iter()
            .map(|table| table.tag.as_str())
            .collect::<Vec<_>>(),
        ["kern"]
    );
    assert_blocked(&imported, "kern", "competing layout authority");
}

#[test]
fn fingerprints_are_stable_but_detect_layout_and_provenance_changes() {
    let original = compile("feature calt { sub a by a.alt; } calt;");
    let imported = import(&original.bytes);
    let fingerprint = LayoutImport::fingerprint_for_font(&imported);
    let repeated = import(&original.bytes);
    assert_eq!(fingerprint, LayoutImport::from_font(&repeated).fingerprint);
    let mut changed = imported.clone();
    changed
        .extra
        .raw_tables
        .insert(Tag(*b"TEST"), vec![1, 2, 3]);
    assert_eq!(fingerprint, LayoutImport::fingerprint_for_font(&changed));
    changed
        .extra
        .raw_tables
        .get_mut(&Tag(*b"GSUB"))
        .unwrap()
        .push(0);
    assert_ne!(fingerprint, LayoutImport::fingerprint_for_font(&changed));
    let mut changed = imported.clone();
    changed
        .extra
        .source
        .as_mut()
        .unwrap()
        .glyph_order
        .swap(1, 2);
    assert_ne!(fingerprint, LayoutImport::fingerprint_for_font(&changed));
    assert_blocked(&changed, "provenance", "glyph order differs");
    changed.extra.source = None;
    assert_blocked(&changed, "provenance", "glyph order is missing");
}

#[test]
fn missing_and_unused_mark_filter_dependencies_block_complete_conversion() {
    let original = compile(
        "@marks = [acute]; table GDEF { GlyphClassDef [a b], , [acute], ; } GDEF;
         lookup Filter { lookupflag UseMarkFilteringSet @marks; sub a by a.alt; } Filter;
         feature calt { lookup Filter; } calt;",
    );
    let imported = import(&original.bytes);
    assert!(LayoutImport::from_font(&imported).can_convert());
    let mut missing = imported.clone();
    missing.extra.raw_tables.remove(&Tag(*b"GDEF"));
    assert_blocked(&missing, "GSUB", "missing GDEF mark filtering set");
    let mut unused = imported.clone();
    let data = unused.extra.raw_tables.get_mut(&Tag(*b"GSUB")).unwrap();
    let list = usize::from(u16_at(data, 8));
    let lookup = list + usize::from(u16_at(data, list + 2));
    set_u16(data, lookup + 2, 0);
    assert_blocked(&unused, "GDEF", "unused mark filtering sets");
}

/// Rebuild a valid sfnt directory/checksums while injecting one test table.
fn replace_sfnt_table(bytes: &[u8], tag: [u8; 4], data: Vec<u8>) -> Vec<u8> {
    let mut tables = BTreeMap::<[u8; 4], Vec<u8>>::new();
    for index in 0..usize::from(u16_at(bytes, 4)) {
        let record = 12 + index * 16;
        let tag = bytes[record..record + 4].try_into().unwrap();
        let offset = u32_at(bytes, record + 8) as usize;
        let length = u32_at(bytes, record + 12) as usize;
        tables.insert(tag, bytes[offset..offset + length].to_vec());
    }
    tables.insert(tag, data);
    tables.get_mut(b"head").unwrap()[8..12].fill(0);
    let count = tables.len() as u16;
    let selector = count.ilog2() as u16;
    let search_range = (1u16 << selector) * 16;
    let mut output = bytes[..4].to_vec();
    output.extend(words(&[
        count,
        search_range,
        selector,
        count * 16 - search_range,
    ]));
    output.resize(12 + tables.len() * 16, 0);
    let mut head = 0;
    for (index, (tag, data)) in tables.into_iter().enumerate() {
        let record = 12 + index * 16;
        let offset = output.len();
        output[record..record + 4].copy_from_slice(&tag);
        output[record + 4..record + 8].copy_from_slice(&checksum(&data).to_be_bytes());
        output[record + 8..record + 12].copy_from_slice(&(offset as u32).to_be_bytes());
        output[record + 12..record + 16].copy_from_slice(&(data.len() as u32).to_be_bytes());
        if tag == *b"head" {
            head = offset;
        }
        output.extend(data);
        output.resize(output.len().next_multiple_of(4), 0);
    }
    let adjustment = 0xb1b0_afbau32.wrapping_sub(checksum(&output));
    output[head + 8..head + 12].copy_from_slice(&adjustment.to_be_bytes());
    output
}

fn checksum(bytes: &[u8]) -> u32 {
    bytes.chunks(4).fold(0u32, |sum, chunk| {
        let mut word = [0u8; 4];
        word[..chunk.len()].copy_from_slice(chunk);
        sum.wrapping_add(u32::from_be_bytes(word))
    })
}
