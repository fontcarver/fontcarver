//! UFO font-info conversion.

use std::str::FromStr;

use fc_model::{
    FontInfo, GaspRecord, Guide, Master, MasterMetrics, NamePlatform, NameRecord, PsPrivate,
};
use norad::{
    fontinfo::{GaspBehavior, GaspRangeRecord, Os2FamilyClass, Os2Panose, Os2WidthClass},
    Color, Guideline, Identifier, Line,
};

use super::value::{json_to_plist, plist_to_json};
use crate::Error;

pub(crate) const RAW_FONTINFO_KEY: &str = "ufo.fontinfo";

pub(crate) fn from_norad(
    source: &norad::FontInfo,
) -> Result<(FontInfo, MasterMetrics, PsPrivate, Vec<Guide>), Error> {
    let raw = plist_to_json(&plist::to_value(source)?)?;
    let info = FontInfo {
        family_name: source.family_name.clone().unwrap_or_default(),
        style_name: source.style_name.clone().unwrap_or_default(),
        units_per_em: source.units_per_em.as_ref().map_or(0, |value| {
            value.as_f64().round().clamp(0.0, f64::from(u16::MAX)) as u16
        }),
        version_major: source.version_major.unwrap_or_default().max(0) as u32,
        version_minor: source.version_minor.unwrap_or_default(),
        copyright: source.copyright.clone(),
        trademark: source.trademark.clone(),
        note: source.note.clone(),
        open_type_name_designer: source.open_type_name_designer.clone(),
        open_type_name_designer_url: source.open_type_name_designer_url.clone(),
        open_type_name_manufacturer: source.open_type_name_manufacturer.clone(),
        open_type_name_manufacturer_url: source.open_type_name_manufacturer_url.clone(),
        open_type_name_license: source.open_type_name_license.clone(),
        open_type_name_license_url: source.open_type_name_license_url.clone(),
        open_type_name_version: source.open_type_name_version.clone(),
        open_type_name_unique_id: source.open_type_name_unique_id.clone(),
        open_type_name_description: source.open_type_name_description.clone(),
        open_type_name_preferred_family_name: source.open_type_name_preferred_family_name.clone(),
        open_type_name_preferred_subfamily_name: source
            .open_type_name_preferred_subfamily_name
            .clone(),
        open_type_name_compatible_full_name: source.open_type_name_compatible_full_name.clone(),
        open_type_name_sample_text: source.open_type_name_sample_text.clone(),
        open_type_name_wws_family_name: source.open_type_name_wws_family_name.clone(),
        open_type_name_wws_subfamily_name: source.open_type_name_wws_subfamily_name.clone(),
        postscript_font_name: source.postscript_font_name.clone(),
        postscript_full_name: source.postscript_full_name.clone(),
        names: model_names(source.open_type_name_records.as_deref()),
        open_type_os2_vendor_id: source.open_type_os2_vendor_id.clone(),
        open_type_os2_weight_class: source
            .open_type_os2_weight_class
            .and_then(|value| u16::try_from(value).ok()),
        open_type_os2_width_class: source
            .open_type_os2_width_class
            .map(|value| value as u8 as u16),
        open_type_os2_selection: source.open_type_os2_selection.clone().unwrap_or_default(),
        open_type_os2_type: source.open_type_os2_type.clone().unwrap_or_default(),
        open_type_os2_panose: source.open_type_os2_panose.map(panose_to_array),
        open_type_os2_family_class: source
            .open_type_os2_family_class
            .map(|value| (value.class_id, value.subclass_id)),
        open_type_os2_unicode_ranges: source
            .open_type_os2_unicode_ranges
            .clone()
            .unwrap_or_default(),
        open_type_os2_code_page_ranges: source
            .open_type_os2_code_page_ranges
            .clone()
            .unwrap_or_default(),
        open_type_os2_subscript_x_size: source.open_type_os2_subscript_x_size.map(f64::from),
        open_type_os2_subscript_y_size: source.open_type_os2_subscript_y_size.map(f64::from),
        open_type_os2_subscript_x_offset: source.open_type_os2_subscript_x_offset.map(f64::from),
        open_type_os2_subscript_y_offset: source.open_type_os2_subscript_y_offset.map(f64::from),
        open_type_os2_superscript_x_size: source.open_type_os2_superscript_x_size.map(f64::from),
        open_type_os2_superscript_y_size: source.open_type_os2_superscript_y_size.map(f64::from),
        open_type_os2_superscript_x_offset: source
            .open_type_os2_superscript_x_offset
            .map(f64::from),
        open_type_os2_superscript_y_offset: source
            .open_type_os2_superscript_y_offset
            .map(f64::from),
        open_type_os2_strikeout_size: source.open_type_os2_strikeout_size.map(f64::from),
        open_type_os2_strikeout_position: source.open_type_os2_strikeout_position.map(f64::from),
        open_type_head_created: source.open_type_head_created.clone(),
        open_type_head_flags: source.open_type_head_flags.clone().unwrap_or_default(),
        open_type_head_lowest_rec_ppem: source
            .open_type_head_lowest_rec_ppem
            .and_then(|value| u16::try_from(value).ok()),
        open_type_gasp_range_records: source
            .open_type_gasp_range_records
            .as_deref()
            .unwrap_or_default()
            .iter()
            .filter_map(model_gasp)
            .collect(),
        open_type_hhea_caret_slope_rise: source.open_type_hhea_caret_slope_rise,
        open_type_hhea_caret_slope_run: source.open_type_hhea_caret_slope_run,
        open_type_hhea_caret_offset: source.open_type_hhea_caret_offset,
        open_type_vhea_caret_slope_rise: source.open_type_vhea_caret_slope_rise,
        open_type_vhea_caret_slope_run: source.open_type_vhea_caret_slope_run,
        open_type_vhea_caret_offset: source.open_type_vhea_caret_offset,
        postscript_is_fixed_pitch: source.postscript_is_fixed_pitch,
        postscript_default_width_x: source.postscript_default_width_x,
        postscript_nominal_width_x: source.postscript_nominal_width_x,
        postscript_underline_position: source.postscript_underline_position,
        postscript_underline_thickness: source.postscript_underline_thickness,
        postscript_unique_id: source.postscript_unique_id,
        macintosh_fond_name: source.macintosh_fond_name.clone(),
        macintosh_fond_family_id: source.macintosh_fond_family_id,
        user_data: serde_json::json!({ RAW_FONTINFO_KEY: raw }),
    };

    let metrics = MasterMetrics {
        ascender: source.ascender.unwrap_or_default(),
        descender: source.descender.unwrap_or_default(),
        x_height: source.x_height.unwrap_or_default(),
        cap_height: source.cap_height.unwrap_or_default(),
        italic_angle: source.italic_angle.unwrap_or_default(),
        hhea_ascender: source.open_type_hhea_ascender.map(f64::from),
        hhea_descender: source.open_type_hhea_descender.map(f64::from),
        hhea_line_gap: source.open_type_hhea_line_gap.map(f64::from),
        typo_ascender: source.open_type_os2_typo_ascender.map(f64::from),
        typo_descender: source.open_type_os2_typo_descender.map(f64::from),
        typo_line_gap: source.open_type_os2_typo_line_gap.map(f64::from),
        win_ascent: source.open_type_os2_win_ascent.map(f64::from),
        win_descent: source.open_type_os2_win_descent.map(f64::from),
        vhea_ascender: source.open_type_vhea_vert_typo_ascender.map(f64::from),
        vhea_descender: source.open_type_vhea_vert_typo_descender.map(f64::from),
        vhea_line_gap: source.open_type_vhea_vert_typo_line_gap.map(f64::from),
        default_vertical_origin: None,
        default_vertical_advance: None,
    };
    let private = PsPrivate {
        blue_values: source.postscript_blue_values.clone().unwrap_or_default(),
        other_blues: source.postscript_other_blues.clone().unwrap_or_default(),
        family_blues: source.postscript_family_blues.clone().unwrap_or_default(),
        family_other_blues: source
            .postscript_family_other_blues
            .clone()
            .unwrap_or_default(),
        blue_scale: source.postscript_blue_scale,
        blue_shift: source.postscript_blue_shift,
        blue_fuzz: source.postscript_blue_fuzz,
        std_hw: None,
        std_vw: None,
        stem_snap_h: source.postscript_stem_snap_h.clone().unwrap_or_default(),
        stem_snap_v: source.postscript_stem_snap_v.clone().unwrap_or_default(),
        force_bold: source.postscript_force_bold,
        language_group: None,
        other: Default::default(),
    };
    let guides = source
        .guidelines
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(guide_from_norad)
        .collect();
    Ok((info, metrics, private, guides))
}

