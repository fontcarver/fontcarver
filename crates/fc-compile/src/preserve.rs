//! Preservation merge for imported raw tables and untouched TrueType outlines.

use std::collections::HashSet;

use fc_model::{FeaturesSource, Font, GlyphName, MasterId, Tag};
use read_fonts::{types::GlyphId, FontRef, TableProvider};
use sha2::{Digest, Sha256};
use skrifa::{GlyphNameSource, GlyphNames};
use write_fonts::{from_obj::ToOwnedTable, FontBuilder};

use crate::{Error, ExportOptions, ExportWarning};

const LAYOUT_TAGS: [Tag; 4] = [Tag(*b"GSUB"), Tag(*b"GPOS"), Tag(*b"GDEF"), Tag(*b"BASE")];
const GID_INDEXED_TAGS: [Tag; 15] = [
    Tag(*b"kern"),
    Tag(*b"MATH"),
    Tag(*b"hdmx"),
    Tag(*b"VDMX"),
    Tag(*b"LTSH"),
    Tag(*b"EBDT"),
    Tag(*b"EBLC"),
    Tag(*b"CBDT"),
    Tag(*b"CBLC"),
    Tag(*b"sbix"),
    Tag(*b"COLR"),
    Tag(*b"CPAL"),
    Tag(*b"SVG "),
    Tag(*b"VORG"),
    Tag(*b"JSTF"),
];
const VARIATION_TAGS: [Tag; 7] = [
    Tag(*b"gvar"),
    Tag(*b"HVAR"),
    Tag(*b"MVAR"),
    Tag(*b"cvar"),
    Tag(*b"avar"),
    Tag(*b"fvar"),
    Tag(*b"STAT"),
];
const DSIG: Tag = Tag(*b"DSIG");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OrderVerification {
    Verified,
    Changed,
    Unavailable,
}

struct Provenance<'a> {
    order: OrderVerification,
    original: Option<FontRef<'a>>,
}

/// Merges compiled tables with eligible imported data and untouched outlines.
pub fn preserve_merge(
    compiled: &[u8],
    font: &Font,
    touched: &HashSet<GlyphName>,
    options: &ExportOptions,
) -> Result<(Vec<u8>, Vec<ExportWarning>), Error> {
    let compiled_face =
        FontRef::new(compiled).map_err(|error| Error::parse("compiled sfnt", error))?;
    let compiled_order =
        crate::large_font::compiled_glyph_order(&compiled_face, font, options.production_names)?;
    let mut warnings = Vec::new();
    let provenance = provenance(font, &compiled_order, &mut warnings);
    let master = selected_master(font, options)?;
    let source_has_hinting = has_hinting(font, master);
    let imported_layout = font.features.source_of_truth == FeaturesSource::ImportedTables
        && font
            .extra
            .raw_tables
            .keys()
            .any(|tag| LAYOUT_TAGS.contains(tag));
    if provenance.order == OrderVerification::Unavailable
        && ((options.preserve_untouched_tables && imported_layout)
            || (options.keep_tt_instructions && source_has_hinting))
    {
        warnings.push(ExportWarning {
            code: "preservation-unavailable-no-provenance",
            message: "Layout and hinting preservation were skipped because neither original bytes nor imported glyph order provenance is available".to_owned(),
        });
    }

    let mut builder = FontBuilder::new();
    for record in compiled_face.table_directory().table_records() {
        let tag = Tag(record.tag().to_be_bytes());
        if tag == DSIG || VARIATION_TAGS.contains(&tag) {
            continue;
        }
        let data = compiled_face
            .table_data(record.tag())
            .ok_or_else(|| Error::parse("compiled table", record.tag()))?;
        builder.add_raw(record.tag(), data.as_bytes());
    }

    warn_variation_tables(font, &mut warnings);
    if options.preserve_untouched_tables {
        merge_raw_tables(&mut builder, font, provenance.order, &mut warnings);
    } else {
        warn_tables_not_preserved(font, &mut warnings);
    }

    if options.keep_tt_instructions {
        if touched.is_empty()
            && provenance.order == OrderVerification::Verified
            && provenance.original.as_ref().is_some_and(|original| {
                original
                    .table_data(read_fonts::types::Tag::new(b"glyf"))
                    .is_some()
            })
        {
            if let Some(original) = provenance.original.as_ref() {
                preserve_original_outlines(&mut builder, &compiled_face, original)?;
            }
        } else if source_has_hinting && !touched.is_empty() {
            warnings.push(hinting_dropped_touched(touched));
        } else if source_has_hinting {
            match (provenance.order, provenance.original.as_ref()) {
                (OrderVerification::Verified, _) => warnings.push(ExportWarning {
                    code: "hinting-dropped",
                    message: "TrueType instructions were dropped because the original outline bytes are unavailable".to_owned(),
                }),
                (OrderVerification::Changed, _) => warnings.push(ExportWarning {
                    code: "hinting-dropped-glyph-set-changed",
                    message: "TrueType instructions were dropped because the exported glyph set or order changed".to_owned(),
                }),
                (OrderVerification::Unavailable, _) => {}
            }
        }
    }

    Ok((builder.build(), warnings))
}

