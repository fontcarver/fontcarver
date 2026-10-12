//! Valid-table fallbacks for sfnts near the 65,535-glyph OpenType limit.
//!
//! `write-fonts` 0.52 constructs post format 2 indices with `u16` and panics
//! when `258 + custom_name_count > 65_535`. Fontc also builds cmap format 4
//! through `write-fonts`, whose 16-bit length and range offsets cannot encode
//! every pathological BMP mapping. The fallback below temporarily renames
//! enough glyphs to let fontc finish, then emits post format 3 and a bounded
//! format 4 compatibility cmap alongside a complete format 12 cmap.

use std::collections::{BTreeMap, HashMap, HashSet};

use fc_model::{Font, GlyphName};
use read_fonts::{types::Version16Dot16, FontRef, TableProvider};
use write_fonts::FontBuilder;

use crate::Error;

const POST_STANDARD_NAMES: usize = 258;
const POST_INDEX_LIMIT: usize = u16::MAX as usize;
const FORMAT_4_MAX_MAPPING_SEGMENTS: usize = 8_188;
const FONTC_FORMAT_4_MAPPING_LIMIT: usize = 4_096;

pub(crate) fn requires_workaround(font: &Font, production_names: bool) -> bool {
    let names = final_post_names(font, production_names, &HashMap::new());
    !post_names_fit_capacity(names.iter().map(|(_, name)| name.as_str()))
}

pub(crate) fn workaround_renames(
    font: &Font,
    production_names: bool,
) -> HashMap<GlyphName, String> {
    let names = final_post_names(font, production_names, &HashMap::new());
    let standard: HashSet<_> = read_fonts::tables::post::DEFAULT_GLYPH_NAMES
        .iter()
        .copied()
        .collect();
    let mut custom_count = names
        .iter()
        .filter(|(_, name)| !standard.contains(name.as_str()))
        .count();
    let mut available = read_fonts::tables::post::DEFAULT_GLYPH_NAMES
        .iter()
        .copied()
        .filter(|standard_name| names.iter().all(|(_, name)| name != standard_name));
    let mut renames = HashMap::new();
    for (identity, name) in &names {
        if POST_STANDARD_NAMES + custom_count <= POST_INDEX_LIMIT {
            break;
        }
        if standard.contains(name.as_str()) || identity.as_ref() == ".notdef" {
            continue;
        }
        if let Some(standard_name) = available.next() {
            renames.insert(identity.clone(), standard_name.to_owned());
            custom_count -= 1;
        }
    }
    renames
}

pub(crate) fn fontir_glyph_order(font: &Font) -> Vec<GlyphName> {
    let mut order: Vec<_> = font
        .glyphs
        .values()
        .filter(|glyph| glyph.export && glyph.name.as_ref() != ".notdef")
        .map(|glyph| glyph.name.clone())
        .collect();
    order.insert(0, GlyphName::from(".notdef"));
    order
}

pub(crate) fn fontc_safe_bmp_codepoints(font: &Font) -> HashSet<(GlyphName, u32)> {
    let mut mappings = BTreeMap::new();
    for (gid, name) in fontir_glyph_order(font).into_iter().enumerate() {
        let Some(glyph) = font.glyph(&name) else {
            continue;
        };
        for codepoint in glyph.unicodes.iter().copied().filter(|cp| *cp <= 0xFFFF) {
            let delta = i64::try_from(gid).unwrap_or(i64::MAX) - i64::from(codepoint);
            if (i64::from(i16::MIN)..=i64::from(i16::MAX)).contains(&delta) {
                mappings.entry(codepoint).or_insert_with(|| name.clone());
            }
        }
    }
    mappings
        .into_iter()
        .take(FONTC_FORMAT_4_MAPPING_LIMIT)
        .map(|(codepoint, name)| (name, codepoint))
        .collect()
}

