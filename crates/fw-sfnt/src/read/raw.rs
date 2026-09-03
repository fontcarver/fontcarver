use std::collections::BTreeMap;

use fw_model::Tag;
use read_fonts::FontRef;

const CONSUMED: &[[u8; 4]] = &[
    *b"CFF ", *b"CFF2", *b"OS/2", *b"avar", *b"cmap", *b"cvt ", *b"fpgm", *b"fvar", *b"gasp",
    *b"glyf", *b"head", *b"hhea", *b"hmtx", *b"loca", *b"maxp", *b"name", *b"post", *b"prep",
    *b"vhea", *b"vmtx",
];

const RAW_ONLY: &[[u8; 4]] = &[
    *b"GSUB", *b"GPOS", *b"GDEF", *b"BASE", *b"MATH", *b"JSTF", *b"STAT", *b"COLR", *b"CPAL",
    *b"SVG ", *b"EBDT", *b"EBLC", *b"CBDT", *b"CBLC", *b"sbix", *b"EBSC", *b"hdmx", *b"VDMX",
    *b"LTSH", *b"PCLT", *b"meta", *b"kern", *b"VORG", *b"gasp", *b"gvar", *b"HVAR", *b"MVAR",
    *b"cvar",
];

pub(crate) fn table_tags(face: &FontRef<'_>) -> Vec<Tag> {
    face.table_directory()
        .table_records()
        .iter()
        .map(|record| Tag(record.tag().to_be_bytes()))
        .collect()
}

pub(crate) fn raw_tables(face: &FontRef<'_>) -> BTreeMap<Tag, Vec<u8>> {
    face.table_directory()
        .table_records()
        .iter()
        .filter_map(|record| {
            let tag = record.tag().to_be_bytes();
            if tag == *b"DSIG" || (CONSUMED.contains(&tag) && !RAW_ONLY.contains(&tag)) {
                return None;
            }
            face.table_data(record.tag())
                .map(|data| (Tag(tag), data.as_bytes().to_vec()))
        })
        .collect()
}
