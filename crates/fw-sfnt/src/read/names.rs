use std::collections::HashSet;

use fw_model::{FontInfo, GlyphName, NamePlatform, NameRecord};
use read_fonts::{
    tables::{head::Head, name::Name},
    FontRef, TableProvider,
};
use skrifa::GlyphNameSource;

use super::Warning;
use crate::Error;

pub(crate) fn font_info(face: &FontRef<'_>, head: &Head<'_>) -> Result<FontInfo, Error> {
    let mut info = FontInfo {
        units_per_em: head.units_per_em(),
        open_type_head_created: Some(head.created().as_secs().to_string()),
        open_type_head_flags: set_bits(u32::from(head.flags().bits())),
        open_type_head_lowest_rec_ppem: Some(head.lowest_rec_ppem()),
        ..FontInfo::default()
    };
    let revision = head.font_revision().to_f64();
    info.version_major = revision.max(0.0).floor() as u32;
    info.version_minor = ((revision.fract().abs() * 1000.0).round()) as u32;

    let Some(name) = optional_name(face)? else {
        return Ok(info);
    };
    info.names = all_name_records(&name)?;
    info.family_name = select(&info.names, 1).unwrap_or_default();
    info.style_name = select(&info.names, 2).unwrap_or_else(|| "Regular".to_owned());
    info.copyright = select(&info.names, 0);
    info.open_type_name_unique_id = select(&info.names, 3);
    info.postscript_full_name = select(&info.names, 4);
    info.open_type_name_version = select(&info.names, 5);
    info.postscript_font_name = select(&info.names, 6);
    info.trademark = select(&info.names, 7);
    info.open_type_name_manufacturer = select(&info.names, 8);
    info.open_type_name_designer = select(&info.names, 9);
    info.open_type_name_description = select(&info.names, 10);
    info.open_type_name_manufacturer_url = select(&info.names, 11);
    info.open_type_name_designer_url = select(&info.names, 12);
    info.open_type_name_license = select(&info.names, 13);
    info.open_type_name_license_url = select(&info.names, 14);
    info.open_type_name_preferred_family_name = select(&info.names, 16);
    info.open_type_name_preferred_subfamily_name = select(&info.names, 17);
    info.open_type_name_compatible_full_name = select(&info.names, 18);
    info.open_type_name_sample_text = select(&info.names, 19);
    info.open_type_name_wws_family_name = select(&info.names, 21);
    info.open_type_name_wws_subfamily_name = select(&info.names, 22);
    Ok(info)
}

pub(crate) fn best_name(face: &FontRef<'_>, name_id: u16) -> Option<String> {
    let name = face.name().ok()?;
    all_name_records(&name)
        .ok()
        .and_then(|records| select(&records, name_id))
}

pub(crate) fn name_by_id(face: &FontRef<'_>, name_id: u16) -> Option<String> {
    best_name(face, name_id)
}

pub(crate) fn glyph_names(
    face: &FontRef<'_>,
    glyph_count: u16,
    warnings: &mut Vec<Warning>,
) -> Result<Vec<GlyphName>, Error> {
    if let Some(cids) = super::cff::cid_values(face)? {
        if cids.len() != usize::from(glyph_count) {
            return Err(Error::Parse {
                table: "CFF ",
                reason: format!(
                    "CID charset has {} entries for {glyph_count} glyphs",
                    cids.len()
                ),
            });
        }
        return Ok(cids
            .into_iter()
            .map(|cid| GlyphName::from(format!("cid{cid:05}")))
            .collect());
    }

    let source = skrifa::GlyphNames::new(face);
    let synthesized_source = source.source() == GlyphNameSource::Synthesized;
    let mut synthesized = synthesized_source;
    let mut used = HashSet::with_capacity(usize::from(glyph_count));
    let mut result = Vec::with_capacity(usize::from(glyph_count));
    for gid in 0..glyph_count {
        let raw_gid = read_fonts::types::GlyphId::new(u32::from(gid));
        let candidate = source.get(raw_gid);
        synthesized |= candidate.as_ref().is_none_or(|name| name.is_synthesized());
        let mut value = if gid == 0 && synthesized_source {
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
        result.push(GlyphName::from(value));
    }
    if synthesized {
        warnings.push(Warning {
            code: "glyph-names-synthesized",
            message:
                "one or more glyph names were synthesized because post/CFF names were unavailable"
                    .to_owned(),
        });
    }
    Ok(result)
}

fn optional_name<'a>(face: &FontRef<'a>) -> Result<Option<Name<'a>>, Error> {
    if face
        .table_data(read_fonts::types::Tag::new(b"name"))
        .is_none()
    {
        return Ok(None);
    }
    face.name()
        .map(Some)
        .map_err(|error| Error::parse("name", error))
}