fn merge_raw_tables(
    builder: &mut FontBuilder<'_>,
    font: &Font,
    verified_order: OrderVerification,
    warnings: &mut Vec<ExportWarning>,
) {
    let imported_layout = font.features.source_of_truth == FeaturesSource::ImportedTables;
    let mut layout_dropped = Vec::new();
    let mut gid_dropped = Vec::new();
    for (tag, data) in &font.extra.raw_tables {
        if *tag == DSIG || VARIATION_TAGS.contains(tag) {
            continue;
        }
        if LAYOUT_TAGS.contains(tag) {
            if imported_layout && verified_order == OrderVerification::Verified {
                builder.add_raw(read_tag(*tag), data.clone());
            } else if imported_layout {
                layout_dropped.push(*tag);
            }
            continue;
        }
        if GID_INDEXED_TAGS.contains(tag) && verified_order != OrderVerification::Verified {
            gid_dropped.push(*tag);
            continue;
        }
        let read_tag = read_tag(*tag);
        if *tag == Tag(*b"gasp") || !builder.contains(read_tag) {
            builder.add_raw(read_tag, data.clone());
        }
    }
    if !layout_dropped.is_empty() && verified_order == OrderVerification::Changed {
        warnings.push(ExportWarning {
            code: "layout-dropped-glyph-order-changed",
            message: format!(
                "Imported layout tables were dropped because the glyph set or order changed: {}",
                tag_list(layout_dropped)
            ),
        });
    }
    if !gid_dropped.is_empty() {
        warnings.push(ExportWarning {
            code: "raw-table-dropped-glyph-order-changed",
            message: format!(
                "Glyph-ID-indexed raw tables were dropped because the original glyph order was not verified unchanged: {}",
                tag_list(gid_dropped)
            ),
        });
    }
}

fn warn_variation_tables(font: &Font, warnings: &mut Vec<ExportWarning>) {
    let tags: Vec<_> = VARIATION_TAGS
        .into_iter()
        .filter(|tag| source_has_table(font, *tag))
        .collect();
    if !tags.is_empty() {
        warnings.push(ExportWarning {
            code: "variation-tables-dropped",
            message: format!(
                "Variation tables were dropped from the static export: {}",
                tag_list(tags)
            ),
        });
    }
}

fn warn_tables_not_preserved(font: &Font, warnings: &mut Vec<ExportWarning>) {
    let tags: Vec<_> = font
        .extra
        .raw_tables
        .keys()
        .copied()
        .filter(|tag| *tag != DSIG && !VARIATION_TAGS.contains(tag))
        .collect();
    if !tags.is_empty() {
        warnings.push(ExportWarning {
            code: "tables-not-preserved",
            message: format!(
                "Imported raw tables were not preserved by request: {}",
                tag_list(tags)
            ),
        });
    }
}