pub(crate) fn to_norad(
    info: &FontInfo,
    master: &Master,
    guides: &[Guide],
    guide_ids: &[Option<String>],
) -> Result<norad::FontInfo, Error> {
    let mut result = master
        .user_data
        .get(RAW_FONTINFO_KEY)
        .or_else(|| info.user_data.get(RAW_FONTINFO_KEY))
        .map(json_to_plist)
        .transpose()?
        .map(|value| plist::from_value::<norad::FontInfo>(&value))
        .transpose()?
        .unwrap_or_default();

    result.family_name = optional_string(&info.family_name);
    result.style_name = optional_string(if master.name.is_empty() {
        &info.style_name
    } else {
        &master.name
    });
    result.units_per_em = (info.units_per_em != 0).then(|| u32::from(info.units_per_em).into());
    result.version_major =
        (info.version_major != 0).then(|| i32::try_from(info.version_major).unwrap_or(i32::MAX));
    result.version_minor = (info.version_minor != 0).then_some(info.version_minor);
    result.copyright.clone_from(&info.copyright);
    result.trademark.clone_from(&info.trademark);
    result.note.clone_from(&info.note);
    result
        .open_type_name_designer
        .clone_from(&info.open_type_name_designer);
    result
        .open_type_name_designer_url
        .clone_from(&info.open_type_name_designer_url);
    result
        .open_type_name_manufacturer
        .clone_from(&info.open_type_name_manufacturer);
    result
        .open_type_name_manufacturer_url
        .clone_from(&info.open_type_name_manufacturer_url);
    result
        .open_type_name_license
        .clone_from(&info.open_type_name_license);
    result
        .open_type_name_license_url
        .clone_from(&info.open_type_name_license_url);
    result
        .open_type_name_version
        .clone_from(&info.open_type_name_version);
    result
        .open_type_name_unique_id
        .clone_from(&info.open_type_name_unique_id);
    result
        .open_type_name_description
        .clone_from(&info.open_type_name_description);
    result
        .open_type_name_preferred_family_name
        .clone_from(&info.open_type_name_preferred_family_name);
    result
        .open_type_name_preferred_subfamily_name
        .clone_from(&info.open_type_name_preferred_subfamily_name);
    result
        .open_type_name_compatible_full_name
        .clone_from(&info.open_type_name_compatible_full_name);
    result
        .open_type_name_sample_text
        .clone_from(&info.open_type_name_sample_text);
    result
        .open_type_name_wws_family_name
        .clone_from(&info.open_type_name_wws_family_name);
    result
        .open_type_name_wws_subfamily_name
        .clone_from(&info.open_type_name_wws_subfamily_name);
    result
        .postscript_font_name
        .clone_from(&info.postscript_font_name);
    result
        .postscript_full_name
        .clone_from(&info.postscript_full_name);
    if model_names(result.open_type_name_records.as_deref()) != info.names {
        result.open_type_name_records = (!info.names.is_empty()).then(|| norad_names(&info.names));
    }
    result
        .open_type_os2_vendor_id
        .clone_from(&info.open_type_os2_vendor_id);
    result.open_type_os2_weight_class = info.open_type_os2_weight_class.map(u32::from);
    result.open_type_os2_width_class = info.open_type_os2_width_class.and_then(width_class);
    result.open_type_os2_selection = optional_vec(
        &info
            .open_type_os2_selection
            .iter()
            .copied()
            .filter(|bit| !matches!(bit, 0 | 5 | 6))
            .collect::<Vec<_>>(),
    );
    result.open_type_os2_type = optional_vec(&info.open_type_os2_type);
    result.open_type_os2_panose = info.open_type_os2_panose.map(array_to_panose);
    result.open_type_os2_family_class =
        info.open_type_os2_family_class
            .map(|(class_id, subclass_id)| Os2FamilyClass {
                class_id,
                subclass_id,
            });
    result.open_type_os2_unicode_ranges = optional_vec(&info.open_type_os2_unicode_ranges);
    result.open_type_os2_code_page_ranges = optional_vec(&info.open_type_os2_code_page_ranges);
    result.open_type_os2_subscript_x_size = rounded(info.open_type_os2_subscript_x_size);
    result.open_type_os2_subscript_y_size = rounded(info.open_type_os2_subscript_y_size);
    result.open_type_os2_subscript_x_offset = rounded(info.open_type_os2_subscript_x_offset);
    result.open_type_os2_subscript_y_offset = rounded(info.open_type_os2_subscript_y_offset);
    result.open_type_os2_superscript_x_size = rounded(info.open_type_os2_superscript_x_size);
    result.open_type_os2_superscript_y_size = rounded(info.open_type_os2_superscript_y_size);
    result.open_type_os2_superscript_x_offset = rounded(info.open_type_os2_superscript_x_offset);
    result.open_type_os2_superscript_y_offset = rounded(info.open_type_os2_superscript_y_offset);
    result.open_type_os2_strikeout_size = rounded(info.open_type_os2_strikeout_size);
    result.open_type_os2_strikeout_position = rounded(info.open_type_os2_strikeout_position);
    result.open_type_head_created = info
        .open_type_head_created
        .as_deref()
        .filter(|value| valid_ufo_date(value))
        .map(ToOwned::to_owned);
    result.open_type_head_flags = optional_vec(&info.open_type_head_flags);
    result.open_type_head_lowest_rec_ppem = info.open_type_head_lowest_rec_ppem.map(u32::from);
    result.open_type_gasp_range_records = (!info.open_type_gasp_range_records.is_empty())
        .then(|| norad_gasp(&info.open_type_gasp_range_records));
    result.open_type_hhea_caret_slope_rise = info.open_type_hhea_caret_slope_rise;
    result.open_type_hhea_caret_slope_run = info.open_type_hhea_caret_slope_run;
    result.open_type_hhea_caret_offset = info.open_type_hhea_caret_offset;
    result.open_type_vhea_caret_slope_rise = info.open_type_vhea_caret_slope_rise;
    result.open_type_vhea_caret_slope_run = info.open_type_vhea_caret_slope_run;
    result.open_type_vhea_caret_offset = info.open_type_vhea_caret_offset;
    result.postscript_is_fixed_pitch = info.postscript_is_fixed_pitch;
    result.postscript_default_width_x = info.postscript_default_width_x;
    result.postscript_nominal_width_x = info.postscript_nominal_width_x;
    result.postscript_underline_position = info.postscript_underline_position;
    result.postscript_underline_thickness = info.postscript_underline_thickness;
    result.postscript_unique_id = info.postscript_unique_id;
    result
        .macintosh_fond_name
        .clone_from(&info.macintosh_fond_name);
    result.macintosh_fond_family_id = info.macintosh_fond_family_id;

    let metrics = &master.metrics;
    set_metric(&mut result.ascender, metrics.ascender);
    set_metric(&mut result.descender, metrics.descender);
    set_metric(&mut result.x_height, metrics.x_height);
    set_metric(&mut result.cap_height, metrics.cap_height);
    set_metric(&mut result.italic_angle, metrics.italic_angle);
    result.open_type_hhea_ascender = rounded(metrics.hhea_ascender);
    result.open_type_hhea_descender = rounded(metrics.hhea_descender);
    result.open_type_hhea_line_gap = rounded(metrics.hhea_line_gap);
    result.open_type_os2_typo_ascender = rounded(metrics.typo_ascender);
    result.open_type_os2_typo_descender = rounded(metrics.typo_descender);
    result.open_type_os2_typo_line_gap = rounded(metrics.typo_line_gap);
    result.open_type_os2_win_ascent = nonnegative_rounded(metrics.win_ascent);
    result.open_type_os2_win_descent = nonnegative_rounded(metrics.win_descent);
    result.open_type_vhea_vert_typo_ascender = rounded(metrics.vhea_ascender);
    result.open_type_vhea_vert_typo_descender = rounded(metrics.vhea_descender);
    result.open_type_vhea_vert_typo_line_gap = rounded(metrics.vhea_line_gap);

    let private = &master.private_dict;
    result.postscript_blue_values = optional_vec(&private.blue_values);
    result.postscript_other_blues = optional_vec(&private.other_blues);
    result.postscript_family_blues = optional_vec(&private.family_blues);
    result.postscript_family_other_blues = optional_vec(&private.family_other_blues);
    result.postscript_blue_scale = private.blue_scale;
    result.postscript_blue_shift = private.blue_shift;
    result.postscript_blue_fuzz = private.blue_fuzz;
    result.postscript_stem_snap_h = optional_vec(&private.stem_snap_h);
    result.postscript_stem_snap_v = optional_vec(&private.stem_snap_v);
    result.postscript_force_bold = private.force_bold;

    if !guides.is_empty() || result.guidelines.is_some() {
        result.guidelines = Some(
            guides
                .iter()
                .enumerate()
                .map(|(index, guide)| {
                    guide_to_norad(guide, guide_ids.get(index).and_then(Option::as_deref))
                })
                .collect::<Result<_, _>>()?,
        );
    }
    Ok(result)
}

