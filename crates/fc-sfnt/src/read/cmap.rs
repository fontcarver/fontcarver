use std::collections::HashMap;

use read_fonts::{
    tables::cmap::{Cmap2, CmapSubtable, EncodingRecord, PlatformId},
    FontRef, TableProvider,
};

use crate::Error;

#[derive(Debug, PartialEq, Eq)]
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
    let has_unicode_or_windows = records.iter().any(is_unicode_or_windows);
    let mut owners = HashMap::new();
    for record in records {
        if should_skip_record(&record, has_unicode_or_windows) {
            continue;
        }
        let subtable = record
            .subtable(cmap.offset_data())
            .map_err(|error| Error::parse("cmap", error))?;
        if matches!(subtable, CmapSubtable::Format14(_)) {
            continue;
        }
        let is_symbol = record.platform_id() == PlatformId::Windows && record.encoding_id() == 0;
        if matches!(
            &subtable,
            CmapSubtable::Format4(_) | CmapSubtable::Format12(_) | CmapSubtable::Format13(_)
        ) {
            for (codepoint, gid) in
                subtable.iter_with_limits(read_fonts::tables::cmap::CmapIterLimits {
                    max_char: 0x10_FFFF,
                    glyph_count: u32::from(glyph_count),
                })
            {
                if let Ok(gid) = u16::try_from(gid.to_u32()) {
                    if let Some(codepoint) = record_codepoint(&record, codepoint) {
                        add_record_mapping(&mut owners, codepoint, gid, glyph_count, is_symbol);
                    }
                }
            }
        } else {
            for (codepoint, gid) in pairs(&subtable, glyph_count)? {
                if let Some(codepoint) = record_codepoint(&record, codepoint) {
                    add_record_mapping(&mut owners, codepoint, gid, glyph_count, is_symbol);
                }
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

fn is_unicode_or_windows(record: &EncodingRecord) -> bool {
    matches!(
        record.platform_id(),
        PlatformId::Unicode | PlatformId::Windows
    )
}

fn should_skip_record(record: &EncodingRecord, has_unicode_or_windows: bool) -> bool {
    match record.platform_id() {
        PlatformId::Macintosh => has_unicode_or_windows || record.encoding_id() != 0,
        PlatformId::Unicode | PlatformId::Windows => false,
        _ => true,
    }
}

fn record_codepoint(record: &EncodingRecord, codepoint: u32) -> Option<u32> {
    if record.platform_id() != PlatformId::Macintosh || record.encoding_id() != 0 {
        return Some(codepoint);
    }
    let byte = u8::try_from(codepoint).ok()?;
    Some(if byte < 0x80 {
        u32::from(byte)
    } else {
        u32::from(MAC_ROMAN[usize::from(byte - 0x80)])
    })
}

const MAC_ROMAN: [u16; 128] = [
    0x00C4, 0x00C5, 0x00C7, 0x00C9, 0x00D1, 0x00D6, 0x00DC, 0x00E1, 0x00E0, 0x00E2, 0x00E4, 0x00E3,
    0x00E5, 0x00E7, 0x00E9, 0x00E8, 0x00EA, 0x00EB, 0x00ED, 0x00EC, 0x00EE, 0x00EF, 0x00F1, 0x00F3,
    0x00F2, 0x00F4, 0x00F6, 0x00F5, 0x00FA, 0x00F9, 0x00FB, 0x00FC, 0x2020, 0x00B0, 0x00A2, 0x00A3,
    0x00A7, 0x2022, 0x00B6, 0x00DF, 0x00AE, 0x00A9, 0x2122, 0x00B4, 0x00A8, 0x2260, 0x00C6, 0x00D8,
    0x221E, 0x00B1, 0x2264, 0x2265, 0x00A5, 0x00B5, 0x2202, 0x2211, 0x220F, 0x03C0, 0x222B, 0x00AA,
    0x00BA, 0x03A9, 0x00E6, 0x00F8, 0x00BF, 0x00A1, 0x00AC, 0x221A, 0x0192, 0x2248, 0x2206, 0x00AB,
    0x00BB, 0x2026, 0x00A0, 0x00C0, 0x00C3, 0x00D5, 0x0152, 0x0153, 0x2013, 0x2014, 0x201C, 0x201D,
    0x2018, 0x2019, 0x00F7, 0x25CA, 0x00FF, 0x0178, 0x2044, 0x20AC, 0x2039, 0x203A, 0xFB01, 0xFB02,
    0x2021, 0x00B7, 0x201A, 0x201E, 0x2030, 0x00C2, 0x00CA, 0x00C1, 0x00CB, 0x00C8, 0x00CD, 0x00CE,
    0x00CF, 0x00CC, 0x00D3, 0x00D4, 0xF8FF, 0x00D2, 0x00DA, 0x00DB, 0x00D9, 0x0131, 0x02C6, 0x02DC,
    0x00AF, 0x02D8, 0x02D9, 0x02DA, 0x00B8, 0x02DD, 0x02DB, 0x02C7,
];

fn add_record_mapping(
    owners: &mut HashMap<u32, u16>,
    codepoint: u32,
    gid: u16,
    glyph_count: u16,
    is_symbol: bool,
) {
    add_mapping(owners, codepoint, gid, glyph_count);
    if is_symbol && (0xF000..=0xF0FF).contains(&codepoint) {
        add_mapping(owners, codepoint - 0xF000, gid, glyph_count);
    }
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

#[cfg(test)]
mod tests {
    use std::fs;

    use read_fonts::{FontRef, TableProvider};

    use super::{
        add_record_mapping, apply_variation_sequences, is_unicode_or_windows, pairs,
        record_codepoint, record_rank, should_skip_record, GlyphData,
    };
    use crate::Error;

    #[test]
    fn streaming_common_formats_match_eager_mapping() {
        for fixture in ["DejaVuSans.ttf", "NotoSans-Regular.ttf"] {
            let bytes = fs::read(fc_fixtures::path(fixture)).expect("read fixture");
            let face = FontRef::new(&bytes).expect("parse fixture");
            let glyph_count = face.maxp().expect("maxp table").num_glyphs();

            assert_eq!(
                super::glyph_data(&face, glyph_count).expect("streaming cmap"),
                eager_glyph_data(&face, glyph_count).expect("eager cmap"),
                "{fixture} cmap changed"
            );
        }
    }

    fn eager_glyph_data(face: &FontRef<'_>, glyph_count: u16) -> Result<GlyphData, Error> {
        let cmap = face.cmap().map_err(|error| Error::parse("cmap", error))?;
        let mut records = cmap.encoding_records().to_vec();
        records.sort_by_key(record_rank);
        let has_unicode_or_windows = records.iter().any(is_unicode_or_windows);
        let mut owners = std::collections::HashMap::new();
        for record in records {
            if should_skip_record(&record, has_unicode_or_windows) {
                continue;
            }
            let subtable = record
                .subtable(cmap.offset_data())
                .map_err(|error| Error::parse("cmap", error))?;
            let is_symbol = record.platform_id() == read_fonts::tables::cmap::PlatformId::Windows
                && record.encoding_id() == 0;
            for (codepoint, gid) in pairs(&subtable, glyph_count)? {
                if let Some(codepoint) = record_codepoint(&record, codepoint) {
                    add_record_mapping(&mut owners, codepoint, gid, glyph_count, is_symbol);
                }
            }
        }
        let mut data = GlyphData {
            unicodes: vec![Vec::new(); usize::from(glyph_count)],
            variation_selectors: vec![Vec::new(); usize::from(glyph_count)],
        };
        for (codepoint, gid) in &owners {
            data.unicodes[usize::from(*gid)].push(*codepoint);
        }
        for values in &mut data.unicodes {
            values.sort_unstable();
        }
        apply_variation_sequences(&cmap, glyph_count, &owners, &mut data.variation_selectors)?;
        Ok(data)
    }
}