pub(crate) fn compiled_glyph_order(
    face: &FontRef<'_>,
    font: &Font,
    production_names: bool,
) -> Result<Vec<GlyphName>, Error> {
    let glyph_count = usize::from(
        face.maxp()
            .map_err(|error| Error::parse("compiled maxp", error))?
            .num_glyphs(),
    );
    let workaround = if requires_workaround(font, production_names) {
        workaround_renames(font, production_names)
    } else {
        HashMap::new()
    };
    let expected = final_post_names(font, production_names, &workaround);
    if expected.len() != glyph_count {
        return Err(Error::parse(
            "compiled glyph order",
            format!(
                "fontc emitted {glyph_count} glyphs for {} expected names",
                expected.len()
            ),
        ));
    }

    let post = face.post().ok();
    if let Some(post) = post.filter(|post| post.version() == Version16Dot16::VERSION_2_0) {
        if usize::from(post.num_glyphs().unwrap_or_default()) != glyph_count {
            return Err(Error::parse(
                "compiled post",
                "format 2 name count does not match maxp.numGlyphs",
            ));
        }
        let by_post_name: HashMap<_, _> = expected
            .iter()
            .map(|(identity, post_name)| (post_name.as_str(), identity.clone()))
            .collect();
        // Random access rescans Pascal strings from the start for every name.
        // The iterator reuses offsets while preserving the actual emitted GIDs.
        let order: Vec<_> = post
            .glyph_names()
            .map(|(_, name)| {
                by_post_name.get(name).cloned().ok_or_else(|| {
                    Error::parse(
                        "compiled glyph order",
                        format!("post name {name:?} is not a model glyph name"),
                    )
                })
            })
            .collect::<Result<_, _>>()?;
        if order.len() != glyph_count {
            return Err(Error::parse("compiled post", "missing format 2 glyph name"));
        }
        return Ok(order);
    }

    Ok(expected.into_iter().map(|(identity, _)| identity).collect())
}

pub(crate) fn patch_tables(
    compiled: &[u8],
    font: &Font,
    order: &[GlyphName],
) -> Result<Vec<u8>, Error> {
    let face =
        FontRef::new(compiled).map_err(|error| Error::parse("large compiled sfnt", error))?;
    let glyph_count = usize::from(
        face.maxp()
            .map_err(|error| Error::parse("large compiled maxp", error))?
            .num_glyphs(),
    );
    if order.len() != glyph_count {
        return Err(Error::parse(
            "large compiled glyph order",
            format!(
                "metadata has {} names for {glyph_count} glyphs",
                order.len()
            ),
        ));
    }
    let cmap = cmap_formats_4_and_12(font, order)?;
    let post = post_format_3(&face)?;
    let mut builder = FontBuilder::new();
    for record in face.table_directory().table_records() {
        let data = face
            .table_data(record.tag())
            .ok_or_else(|| Error::parse("large compiled table", record.tag()))?;
        builder.add_raw(record.tag(), data.as_bytes());
    }
    builder.add_raw(read_fonts::types::Tag::new(b"cmap"), cmap);
    builder.add_raw(read_fonts::types::Tag::new(b"post"), post);
    Ok(builder.build())
}

fn final_post_names(
    font: &Font,
    production_names: bool,
    workaround: &HashMap<GlyphName, String>,
) -> Vec<(GlyphName, String)> {
    let order = fontir_glyph_order(font);
    let has_production_names = production_names
        && font
            .glyphs
            .values()
            .any(|glyph| glyph.production_name.is_some());
    let uses_rename_map = has_production_names || !workaround.is_empty();
    let mut seen = HashMap::<String, usize>::new();
    order
        .into_iter()
        .map(|identity| {
            let model_glyph = font.glyph(&identity);
            let mut name = workaround
                .get(&identity)
                .cloned()
                .or_else(|| {
                    production_names
                        .then(|| model_glyph.and_then(|glyph| glyph.production_name.clone()))
                        .flatten()
                })
                .unwrap_or_else(|| identity.to_string());
            if uses_rename_map {
                name.retain(|character| {
                    character.is_ascii_alphanumeric() || character == '.' || character == '_'
                });
                if let Some(next_suffix) = seen.get(&name).copied() {
                    let mut suffix = next_suffix;
                    while seen.contains_key(&format!("{name}.{suffix}")) {
                        suffix += 1;
                    }
                    seen.insert(name.clone(), suffix + 1);
                    name = format!("{name}.{suffix}");
                }
                seen.insert(name.clone(), 1);
            }
            (identity, name)
        })
        .collect()
}

fn post_names_fit_capacity<'a>(names: impl IntoIterator<Item = &'a str>) -> bool {
    let standard: HashSet<_> = read_fonts::tables::post::DEFAULT_GLYPH_NAMES
        .iter()
        .copied()
        .collect();
    let custom: HashSet<_> = names
        .into_iter()
        .filter(|name| !standard.contains(name))
        .collect();
    POST_STANDARD_NAMES + custom.len() <= POST_INDEX_LIMIT
}

