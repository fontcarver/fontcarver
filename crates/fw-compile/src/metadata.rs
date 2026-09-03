//! Conversion of model font metadata into fontir metadata.

use std::collections::HashMap;

use fontir::ir::{NameKey, StaticMetadata};
use fw_model::FontInfo;
use write_fonts::{
    tables::{gasp::GaspRange, head, os2::SelectionFlags},
    types::{NameId, Tag},
};

pub(super) fn names(info: &FontInfo) -> HashMap<NameKey, String> {
    let mut names = HashMap::new();
    for record in &info.names {
        if record.name_id <= NameId::LAST_ALLOWED_NAME_ID.to_u16() && !record.text.is_empty() {
            names.insert(
                NameKey::new(NameId::new(record.name_id), &record.text),
                record.text.clone(),
            );
        }
    }
    let family = nonempty(&info.family_name, "Untitled");
    let style = nonempty(&info.style_name, "Regular");
    insert_name(&mut names, NameId::FAMILY_NAME, family.to_owned());
    insert_name(&mut names, NameId::SUBFAMILY_NAME, style.to_owned());
    let full_name = info
        .postscript_full_name
        .clone()
        .unwrap_or_else(|| format!("{family} {style}"));
    let postscript_name = info
        .postscript_font_name
        .clone()
        .unwrap_or_else(|| fallback_postscript_name(family, style));
    let version = info
        .open_type_name_version
        .clone()
        .unwrap_or_else(|| format!("Version {}.{:03}", info.version_major, info.version_minor));
    let unique_id = info
        .open_type_name_unique_id
        .clone()
        .unwrap_or_else(|| format!("{version};{postscript_name}"));
    insert_name(&mut names, NameId::UNIQUE_ID, unique_id);
    insert_name(&mut names, NameId::FULL_NAME, full_name);
    insert_name(&mut names, NameId::VERSION_STRING, version);
    insert_name(&mut names, NameId::POSTSCRIPT_NAME, postscript_name);

    for (id, value) in [
        (NameId::COPYRIGHT_NOTICE, info.copyright.as_ref()),
        (NameId::TRADEMARK, info.trademark.as_ref()),
        (
            NameId::MANUFACTURER,
            info.open_type_name_manufacturer.as_ref(),
        ),
        (NameId::DESIGNER, info.open_type_name_designer.as_ref()),
        (
            NameId::DESCRIPTION,
            info.open_type_name_description.as_ref(),
        ),
        (
            NameId::VENDOR_URL,
            info.open_type_name_manufacturer_url.as_ref(),
        ),
        (
            NameId::DESIGNER_URL,
            info.open_type_name_designer_url.as_ref(),
        ),
        (
            NameId::LICENSE_DESCRIPTION,
            info.open_type_name_license.as_ref(),
        ),
        (
            NameId::LICENSE_URL,
            info.open_type_name_license_url.as_ref(),
        ),
        (
            NameId::TYPOGRAPHIC_FAMILY_NAME,
            info.open_type_name_preferred_family_name.as_ref(),
        ),
        (
            NameId::TYPOGRAPHIC_SUBFAMILY_NAME,
            info.open_type_name_preferred_subfamily_name.as_ref(),
        ),
        (
            NameId::COMPATIBLE_FULL_NAME,
            info.open_type_name_compatible_full_name.as_ref(),
        ),
        (
            NameId::SAMPLE_TEXT,
            info.open_type_name_sample_text.as_ref(),
        ),
        (
            NameId::WWS_FAMILY_NAME,
            info.open_type_name_wws_family_name.as_ref(),
        ),
        (
            NameId::WWS_SUBFAMILY_NAME,
            info.open_type_name_wws_subfamily_name.as_ref(),
        ),
    ] {
        if let Some(value) = value.filter(|value| !value.is_empty()) {
            insert_name(&mut names, id, value.clone());
        }
    }
    names
}

fn nonempty<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.is_empty() {
        fallback
    } else {
        value
    }
}

fn insert_name(names: &mut HashMap<NameKey, String>, id: NameId, value: String) {
    names.insert(NameKey::new(id, &value), value);
}

fn fallback_postscript_name(family: &str, style: &str) -> String {
    let filtered = format!("{family}-{style}")
        .chars()
        .filter(|character| character.is_ascii_graphic() && !"[](){}<>/%".contains(*character))
        .take(63)
        .collect::<String>();
    nonempty(&filtered, "Untitled-Regular").to_owned()
}

pub(super) fn apply_misc_metadata(metadata: &mut StaticMetadata, info: &FontInfo) {
    metadata.misc.fs_type = Some(bits_u16(&info.open_type_os2_type));
    metadata.misc.is_fixed_pitch = info.postscript_is_fixed_pitch;
    metadata.misc.selection_flags =
        SelectionFlags::from_bits_truncate(bits_u16(&info.open_type_os2_selection));
    if let Some(vendor) = info.open_type_os2_vendor_id.as_deref().and_then(tag_bytes) {
        metadata.misc.vendor_id = Tag::new(&vendor);
    }
    metadata.misc.version_major = i32::try_from(info.version_major).unwrap_or(i32::MAX);
    metadata.misc.version_minor = info.version_minor;
    metadata.misc.head_flags =
        head::Flags::from_bits_truncate(bits_u16(&info.open_type_head_flags));
    if let Some(ppem) = info.open_type_head_lowest_rec_ppem {
        metadata.misc.lowest_rec_ppm = ppem;
    }
    metadata.misc.family_class = info
        .open_type_os2_family_class
        .map(|(class, subclass)| i16::from_be_bytes([class, subclass]));
    metadata.misc.panose = info.open_type_os2_panose.map(Into::into);
    metadata.misc.unicode_range_bits = (!info.open_type_os2_unicode_ranges.is_empty()).then(|| {
        info.open_type_os2_unicode_ranges
            .iter()
            .copied()
            .map(u32::from)
            .collect()
    });
    metadata.misc.codepage_range_bits =
        (!info.open_type_os2_code_page_ranges.is_empty()).then(|| {
            info.open_type_os2_code_page_ranges
                .iter()
                .copied()
                .map(u32::from)
                .collect()
        });
    metadata.misc.us_weight_class = info.open_type_os2_weight_class;
    metadata.misc.us_width_class = info.open_type_os2_width_class;
    metadata.misc.gasp = info
        .open_type_gasp_range_records
        .iter()
        .map(|record| {
            GaspRange::new(
                record.range_max_ppem,
                read_fonts::tables::gasp::GaspRangeBehavior::from_bits_truncate(bits_u16(
                    &record.behavior,
                )),
            )
        })
        .collect();
}

fn bits_u16(bits: &[u8]) -> u16 {
    bits.iter()
        .copied()
        .filter(|bit| *bit < u16::BITS as u8)
        .fold(0, |value, bit| value | (1_u16 << bit))
}

fn tag_bytes(value: &str) -> Option<[u8; 4]> {
    value.as_bytes().try_into().ok()
}