fn all_name_records(name: &Name<'_>) -> Result<Vec<NameRecord>, Error> {
    name.name_record()
        .iter()
        .filter(|record| matches!(record.platform_id(), 0 | 1 | 3))
        .map(|record| {
            let platform = match record.platform_id() {
                1 => NamePlatform::Mac,
                3 => NamePlatform::Windows,
                _ => NamePlatform::Unicode,
            };
            let text = record
                .string(name.string_data())
                .map_err(|error| Error::parse("name", error))?
                .to_string();
            Ok(NameRecord {
                name_id: record.name_id().to_u16(),
                language: language(name, record.platform_id(), record.language_id()),
                text,
                platform,
            })
        })
        .collect()
}

fn select(records: &[NameRecord], name_id: u16) -> Option<String> {
    records
        .iter()
        .enumerate()
        .filter(|(_, record)| record.name_id == name_id && !record.text.is_empty())
        .min_by_key(|(index, record)| (preference(record), *index))
        .map(|(_, record)| record.text.clone())
}

fn preference(record: &NameRecord) -> u8 {
    match record.platform {
        NamePlatform::Windows if record.language.starts_with("en") => 0,
        NamePlatform::Unicode => 1,
        NamePlatform::Mac if record.language.starts_with("en") => 2,
        NamePlatform::Mac => 3,
        NamePlatform::Windows => 4,
    }
}

fn language(name: &Name<'_>, platform: u16, language_id: u16) -> String {
    if name.version() == 1 && language_id >= 0x8000 {
        let index = usize::from(language_id - 0x8000);
        if let Some(tag) = name
            .lang_tag_record()
            .and_then(|records| records.get(index))
            .and_then(|record| record.lang_tag(name.string_data()).ok())
        {
            return tag.to_string();
        }
    }
    let mapped = match platform {
        1 => mac_language(language_id),
        3 => windows_language(language_id),
        _ => None,
    };
    mapped.unwrap_or("und").to_owned()
}

fn set_bits(bits: u32) -> Vec<u8> {
    (0..u32::BITS)
        .filter(|bit| bits & (1 << bit) != 0)
        .map(|bit| bit as u8)
        .collect()
}

fn mac_language(id: u16) -> Option<&'static str> {
    MAC_LANGUAGES
        .binary_search_by_key(&id, |entry| entry.0)
        .ok()
        .map(|index| MAC_LANGUAGES[index].1)
}

fn windows_language(id: u16) -> Option<&'static str> {
    WINDOWS_LANGUAGES
        .binary_search_by_key(&id, |entry| entry.0)
        .ok()
        .map(|index| WINDOWS_LANGUAGES[index].1)
}