fn provenance<'a>(
    font: &'a Font,
    compiled_order: &[GlyphName],
    warnings: &mut Vec<ExportWarning>,
) -> Provenance<'a> {
    let Some(source) = font.extra.source.as_ref() else {
        return Provenance {
            order: OrderVerification::Unavailable,
            original: None,
        };
    };
    let bytes = source.original_bytes.as_deref();
    if let Some(bytes) = bytes {
        if !source.sha256.is_empty() {
            let actual = format!("{:x}", Sha256::digest(bytes.as_slice()));
            if actual != source.sha256 {
                warnings.push(ExportWarning {
                    code: "provenance-hash-mismatch",
                    message: "Original font bytes did not match their import digest; provenance-dependent preservation was disabled".to_owned(),
                });
                return Provenance {
                    order: OrderVerification::Changed,
                    original: None,
                };
            }
        }
    }

    let source_order = if source.glyph_order.is_empty() {
        bytes.and_then(|bytes| order_from_matching_face(bytes, compiled_order))
    } else {
        Some(source.glyph_order.clone())
    };
    let Some(source_order) = source_order else {
        return Provenance {
            order: OrderVerification::Unavailable,
            original: None,
        };
    };
    let order = if source_order == compiled_order {
        OrderVerification::Verified
    } else {
        OrderVerification::Changed
    };
    let original = bytes.and_then(|bytes| face_with_order(bytes, &source_order));
    Provenance { order, original }
}

fn order_from_matching_face(bytes: &[u8], compiled_order: &[GlyphName]) -> Option<Vec<GlyphName>> {
    let mut same_count = None;
    for index in 0..u32::from(u16::MAX) {
        let Ok(face) = FontRef::from_index(bytes, index) else {
            break;
        };
        let Some(order) = imported_glyph_order(&face) else {
            continue;
        };
        if order == compiled_order {
            return Some(order);
        }
        if order.len() == compiled_order.len() && same_count.is_none() {
            same_count = Some(order);
        }
    }
    same_count
}

fn face_with_order<'a>(bytes: &'a [u8], expected: &[GlyphName]) -> Option<FontRef<'a>> {
    let mut matching = None;
    for index in 0..u32::from(u16::MAX) {
        let Ok(face) = FontRef::from_index(bytes, index) else {
            break;
        };
        if imported_glyph_order(&face).as_deref() == Some(expected) {
            if matching.is_some() {
                return None;
            }
            matching = Some(face);
        }
    }
    matching
}

fn imported_glyph_order(face: &FontRef<'_>) -> Option<Vec<GlyphName>> {
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
        names.push(GlyphName::from(value));
    }
    Some(names)
}

fn preserve_original_outlines(
    builder: &mut FontBuilder<'_>,
    compiled: &FontRef<'_>,
    original: &FontRef<'_>,
) -> Result<(), Error> {
    for tag in [*b"glyf", *b"loca", *b"maxp"] {
        let tag = read_fonts::types::Tag::new(&tag);
        let data = original
            .table_data(tag)
            .ok_or_else(|| Error::parse("original hinting table", tag))?;
        builder.add_raw(tag, data.as_bytes().to_vec());
    }
    for tag in [*b"fpgm", *b"prep", *b"cvt "] {
        let tag = read_fonts::types::Tag::new(&tag);
        if let Some(data) = original.table_data(tag) {
            builder.add_raw(tag, data.as_bytes().to_vec());
        }
    }
    let mut head: write_fonts::tables::head::Head = compiled
        .head()
        .map_err(|error| Error::parse("compiled head", error))?
        .to_owned_table();
    head.index_to_loc_format = original
        .head()
        .map_err(|error| Error::parse("original head", error))?
        .index_to_loc_format();
    builder
        .add_table(&head)
        .map_err(|error| Error::write("head", error))?;
    Ok(())
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

fn hinting_dropped_touched(touched: &HashSet<GlyphName>) -> ExportWarning {
    let mut glyphs: Vec<_> = touched.iter().map(ToString::to_string).collect();
    glyphs.sort();
    ExportWarning {
        code: "hinting-dropped",
        message: format!(
            "TrueType instructions were dropped because these glyphs changed: {}",
            glyphs.join(", ")
        ),
    }
}

fn source_has_table(font: &Font, tag: Tag) -> bool {
    font.extra.raw_tables.contains_key(&tag)
        || font
            .extra
            .source
            .as_ref()
            .is_some_and(|source| source.table_tags.contains(&tag))
}

fn tag_list(mut tags: Vec<Tag>) -> String {
    tags.sort();
    tags.dedup();
    tags.into_iter()
        .map(|tag| tag.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn read_tag(tag: Tag) -> read_fonts::types::Tag {
    read_fonts::types::Tag::new(&tag.0)
}