pub(crate) fn guide_from_norad(guide: &Guideline) -> Guide {
    let (x, y, angle) = match guide.line {
        Line::Vertical(x) => (x, 0.0, 90.0),
        Line::Horizontal(y) => (0.0, y, 0.0),
        Line::Angle { x, y, degrees } => (x, y, degrees),
    };
    Guide {
        id: guide.identifier().map(ToString::to_string),
        x,
        y,
        angle,
        name: guide.name.as_ref().map(ToString::to_string),
        locked: false,
        color: guide.color.map(|color| color.to_rgba_string()),
        ..Guide::default()
    }
}

pub(crate) fn guide_to_norad(guide: &Guide, identifier: Option<&str>) -> Result<Guideline, Error> {
    let identifier = guide.id.as_deref().or(identifier);
    let line = Line::Angle {
        x: guide.x,
        y: guide.y,
        degrees: guide.angle.rem_euclid(360.0),
    };
    let name = guide
        .name
        .as_deref()
        .map(norad::Name::new)
        .transpose()
        .map_err(|_| Error::InvalidName(guide.name.clone().unwrap_or_default()))?;
    let color = guide
        .color
        .as_deref()
        .map(Color::from_str)
        .transpose()
        .map_err(|_| Error::UnsupportedPlist("invalid UFO color".to_owned()))?;
    let identifier = identifier
        .map(Identifier::new)
        .transpose()
        .map_err(|_| Error::InvalidIdentifier(identifier.unwrap_or_default().to_owned()))?;
    Ok(Guideline::new(line, name, color, identifier))
}