// Static copies keep language normalization stable when upstream mappings change.
const MAC_LANGUAGES: &[(u16, &str)] = &[
    (0, "en"),
    (1, "fr"),
    (2, "de"),
    (3, "it"),
    (4, "nl"),
    (5, "sv"),
    (6, "es"),
    (7, "da"),
    (8, "pt"),
    (9, "nb"),
    (10, "he"),
    (11, "ja"),
    (12, "ar"),
    (13, "fi"),
    (14, "el"),
    (15, "is"),
    (16, "mt"),
    (17, "tr"),
    (18, "hr"),
    (19, "zh-Hant"),
    (20, "ur"),
    (21, "hi"),
    (22, "th"),
    (23, "ko"),
    (24, "lt"),
    (25, "pl"),
    (26, "hu"),
    (27, "et"),
    (28, "lv"),
    (29, "se"),
    (30, "fo"),
    (31, "fa"),
    (32, "ru"),
    (33, "zh-Hans"),
    (34, "nl-BE"),
    (35, "ga"),
    (36, "sq"),
    (37, "ro"),
    (38, "cs"),
    (39, "sk"),
    (40, "sl"),
    (41, "yi"),
    (42, "sr"),
    (43, "mk"),
    (44, "bg"),
    (45, "uk"),
    (46, "be"),
    (47, "uz"),
    (48, "kk"),
    (49, "az-Cyrl"),
    (50, "az-Arab"),
    (51, "hy"),
    (52, "ka"),
    (53, "mo"),
    (54, "ky"),
    (55, "tg"),
    (56, "tk"),
    (57, "mn-Mong"),
    (58, "mn-Cyrl"),
    (59, "ps"),
    (60, "ku"),
    (61, "ks"),
    (62, "sd"),
    (63, "bo"),
    (64, "ne"),
    (65, "sa"),
    (66, "mr"),
    (67, "bn"),
    (68, "as"),
    (69, "gu"),
    (70, "pa"),
    (71, "or"),
    (72, "ml"),
    (73, "kn"),
    (74, "ta"),
    (75, "te"),
    (76, "si"),
    (77, "my"),
    (78, "km"),
    (79, "lo"),
    (80, "vi"),
    (81, "id"),
    (82, "tl"),
    (83, "ms-Latn"),
    (84, "ms-Arab"),
    (85, "am"),
    (86, "ti"),
    (87, "om"),
    (88, "so"),
    (89, "sw"),
    (90, "rw"),
    (91, "rn"),
    (92, "ny"),
    (93, "mg"),
    (94, "eo"),
    (128, "cy"),
    (129, "eu"),
    (130, "ca"),
    (131, "la"),
    (132, "qu"),
    (133, "gn"),
    (134, "ay"),
    (135, "tt"),
    (136, "ug"),
    (137, "dz"),
    (138, "jv-Latn"),
    (139, "su-Latn"),
    (140, "gl"),
    (141, "af"),
    (142, "br"),
    (143, "iu"),
    (144, "gd"),
    (145, "gv"),
    (146, "ga-Latg"),
    (147, "to"),
    (148, "grc"),
    (149, "kl"),
    (150, "az-Latn"),
    (151, "nn"),
];

