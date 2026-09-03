//! Conversion of model font metadata into fontir metadata.

use std::collections::{BTreeSet, HashMap};

use chrono::NaiveDateTime;
use fontir::ir::{NameKey, StaticMetadata};
use fw_model::{FontInfo, NamePlatform};
use read_fonts::{FontRef, TableProvider};
use write_fonts::{
    from_obj::ToOwnedTable,
    tables::{gasp::GaspRange, head, os2::SelectionFlags},
    types::{NameId, Tag},
    FontBuilder,
};

use crate::{Error, ExportWarning};

pub(super) fn names(info: &FontInfo) -> HashMap<NameKey, String> {
    let mut names = HashMap::new();
    for record in &info.names {
        if record.platform == NamePlatform::Mac
            || record.name_id > NameId::LAST_ALLOWED_NAME_ID.to_u16()
            || record.text.is_empty()
        {
            continue;
        }
        let Some(language) = windows_language_id(&record.language) else {
            continue;
        };
        names.insert(
            NameKey::new_with_lang(NameId::new(record.name_id), &record.text, language),
            record.text.clone(),
        );
    }
    let family = nonempty(&info.family_name, "Untitled");
    let style = nonempty(&info.style_name, "Regular");
    insert_default_name(&mut names, NameId::FAMILY_NAME, family.to_owned());
    insert_default_name(&mut names, NameId::SUBFAMILY_NAME, style.to_owned());
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
    insert_default_name(&mut names, NameId::UNIQUE_ID, unique_id);
    insert_default_name(&mut names, NameId::FULL_NAME, full_name);
    insert_default_name(&mut names, NameId::VERSION_STRING, version);
    insert_default_name(&mut names, NameId::POSTSCRIPT_NAME, postscript_name);

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
            insert_default_name(&mut names, id, value.clone());
        }
    }
    names
}

pub(super) fn name_warnings(info: &FontInfo) -> Vec<ExportWarning> {
    let mut warnings = Vec::new();
    if info
        .names
        .iter()
        .any(|record| record.platform == NamePlatform::Mac)
    {
        warnings.push(ExportWarning {
            code: "mac-name-records-dropped",
            message: "Macintosh name records were dropped; fontc emits Windows Unicode records"
                .to_owned(),
        });
    }
    let unmapped: BTreeSet<_> = info
        .names
        .iter()
        .filter(|record| {
            record.platform != NamePlatform::Mac
                && !record.text.is_empty()
                && windows_language_id(&record.language).is_none()
        })
        .map(|record| record.language.as_str())
        .collect();
    if !unmapped.is_empty() {
        warnings.push(ExportWarning {
            code: "name-language-unmapped",
            message: format!(
                "Name records with unmapped BCP-47 languages were skipped: {}",
                unmapped.into_iter().collect::<Vec<_>>().join(", ")
            ),
        });
    }
    warnings
}

fn windows_language_id(language: &str) -> Option<u16> {
    let normalized = language.replace('_', "-").to_ascii_lowercase();
    Some(match normalized.as_str() {
        "en" | "en-us" => 0x0409,
        "en-gb" => 0x0809,
        "en-au" => 0x0C09,
        "en-ca" => 0x1009,
        "en-nz" => 0x1409,
        "de" | "de-de" => 0x0407,
        "de-ch" => 0x0807,
        "de-at" => 0x0C07,
        "fr" | "fr-fr" => 0x040C,
        "fr-be" => 0x080C,
        "fr-ca" => 0x0C0C,
        "fr-ch" => 0x100C,
        "es" | "es-es-tradnl" => 0x040A,
        "es-mx" => 0x080A,
        "es-es" => 0x0C0A,
        "it" | "it-it" => 0x0410,
        "it-ch" => 0x0810,
        "ja" | "ja-jp" => 0x0411,
        "ko" | "ko-kr" => 0x0412,
        "zh-tw" | "zh-hant" => 0x0404,
        "zh-cn" | "zh-hans" => 0x0804,
        "zh-hk" => 0x0C04,
        "zh-sg" => 0x1004,
        "zh-mo" => 0x1404,
        "ru" | "ru-ru" => 0x0419,
        "pt-br" => 0x0416,
        "pt" | "pt-pt" => 0x0816,
        "nl" | "nl-nl" => 0x0413,
        "nl-be" => 0x0813,
        "sv" | "sv-se" => 0x041D,
        "sv-fi" => 0x081D,
        "ar" | "ar-sa" => 0x0401,
        "bg" | "bg-bg" => 0x0402,
        "ca" | "ca-es" => 0x0403,
        "cs" | "cs-cz" => 0x0405,
        "da" | "da-dk" => 0x0406,
        "el" | "el-gr" => 0x0408,
        "fi" | "fi-fi" => 0x040B,
        "he" | "he-il" => 0x040D,
        "hu" | "hu-hu" => 0x040E,
        "is" | "is-is" => 0x040F,
        "nb" | "nb-no" => 0x0414,
        "pl" | "pl-pl" => 0x0415,
        "ro" | "ro-ro" => 0x0418,
        "sk" | "sk-sk" => 0x041B,
        "th" | "th-th" => 0x041E,
        "tr" | "tr-tr" => 0x041F,
        "uk" | "uk-ua" => 0x0422,
        "vi" | "vi-vn" => 0x042A,
        _ => return None,
    })
}

fn nonempty<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.is_empty() {
        fallback
    } else {
        value
    }
}

fn insert_default_name(names: &mut HashMap<NameKey, String>, id: NameId, value: String) {
    if names.keys().all(|key| key.name_id != id) {
        names.insert(NameKey::new_with_lang(id, &value, 0x0409), value);
    }
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
    metadata.misc.created = parsed_created(info);
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

pub(super) fn patch_modified_timestamp(bytes: &[u8], info: &FontInfo) -> Result<Vec<u8>, Error> {
    if parsed_created(info).is_none() {
        return Ok(bytes.to_vec());
    }
    let face = FontRef::new(bytes).map_err(|error| Error::parse("compiled sfnt", error))?;
    let mut head: write_fonts::tables::head::Head = face
        .head()
        .map_err(|error| Error::parse("head", error))?
        .to_owned_table();
    head.modified = head.created;
    let mut builder = FontBuilder::new();
    for record in face.table_directory().table_records() {
        let data = face
            .table_data(record.tag())
            .ok_or_else(|| Error::parse("compiled table", record.tag()))?;
        builder.add_raw(record.tag(), data.as_bytes());
    }
    builder
        .add_table(&head)
        .map_err(|error| Error::write("head", error))?;
    Ok(builder.build())
}

fn parsed_created(info: &FontInfo) -> Option<chrono::DateTime<chrono::Utc>> {
    NaiveDateTime::parse_from_str(info.open_type_head_created.as_deref()?, "%Y/%m/%d %H:%M:%S")
        .ok()
        .map(|value| value.and_utc())
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
