use std::collections::HashMap;

use read_fonts::{
    tables::cmap::{Cmap2, CmapSubtable, EncodingRecord, PlatformId},
    FontRef, TableProvider,
};

use crate::Error;

pub(crate) struct GlyphData {
    pub(crate) unicodes: Vec<Vec<u32>>,
    pub(crate) variation_selectors: Vec<Vec<(u32, u32)>>,
}

pub(crate) fn glyph_data(face: &FontRef<'_>, glyph_count: u16) -> Result<GlyphData, Error> {
    let mut data = GlyphData {
        unicodes: vec![Vec::new(); usize::from(glyph_count)],
        variation_selectors: vec![Vec::new(); usize::from(glyph_count)],
    };
    if face
        .table_data(read_fonts::types::Tag::new(b"cmap"))
        .is_none()
    {
        return Ok(data);
    }
    let cmap = face.cmap().map_err(|error| Error::parse("cmap", error))?;
    let mut records = cmap.encoding_records().to_vec();
    records.sort_by_key(record_rank);
    let mut owners = HashMap::new();
    for record in records {
        let subtable = record
            .subtable(cmap.offset_data())
            .map_err(|error| Error::parse("cmap", error))?;
        if matches!(subtable, CmapSubtable::Format14(_)) {
            continue;
        }
        let is_symbol = record.platform_id() == PlatformId::Windows && record.encoding_id() == 0;
        for (codepoint, gid) in pairs(&subtable, glyph_count)? {
            add_mapping(&mut owners, codepoint, gid, glyph_count);
            if is_symbol && (0xF000..=0xF0FF).contains(&codepoint) {
                add_mapping(&mut owners, codepoint - 0xF000, gid, glyph_count);
            }
        }
    }
    for (codepoint, gid) in &owners {
        data.unicodes[usize::from(*gid)].push(*codepoint);
    }
    for values in &mut data.unicodes {
        values.sort_unstable();
    }
    apply_variation_sequences(&cmap, glyph_count, &owners, &mut data.variation_selectors)?;
    Ok(data)
}

fn record_rank(record: &EncodingRecord) -> (u8, u16, u16) {
    let rank = match (record.platform_id(), record.encoding_id()) {
        (PlatformId::Windows, 10) => 0,
        (PlatformId::Windows, 1) => 1,
        (PlatformId::Unicode, _) => 2,
        (PlatformId::Windows, 0) => 3,
        (PlatformId::Macintosh, _) => 4,
        _ => 5,
    };
    (rank, record.platform_id() as u16, record.encoding_id())
}

fn add_mapping(owners: &mut HashMap<u32, u16>, codepoint: u32, gid: u16, glyph_count: u16) {
    if codepoint <= 0x10_FFFF && gid != 0 && gid < glyph_count {
        owners.entry(codepoint).or_insert(gid);
    }
}

fn pairs(subtable: &CmapSubtable<'_>, glyph_count: u16) -> Result<Vec<(u32, u16)>, Error> {
    let limit = u32::from(glyph_count);
    let result = match subtable {
        CmapSubtable::Format0(table) => (0..=u8::MAX)
            .filter_map(|codepoint| {
                table
                    .map_codepoint(codepoint)
                    .map(|gid| (u32::from(codepoint), gid.to_u32()))
            })
            .filter(|(_, gid)| *gid < limit)
            .map(|(cp, gid)| (cp, gid as u16))
            .collect(),
        CmapSubtable::Format2(table) => format2_pairs(table, glyph_count)?,
        CmapSubtable::Format8(table) => sequential_groups(
            table.groups().iter().map(|group| {
                (
                    group.start_char_code(),
                    group.end_char_code(),
                    group.start_glyph_id(),
                )
            }),
            glyph_count,
        )?,
        CmapSubtable::Format14(_) => Vec::new(),
        _ => subtable
            .iter_with_limits(read_fonts::tables::cmap::CmapIterLimits {
                max_char: 0x10_FFFF,
                glyph_count: u32::from(glyph_count),
            })
            .filter_map(|(cp, gid)| u16::try_from(gid.to_u32()).ok().map(|gid| (cp, gid)))
            .collect(),
    };
    Ok(result)
}