fn cmap_formats_4_and_12(font: &Font, order: &[GlyphName]) -> Result<Vec<u8>, Error> {
    let mut mappings = BTreeMap::new();
    for (gid, name) in order.iter().enumerate() {
        let gid = u32::try_from(gid).map_err(|_| Error::Limit("glyph ID"))?;
        let Some(glyph) = font.glyph(name) else {
            continue;
        };
        for codepoint in &glyph.unicodes {
            if char::from_u32(*codepoint).is_some() {
                mappings.entry(*codepoint).or_insert(gid);
            }
        }
    }
    let format_4 = cmap_format_4(&mappings)?;
    let format_12 = cmap_format_12(&mappings)?;
    let format_4_offset = 4usize + 4 * 8;
    let format_12_offset = format_4_offset
        .checked_add(format_4.len())
        .ok_or(Error::Limit("cmap offset"))?;
    let mut bytes = Vec::with_capacity(format_12_offset + format_12.len());
    push_u16(&mut bytes, 0);
    push_u16(&mut bytes, 4);
    for (platform, encoding, offset) in [
        (0, 3, format_4_offset),
        (0, 4, format_12_offset),
        (3, 1, format_4_offset),
        (3, 10, format_12_offset),
    ] {
        push_u16(&mut bytes, platform);
        push_u16(&mut bytes, encoding);
        push_u32(
            &mut bytes,
            u32::try_from(offset).map_err(|_| Error::Limit("cmap offset"))?,
        );
    }
    bytes.extend(format_4);
    bytes.extend(format_12);
    Ok(bytes)
}

fn cmap_format_4(mappings: &BTreeMap<u32, u32>) -> Result<Vec<u8>, Error> {
    let mut groups = mapping_groups(
        mappings
            .iter()
            .filter(|(codepoint, _)| **codepoint < 0xFFFF)
            .map(|(codepoint, gid)| (*codepoint, *gid)),
    );
    groups.truncate(FORMAT_4_MAX_MAPPING_SEGMENTS);
    let segment_count = groups.len() + 1;
    let length = 16usize
        .checked_add(
            segment_count
                .checked_mul(8)
                .ok_or(Error::Limit("cmap format 4"))?,
        )
        .ok_or(Error::Limit("cmap format 4"))?;
    let segment_count_u16 =
        u16::try_from(segment_count).map_err(|_| Error::Limit("cmap format 4"))?;
    let power = 1_u16 << (u16::BITS - 1 - segment_count_u16.leading_zeros());
    let search_range = power * 2;
    let entry_selector = u16::try_from(power.ilog2()).map_err(|_| Error::Limit("cmap format 4"))?;
    let range_shift = segment_count_u16 * 2 - search_range;
    let mut bytes = Vec::with_capacity(length);
    push_u16(&mut bytes, 4);
    push_u16(
        &mut bytes,
        u16::try_from(length).map_err(|_| Error::Limit("cmap format 4"))?,
    );
    push_u16(&mut bytes, 0);
    push_u16(&mut bytes, segment_count_u16 * 2);
    push_u16(&mut bytes, search_range);
    push_u16(&mut bytes, entry_selector);
    push_u16(&mut bytes, range_shift);
    for (_, end, _) in &groups {
        push_u16(&mut bytes, *end as u16);
    }
    push_u16(&mut bytes, 0xFFFF);
    push_u16(&mut bytes, 0);
    for (start, _, _) in &groups {
        push_u16(&mut bytes, *start as u16);
    }
    push_u16(&mut bytes, 0xFFFF);
    for (start, _, start_gid) in &groups {
        push_u16(&mut bytes, start_gid.wrapping_sub(*start) as u16);
    }
    push_u16(&mut bytes, 1);
    for _ in 0..segment_count {
        push_u16(&mut bytes, 0);
    }
    Ok(bytes)
}

fn cmap_format_12(mappings: &BTreeMap<u32, u32>) -> Result<Vec<u8>, Error> {
    let groups = mapping_groups(mappings.iter().map(|(cp, gid)| (*cp, *gid)));
    let subtable_len = 16usize
        .checked_add(
            groups
                .len()
                .checked_mul(12)
                .ok_or(Error::Limit("cmap groups"))?,
        )
        .ok_or(Error::Limit("cmap length"))?;
    let mut bytes = Vec::with_capacity(subtable_len);
    push_u16(&mut bytes, 12);
    push_u16(&mut bytes, 0);
    push_u32(
        &mut bytes,
        u32::try_from(subtable_len).map_err(|_| Error::Limit("cmap length"))?,
    );
    push_u32(&mut bytes, 0);
    push_u32(
        &mut bytes,
        u32::try_from(groups.len()).map_err(|_| Error::Limit("cmap groups"))?,
    );
    for (start, end, gid) in groups {
        push_u32(&mut bytes, start);
        push_u32(&mut bytes, end);
        push_u32(&mut bytes, gid);
    }
    Ok(bytes)
}