fn optional_string(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_owned())
}

fn valid_ufo_date(value: &str) -> bool {
    value.len() == 19
        && value.bytes().enumerate().all(|(index, byte)| match index {
            4 | 7 => byte == b'/',
            10 => byte == b' ',
            13 | 16 => byte == b':',
            _ => byte.is_ascii_digit(),
        })
}

fn optional_vec<T: Clone>(value: &[T]) -> Option<Vec<T>> {
    (!value.is_empty()).then(|| value.to_vec())
}

fn rounded(value: Option<f64>) -> Option<i32> {
    value.map(|value| {
        value
            .round()
            .clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
    })
}

fn nonnegative_rounded(value: Option<f64>) -> Option<u32> {
    value.map(|value| value.round().clamp(0.0, f64::from(u32::MAX)) as u32)
}

fn set_metric(target: &mut Option<f64>, value: f64) {
    if value != 0.0 || target.is_some() {
        *target = Some(value);
    }
}

fn model_names(names: Option<&[norad::fontinfo::NameRecord]>) -> Vec<NameRecord> {
    names
        .unwrap_or_default()
        .iter()
        .filter_map(|record| {
            Some(NameRecord {
                name_id: u16::try_from(record.name_id).ok()?,
                language: format!(
                    "ufo:{}:{}:{}",
                    record.platform_id, record.encoding_id, record.language_id
                ),
                text: record.string.clone(),
                platform: match record.platform_id {
                    0 => NamePlatform::Unicode,
                    1 => NamePlatform::Mac,
                    3 => NamePlatform::Windows,
                    other => NamePlatform::Other(u16::try_from(other).unwrap_or(u16::MAX)),
                },
            })
        })
        .collect()
}