fn sequential_groups(
    groups: impl Iterator<Item = (u32, u32, u32)>,
    glyph_count: u16,
) -> Result<Vec<(u32, u16)>, Error> {
    let mut result = Vec::new();
    for (start, end, start_gid) in groups {
        if end < start || end - start > 0x10_FFFF {
            return Err(Error::Parse {
                table: "cmap",
                reason: "invalid or excessive format 8 mapping group".to_owned(),
            });
        }
        let end = end.min(0x10_FFFF);
        for cp in start..=end {
            let Some(gid) = start_gid.checked_add(cp - start) else {
                continue;
            };
            if gid < u32::from(glyph_count) {
                result.push((cp, gid as u16));
            }
        }
    }
    Ok(result)
}

fn format2_pairs(table: &Cmap2<'_>, glyph_count: u16) -> Result<Vec<(u32, u16)>, Error> {
    let bytes = table.offset_data();
    let bytes = bytes.as_bytes();
    let keys = table.sub_header_keys();
    let max_key = keys.iter().map(|key| key.get()).max().unwrap_or(0);
    if max_key % 8 != 0 {
        return Err(Error::Parse {
            table: "cmap",
            reason: "format 2 subHeaderKey is not a multiple of eight".to_owned(),
        });
    }
    let subheader_count = usize::from(max_key / 8) + 1;
    let subheaders_start = 6 + 512;
    let subheaders_end = subheaders_start + subheader_count * 8;
    if bytes.len() < subheaders_end {
        return Err(Error::Parse {
            table: "cmap",
            reason: "truncated format 2 subheaders".to_owned(),
        });
    }
    let mut result = Vec::new();
    for high in 0u16..=255 {
        let key = usize::from(keys[usize::from(high)].get() / 8);
        if (high == 0 && key != 0) || (high != 0 && key == 0) {
            continue;
        }
        let base = subheaders_start + key * 8;
        let first = read_u16(bytes, base)?;
        let count = read_u16(bytes, base + 2)?;
        let delta = read_i16(bytes, base + 4)?;
        let range_offset = usize::from(read_u16(bytes, base + 6)?);
        for offset in 0..count {
            let low = first.saturating_add(offset);
            if low > 255 {
                continue;
            }
            let glyph_offset = base + 6 + range_offset + usize::from(offset) * 2;
            let mut gid = read_u16(bytes, glyph_offset)?;
            if gid == 0 {
                continue;
            }
            gid = gid.wrapping_add_signed(delta);
            if gid < glyph_count {
                let cp = if high == 0 {
                    u32::from(low)
                } else {
                    (u32::from(high) << 8) | u32::from(low)
                };
                result.push((cp, gid));
            }
        }
    }
    Ok(result)
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, Error> {
    let raw = bytes.get(offset..offset + 2).ok_or_else(|| Error::Parse {
        table: "cmap",
        reason: "truncated format 2 glyph array".to_owned(),
    })?;
    Ok(u16::from_be_bytes([raw[0], raw[1]]))
}

fn read_i16(bytes: &[u8], offset: usize) -> Result<i16, Error> {
    read_u16(bytes, offset).map(|value| i16::from_be_bytes(value.to_be_bytes()))
}

fn apply_variation_sequences(
    cmap: &read_fonts::tables::cmap::Cmap<'_>,
    glyph_count: u16,
    owners: &HashMap<u32, u16>,
    target: &mut [Vec<(u32, u32)>],
) -> Result<(), Error> {
    let Some((_, table)) = cmap.uvs_subtable() else {
        return Ok(());
    };
    for selector in table.var_selector() {
        let selector_value: u32 = selector.var_selector().into();
        if let Some(default) = selector.default_uvs(table.offset_data()) {
            let default = default.map_err(|error| Error::parse("cmap", error))?;
            for range in default.ranges() {
                let start: u32 = range.start_unicode_value().into();
                for cp in start..=start + u32::from(range.additional_count()) {
                    if let Some(gid) = owners.get(&cp) {
                        target[usize::from(*gid)].push((cp, selector_value));
                    }
                }
            }
        }
        if let Some(non_default) = selector.non_default_uvs(table.offset_data()) {
            let non_default = non_default.map_err(|error| Error::parse("cmap", error))?;
            for mapping in non_default.uvs_mapping() {
                let gid = mapping.glyph_id();
                if gid < glyph_count {
                    target[usize::from(gid)].push((mapping.unicode_value().into(), selector_value));
                }
            }
        }
    }
    for sequences in target {
        sequences.sort_unstable();
        sequences.dedup();
    }
    Ok(())
}