const WINDOWS_LANGUAGES: &[(u16, &str)] = &[
    (0x0401, "ar-SA"),
    (0x0402, "bg-BG"),
    (0x0403, "ca-ES"),
    (0x0404, "zh-TW"),
    (0x0405, "cs-CZ"),
    (0x0406, "da-DK"),
    (0x0407, "de-DE"),
    (0x0408, "el-GR"),
    (0x0409, "en-US"),
    (0x040a, "es-ES-tradnl"),
    (0x040b, "fi-FI"),
    (0x040c, "fr-FR"),
    (0x040d, "he-IL"),
    (0x040e, "hu-HU"),
    (0x040f, "is-IS"),
    (0x0410, "it-IT"),
    (0x0411, "ja-JP"),
    (0x0412, "ko-KR"),
    (0x0413, "nl-NL"),
    (0x0414, "nb-NO"),
    (0x0415, "pl-PL"),
    (0x0416, "pt-BR"),
    (0x0417, "rm-CH"),
    (0x0418, "ro-RO"),
    (0x0419, "ru-RU"),
    (0x041a, "hr-HR"),
    (0x041b, "sk-SK"),
    (0x041c, "sq-AL"),
    (0x041d, "sv-SE"),
    (0x041e, "th-TH"),
    (0x041f, "tr-TR"),
    (0x0420, "ur-PK"),
    (0x0421, "id-ID"),
    (0x0422, "uk-UA"),
    (0x0423, "be-BY"),
    (0x0424, "sl-SI"),
    (0x0425, "et-EE"),
    (0x0426, "lv-LV"),
    (0x0427, "lt-LT"),
    (0x0428, "tg-Cyrl-TJ"),
    (0x0429, "fa-IR"),
    (0x042a, "vi-VN"),
    (0x042b, "hy-AM"),
    (0x042c, "az-Latn-AZ"),
    (0x042d, "eu-ES"),
    (0x042e, "hsb-DE"),
    (0x042f, "mk-MK"),
    (0x0432, "tn-ZA"),
    (0x0434, "xh-ZA"),
    (0x0435, "zu-ZA"),
    (0x0436, "af-ZA"),
    (0x0437, "ka-GE"),
    (0x0438, "fo-FO"),
    (0x0439, "hi-IN"),
    (0x043a, "mt-MT"),
    (0x043b, "se-NO"),
    (0x043e, "ms-MY"),
    (0x043f, "kk-KZ"),
    (0x0440, "ky-KG"),
    (0x0441, "sw-KE"),
    (0x0442, "tk-TM"),
    (0x0443, "uz-Latn-UZ"),
    (0x0444, "tt-RU"),
    (0x0445, "bn-IN"),
    (0x0446, "pa-IN"),
    (0x0447, "gu-IN"),
    (0x0448, "or-IN"),
    (0x0449, "ta-IN"),
    (0x044a, "te-IN"),
    (0x044b, "kn-IN"),
    (0x044c, "ml-IN"),
    (0x044d, "as-IN"),
    (0x044e, "mr-IN"),
    (0x044f, "sa-IN"),
    (0x0450, "mn-Cyrl"),
    (0x0451, "bo-CN"),
    (0x0452, "cy-GB"),
    (0x0453, "km-KH"),
    (0x0454, "lo-LA"),
    (0x0456, "gl-ES"),
    (0x0457, "kok-IN"),
    (0x045a, "syr-SY"),
    (0x045b, "si-LK"),
    (0x045d, "iu-Cans-CA"),
    (0x045e, "am-ET"),
    (0x0461, "ne-NP"),
    (0x0462, "fy-NL"),
    (0x0463, "ps-AF"),
    (0x0464, "fil-PH"),
    (0x0465, "dv-MV"),
    (0x0468, "ha-Latn-NG"),
    (0x046a, "yo-NG"),
    (0x046b, "quz-BO"),
    (0x046c, "nso-ZA"),
    (0x046d, "ba-RU"),
    (0x046e, "lb-LU"),
    (0x046f, "kl-GL"),
    (0x0470, "ig-NG"),
    (0x0478, "ii-CN"),
    (0x047a, "arn-CL"),
    (0x047c, "moh-CA"),
    (0x047e, "br-FR"),
    (0x0480, "ug-CN"),
    (0x0481, "mi-NZ"),
    (0x0482, "oc-FR"),
    (0x0483, "co-FR"),
    (0x0484, "gsw-FR"),
    (0x0485, "sah-RU"),
    (0x0486, "qut-GT"),
    (0x0487, "rw-RW"),
    (0x0488, "wo-SN"),
    (0x048c, "prs-AF"),
    (0x0491, "gd-GB"),
    (0x0801, "ar-IQ"),
    (0x0804, "zh-CN"),
    (0x0807, "de-CH"),
    (0x0809, "en-GB"),
    (0x080a, "es-MX"),
    (0x080c, "fr-BE"),
    (0x0810, "it-CH"),
    (0x0813, "nl-BE"),
    (0x0814, "nn-NO"),
    (0x0816, "pt-PT"),
    (0x081a, "sr-Latn-CS"),
    (0x081d, "sv-FI"),
    (0x082c, "az-Cyrl-AZ"),
    (0x082e, "dsb-DE"),
    (0x083b, "se-SE"),
    (0x083c, "ga-IE"),
    (0x083e, "ms-BN"),
    (0x0843, "uz-Cyrl-UZ"),
    (0x0845, "bn-BD"),
    (0x0850, "mn-Mong-CN"),
    (0x085d, "iu-Latn-CA"),
    (0x085f, "tzm-Latn-DZ"),
    (0x086b, "quz-EC"),
    (0x0c01, "ar-EG"),
    (0x0c04, "zh-HK"),
    (0x0c07, "de-AT"),
    (0x0c09, "en-AU"),
    (0x0c0a, "es-ES"),
    (0x0c0c, "fr-CA"),
    (0x0c1a, "sr-Cyrl-CS"),
    (0x0c3b, "se-FI"),
    (0x0c6b, "quz-PE"),
    (0x1001, "ar-LY"),
    (0x1004, "zh-SG"),
    (0x1007, "de-LU"),
    (0x1009, "en-CA"),
    (0x100a, "es-GT"),
    (0x100c, "fr-CH"),
    (0x101a, "hr-BA"),
    (0x103b, "smj-NO"),
    (0x1401, "ar-DZ"),
    (0x1404, "zh-MO"),
    (0x1407, "de-LI"),
    (0x1409, "en-NZ"),
    (0x140a, "es-CR"),
    (0x140c, "fr-LU"),
    (0x141a, "bs-Latn-BA"),
    (0x143b, "smj-SE"),
    (0x1801, "ar-MA"),
    (0x1809, "en-IE"),
    (0x180a, "es-PA"),
    (0x180c, "fr-MC"),
    (0x181a, "sr-Latn-BA"),
    (0x183b, "sma-NO"),
    (0x1c01, "ar-TN"),
    (0x1c09, "en-ZA"),
    (0x1c0a, "es-DO"),
    (0x1c1a, "sr-Cyrl-BA"),
    (0x1c3b, "sma-SE"),
    (0x2001, "ar-OM"),
    (0x2009, "en-JM"),
    (0x200a, "es-VE"),
    (0x201a, "bs-Cyrl-BA"),
    (0x203b, "sms-FI"),
    (0x2401, "ar-YE"),
    (0x2409, "en-029"),
    (0x240a, "es-CO"),
    (0x241a, "sr-Latn-RS"),
    (0x243b, "smn-FI"),
    (0x2801, "ar-SY"),
    (0x2809, "en-BZ"),
    (0x280a, "es-PE"),
    (0x281a, "sr-Cyrl-RS"),
    (0x2c01, "ar-JO"),
    (0x2c09, "en-TT"),
    (0x2c0a, "es-AR"),
    (0x2c1a, "sr-Latn-ME"),
    (0x3001, "ar-LB"),
    (0x3009, "en-ZW"),
    (0x300a, "es-EC"),
    (0x301a, "sr-Cyrl-ME"),
    (0x3401, "ar-KW"),
    (0x3409, "en-PH"),
    (0x340a, "es-CL"),
    (0x3801, "ar-AE"),
    (0x380a, "es-UY"),
    (0x3c01, "ar-BH"),
    (0x3c0a, "es-PY"),
    (0x4001, "ar-QA"),
    (0x4009, "en-IN"),
    (0x400a, "es-BO"),
    (0x4409, "en-MY"),
    (0x440a, "es-SV"),
    (0x4809, "en-SG"),
    (0x480a, "es-HN"),
    (0x4c0a, "es-NI"),
    (0x500a, "es-PR"),
    (0x540a, "es-US"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corrected_mac_language_ids_match_fonttools() {
        assert_eq!(mac_language(34), Some("nl-BE"));
        assert_eq!(mac_language(146), Some("ga-Latg"));
        assert_eq!(mac_language(148), Some("grc"));
    }

    #[test]
    fn chinese_windows_language_ids_match_fonttools() {
        assert_eq!(windows_language(0x0404), Some("zh-TW"));
        assert_eq!(windows_language(0x0804), Some("zh-CN"));
        assert_eq!(windows_language(0x0c04), Some("zh-HK"));
        assert_eq!(windows_language(0x1004), Some("zh-SG"));
        assert_eq!(windows_language(0x1404), Some("zh-MO"));
    }

    #[test]
    fn convenience_fields_prefer_english_windows_names() {
        let records = vec![
            NameRecord {
                name_id: 1,
                language: "zh-CN".to_owned(),
                text: "Chinese family".to_owned(),
                platform: NamePlatform::Windows,
            },
            NameRecord {
                name_id: 1,
                language: "en-US".to_owned(),
                text: "English family".to_owned(),
                platform: NamePlatform::Windows,
            },
        ];
        assert_eq!(select(&records, 1).as_deref(), Some("English family"));
    }
}