fn norad_names(names: &[NameRecord]) -> Vec<norad::fontinfo::NameRecord> {
    names
        .iter()
        .map(|record| {
            let ids = record.language.strip_prefix("ufo:").and_then(|value| {
                let mut values = value.split(':').filter_map(|part| part.parse::<u32>().ok());
                Some((values.next()?, values.next()?, values.next()?))
            });
            let (platform_id, encoding_id, language_id) = ids.unwrap_or(match record.platform {
                NamePlatform::Windows => (3, 1, 0x0409),
                NamePlatform::Mac => (1, 0, 0),
                NamePlatform::Unicode => (0, 4, 0),
                NamePlatform::Other(platform) => (u32::from(platform), 0, 0),
            });
            norad::fontinfo::NameRecord {
                name_id: u32::from(record.name_id),
                platform_id,
                encoding_id,
                language_id,
                string: record.text.clone(),
            }
        })
        .collect()
}

fn model_gasp(record: &GaspRangeRecord) -> Option<GaspRecord> {
    Some(GaspRecord {
        range_max_ppem: u16::try_from(record.range_max_ppem).ok()?,
        behavior: record
            .range_gasp_behavior
            .iter()
            .map(|value| *value as u8)
            .collect(),
    })
}

fn norad_gasp(records: &[GaspRecord]) -> Vec<GaspRangeRecord> {
    records
        .iter()
        .map(|record| GaspRangeRecord {
            range_max_ppem: u32::from(record.range_max_ppem),
            range_gasp_behavior: record
                .behavior
                .iter()
                .filter_map(|value| match value {
                    0 => Some(GaspBehavior::Gridfit),
                    1 => Some(GaspBehavior::DoGray),
                    2 => Some(GaspBehavior::SymmetricGridfit),
                    3 => Some(GaspBehavior::SymmetricSmoothing),
                    _ => None,
                })
                .collect(),
        })
        .collect()
}