fn mapping_groups(mappings: impl IntoIterator<Item = (u32, u32)>) -> Vec<(u32, u32, u32)> {
    let mut groups: Vec<(u32, u32, u32)> = Vec::new();
    for (codepoint, gid) in mappings {
        match groups.last_mut() {
            Some((start, end, start_gid))
                if end.checked_add(1) == Some(codepoint)
                    && start_gid.checked_add(codepoint - *start) == Some(gid) =>
            {
                *end = codepoint;
            }
            _ => groups.push((codepoint, codepoint, gid)),
        }
    }
    groups
}

fn post_format_3(face: &FontRef<'_>) -> Result<Vec<u8>, Error> {
    let source = face
        .table_data(read_fonts::types::Tag::new(b"post"))
        .ok_or_else(|| Error::parse("post", "table is missing"))?;
    let header = source
        .as_bytes()
        .get(..32)
        .ok_or_else(|| Error::parse("post", "table is shorter than its header"))?;
    let mut post = header.to_vec();
    post[..4].copy_from_slice(&0x0003_0000_u32.to_be_bytes());
    Ok(post)
}

fn push_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_be_bytes());
}

fn push_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_be_bytes());
}

#[cfg(test)]
mod tests {
    use fc_model::{Font, Glyph, GlyphName};
    use read_fonts::{tables::cmap::CmapSubtable, FontData, FontRead};

    use super::{cmap_formats_4_and_12, post_names_fit_capacity};

    #[test]
    fn large_post_order_is_exact_and_bounded() {
        use read_fonts::FontRef;
        use write_fonts::{
            tables::{maxp::Maxp, post::Post},
            FontBuilder,
        };
        let mut font = Font::new_empty("Color glyph order", 1_000);
        for index in 0..20_000 {
            font.insert_glyph(Glyph::new(format!("color.layer.custom.{index:05}")))
                .expect("insert glyph");
        }
        let mut order = super::fontir_glyph_order(&font);
        order[1..].reverse();
        let post = Post::new_v2(order.iter().map(|name| name.as_ref()));
        let mut builder = FontBuilder::new();
        builder
            .add_table(&Maxp {
                num_glyphs: order.len() as u16,
                ..Default::default()
            })
            .unwrap();
        builder.add_table(&post).unwrap();
        let bytes = builder.build();
        let face = FontRef::new(&bytes).unwrap();
        let started = std::time::Instant::now();
        let actual = super::compiled_glyph_order(&face, &font, false).unwrap();
        let elapsed = started.elapsed();
        assert_eq!(actual, order);
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "post order exceeded 2s: {elapsed:?}"
        );

        // A stopped name iterator must not silently accept a truncated table.
        use read_fonts::TableProvider;
        let mut truncated = face.post().unwrap().offset_data().as_bytes().to_vec();
        truncated.pop();
        builder.add_raw(read_fonts::types::Tag::new(b"post"), truncated);
        let bytes = builder.build();
        assert!(super::compiled_glyph_order(&FontRef::new(&bytes).unwrap(), &font, false).is_err());
    }

    #[test]
    fn post_capacity_uses_unique_nonstandard_name_count() {
        let mut representable: Vec<String> = read_fonts::tables::post::DEFAULT_GLYPH_NAMES
            .iter()
            .map(|name| (*name).to_owned())
            .collect();
        representable.extend((0..65_142).map(|index| format!("custom{index:05}")));
        assert_eq!(representable.len(), 65_400);
        assert!(post_names_fit_capacity(
            representable.iter().map(String::as_str)
        ));

        let overflowing: Vec<_> = (0..65_400)
            .map(|index| format!("custom{index:05}"))
            .collect();
        assert!(!post_names_fit_capacity(
            overflowing.iter().map(String::as_str)
        ));
    }

    #[test]
    fn fallback_cmap_contains_bmp_and_full_repertoire_subtables() {
        let mut font = Font::new_empty("Large cmap", 1_000);
        let mut glyph = Glyph::new("A");
        glyph.unicodes = vec![0x41, 0x1F600];
        font.insert_glyph(glyph).expect("insert A");
        let order = vec![GlyphName::from(".notdef"), GlyphName::from("A")];
        let bytes = cmap_formats_4_and_12(&font, &order).expect("build cmap");
        let cmap = read_fonts::tables::cmap::Cmap::read(FontData::new(&bytes)).expect("parse cmap");
        let formats: Vec<_> = cmap
            .encoding_records()
            .iter()
            .map(
                |record| match record.subtable(cmap.offset_data()).expect("subtable") {
                    CmapSubtable::Format4(_) => 4,
                    CmapSubtable::Format12(_) => 12,
                    _ => 0,
                },
            )
            .collect();
        assert_eq!(formats, [4, 12, 4, 12]);
    }
}