fn panose_to_array(value: Os2Panose) -> [u8; 10] {
    [
        value.family_type,
        value.serif_style,
        value.weight,
        value.proportion,
        value.contrast,
        value.stroke_variation,
        value.arm_style,
        value.letterform,
        value.midline,
        value.x_height,
    ]
    .map(|value| u8::try_from(value).unwrap_or(u8::MAX))
}

fn array_to_panose(value: [u8; 10]) -> Os2Panose {
    Os2Panose {
        family_type: u32::from(value[0]),
        serif_style: u32::from(value[1]),
        weight: u32::from(value[2]),
        proportion: u32::from(value[3]),
        contrast: u32::from(value[4]),
        stroke_variation: u32::from(value[5]),
        arm_style: u32::from(value[6]),
        letterform: u32::from(value[7]),
        midline: u32::from(value[8]),
        x_height: u32::from(value[9]),
    }
}

fn width_class(value: u16) -> Option<Os2WidthClass> {
    match value {
        1 => Some(Os2WidthClass::UltraCondensed),
        2 => Some(Os2WidthClass::ExtraCondensed),
        3 => Some(Os2WidthClass::Condensed),
        4 => Some(Os2WidthClass::SemiCondensed),
        5 => Some(Os2WidthClass::Normal),
        6 => Some(Os2WidthClass::SemiExpanded),
        7 => Some(Os2WidthClass::Expanded),
        8 => Some(Os2WidthClass::ExtraExpanded),
        9 => Some(Os2WidthClass::UltraExpanded),
        _ => None,
    }
}
