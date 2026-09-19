//! Native naming properties and metric custom parameters.

use super::{error, plist::*};
use crate::Error;
use fc_model::{FontInfo, MasterMetrics};
use glyphs_reader::{CustomParameters, Plist};

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub(super) struct AxisLimits {
    pub min: f64,
    pub max: f64,
    pub native_locations: Vec<f64>,
    #[serde(default)]
    pub native_map: Vec<(f64, f64)>,
}

pub(super) fn write_axis_limits(
    raw: &mut Plist,
    font: &fc_model::Font,
    axis: &fc_model::Axis,
) -> Result<(), Error> {
    let mut native_locations = font
        .masters
        .iter()
        .map(|master| {
            master
                .location
                .get(&axis.tag)
                .copied()
                .unwrap_or(axis.default)
        })
        .collect::<Vec<_>>();
    native_locations.sort_by(f64::total_cmp);
    let limits = AxisLimits {
        min: axis.min,
        max: axis.max,
        native_locations,
        native_map: if axis.map.iter().all(|(user, design)| user == design) {
            Vec::new()
        } else {
            let mut map = axis.map.clone();
            map.sort_by(|a, b| a.0.total_cmp(&b.0));
            map
        },
    };
    super::extras::store(
        raw,
        &limits,
        &["min", "max", "native_locations", "native_map"],
    )
}

pub(super) fn user_coordinate(map: &[(f64, f64)], design: f64) -> Result<f64, Error> {
    if map.is_empty() {
        return Ok(design);
    }
    if map
        .iter()
        .any(|(user, design)| !user.is_finite() || !design.is_finite())
        || map.windows(2).any(|w| w[0].0 >= w[1].0 || w[0].1 >= w[1].1)
    {
        return Err(error(
            "axis mappings must be finite and strictly increasing",
        ));
    }
    if map.len() == 1 {
        return finite_coordinate(design + map[0].0 - map[0].1);
    }
    let pair = map
        .windows(2)
        .find(|w| design <= w[1].1)
        .unwrap_or(&map[map.len() - 2..]);
    finite_coordinate(
        pair[0].0 + (design - pair[0].1) / (pair[1].1 - pair[0].1) * (pair[1].0 - pair[0].0),
    )
}

fn finite_coordinate(value: f64) -> Result<f64, Error> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(error("axis mapping overflows its coordinate range"))
    }
}

pub(super) fn metric_definitions(source: &Plist) -> Result<Vec<Plist>, Error> {
    let mut metrics = array(source, "metrics")?.to_vec();
    for name in [
        "ascender",
        "descender",
        "x-height",
        "cap height",
        "italic angle",
    ] {
        if !metrics
            .iter()
            .any(|metric| text(metric, "type") == Some(name))
        {
            let mut metric = dict();
            set(&mut metric, "type", string(name));
            metrics.push(metric);
        }
    }
    Ok(metrics)
}

pub(super) fn metric_values(
    source: &Plist,
    master: &Plist,
    metrics: &MasterMetrics,
) -> Result<Plist, Error> {
    let old = array(master, "metricValues")?;
    Ok(Plist::Array(
        metric_definitions(source)?
            .iter()
            .enumerate()
            .map(|(index, definition)| {
                let mut value = old.get(index).cloned().unwrap_or_else(dict);
                let position = match text(definition, "type") {
                    Some("ascender") => Some(metrics.ascender),
                    Some("descender") => Some(metrics.descender),
                    Some("x-height") => Some(metrics.x_height),
                    Some("cap height") => Some(metrics.cap_height),
                    Some("italic angle") => Some(metrics.italic_angle),
                    _ => None,
                };
                if let Some(position) = position {
                    set(&mut value, "pos", number(position)?);
                }
                Ok(value)
            })
            .collect::<Result<_, Error>>()?,
    ))
}

pub(super) fn reconcile_generation(raw: &Plist, font: &mut fc_model::Font) -> Result<(), Error> {
    // Persistent source identities survive ordinary native roundtrips. Editing
    // or removing their native rules freezes recipes instead of overwriting the
    // external author's work during a later materialization.
    let mut changed = false;
    for record in &font.feature_settings.generated {
        let mut matched = false;
        for item in array(raw, "features")? {
            let snapshot: fc_model::FeatureBlock =
                super::extras::restore(item, &["id", "tag", "code", "automatic", "disabled"])?;
            if snapshot.id != record.source_id {
                continue;
            }
            matched = font.features.features.iter().any(|current| {
                current.id == snapshot.id
                    && current.tag == snapshot.tag
                    && current.code == snapshot.code
                    && current.automatic == snapshot.automatic
                    && current.disabled == snapshot.disabled
            });
            break;
        }
        changed |= !matched;
    }
    if changed {
        font.feature_settings = fc_model::FeatureGenerationSettings {
            writer_policy: font.feature_settings.writer_policy.clone(),
            ..Default::default()
        };
    }
    Ok(())
}

pub(super) fn read_info(info: &mut FontInfo, native: &glyphs_reader::Font) {
    for (target, key) in [
        (&mut info.open_type_name_designer_url, "designerURL"),
        (&mut info.open_type_name_manufacturer_url, "manufacturerURL"),
        (&mut info.open_type_name_license_url, "licenseURL"),
        (&mut info.open_type_name_version, "version"),
        (&mut info.open_type_name_unique_id, "uniqueID"),
        (&mut info.open_type_name_description, "descriptions"),
        (&mut info.postscript_font_name, "postscriptFontName"),
        (&mut info.postscript_full_name, "postscriptFullName"),
    ] {
        *target = native.get_default_name(key).map(str::to_owned);
    }
    info.open_type_os2_vendor_id = native.vendor_id().map(str::to_owned);
    let params = &native.custom_parameters;
    info.open_type_os2_type = params
        .fs_type
        .map(|mask| (0..16).filter(|bit| mask & (1 << bit) != 0).collect())
        .unwrap_or_default();
    info.postscript_is_fixed_pitch = params.is_fixed_pitch;
    info.postscript_underline_position = params.underline_position.map(|v| v.into_inner());
    info.postscript_underline_thickness = params.underline_thickness.map(|v| v.into_inner());
    if params.use_typo_metrics == Some(true) && !info.open_type_os2_selection.contains(&7) {
        info.open_type_os2_selection.push(7);
    }
    if params.use_typo_metrics == Some(false) {
        info.open_type_os2_selection.retain(|bit| *bit != 7);
    }
}

pub(super) fn read_metrics(
    metrics: &mut MasterMetrics,
    local: &CustomParameters,
    global: &CustomParameters,
) {
    for (target, local, global) in [
        (
            &mut metrics.hhea_ascender,
            local.hhea_ascender,
            global.hhea_ascender,
        ),
        (
            &mut metrics.hhea_descender,
            local.hhea_descender,
            global.hhea_descender,
        ),
        (
            &mut metrics.hhea_line_gap,
            local.hhea_line_gap,
            global.hhea_line_gap,
        ),
        (
            &mut metrics.typo_ascender,
            local.typo_ascender,
            global.typo_ascender,
        ),
        (
            &mut metrics.typo_descender,
            local.typo_descender,
            global.typo_descender,
        ),
        (
            &mut metrics.typo_line_gap,
            local.typo_line_gap,
            global.typo_line_gap,
        ),
        (&mut metrics.win_ascent, local.win_ascent, global.win_ascent),
        (
            &mut metrics.win_descent,
            local.win_descent,
            global.win_descent,
        ),
        (
            &mut metrics.vhea_ascender,
            local.vhea_ascender,
            global.vhea_ascender,
        ),
        (
            &mut metrics.vhea_descender,
            local.vhea_descender,
            global.vhea_descender,
        ),
        (
            &mut metrics.vhea_line_gap,
            local.vhea_line_gap,
            global.vhea_line_gap,
        ),
    ] {
        *target = local.or(global).map(|v| v as f64);
    }
}

pub(super) fn write_info(raw: &mut Plist, info: &FontInfo) -> Result<(), Error> {
    let mut properties = array(raw, "properties")?.to_vec();
    for (key, value) in [
        ("designerURL", info.open_type_name_designer_url.as_deref()),
        (
            "manufacturerURL",
            info.open_type_name_manufacturer_url.as_deref(),
        ),
        ("licenseURL", info.open_type_name_license_url.as_deref()),
        ("versionString", info.open_type_name_version.as_deref()),
        ("uniqueID", info.open_type_name_unique_id.as_deref()),
        ("postscriptFontName", info.postscript_font_name.as_deref()),
        ("postscriptFullName", info.postscript_full_name.as_deref()),
        ("vendorID", info.open_type_os2_vendor_id.as_deref()),
    ] {
        properties.retain(|p| text(p, "key") != Some(key));
        if let Some(value) = value {
            let mut property = dict();
            set(&mut property, "key", string(key));
            set(&mut property, "value", string(value));
            properties.push(property);
        }
    }
    let previous = properties
        .iter()
        .position(|p| text(p, "key") == Some("descriptions"));
    let mut property = previous
        .map(|index| properties.remove(index))
        .unwrap_or_else(dict);
    if let Some(value) = &info.open_type_name_description {
        set(&mut property, "key", string("descriptions"));
        let mut values = array(&property, "values")?
            .iter()
            .filter(|v| text(v, "language") != Some("dflt"))
            .cloned()
            .collect::<Vec<_>>();
        let mut item = dict();
        set(&mut item, "language", string("dflt"));
        set(&mut item, "value", string(value));
        values.insert(0, item);
        remove(&mut property, &["value"]);
        set(&mut property, "values", Plist::Array(values));
        properties.push(property);
    }
    set(raw, "properties", Plist::Array(properties));
    remove(raw, &["designerURL", "manufacturerURL"]);
    let mut parameters = array(raw, "customParameters")?.to_vec();
    for (key, value) in [
        ("isFixedPitch", info.postscript_is_fixed_pitch.map(boolean)),
        (
            "underlinePosition",
            info.postscript_underline_position.map(number).transpose()?,
        ),
        (
            "underlineThickness",
            info.postscript_underline_thickness
                .map(number)
                .transpose()?,
        ),
        (
            "Use Typo Metrics",
            Some(boolean(info.open_type_os2_selection.contains(&7))),
        ),
        (
            "fsType",
            Some(Plist::Array(
                info.open_type_os2_type
                    .iter()
                    .map(|v| integer(*v))
                    .collect(),
            )),
        ),
    ] {
        replace_parameter(&mut parameters, key, value);
    }
    set(raw, "customParameters", Plist::Array(parameters));
    Ok(())
}

pub(super) fn write_metrics(raw: &mut Plist, metrics: &MasterMetrics) -> Result<(), Error> {
    let mut parameters = array(raw, "customParameters")?.to_vec();
    for (key, value) in [
        ("hheaAscender", metrics.hhea_ascender),
        ("hheaDescender", metrics.hhea_descender),
        ("hheaLineGap", metrics.hhea_line_gap),
        ("typoAscender", metrics.typo_ascender),
        ("typoDescender", metrics.typo_descender),
        ("typoLineGap", metrics.typo_line_gap),
        ("winAscent", metrics.win_ascent),
        ("winDescent", metrics.win_descent),
        ("vheaAscender", metrics.vhea_ascender),
        ("vheaDescender", metrics.vhea_descender),
        ("vheaLineGap", metrics.vhea_line_gap),
    ] {
        let value = value
            .map(|v| {
                if !v.is_finite() || v.fract() != 0. || v < i32::MIN as f64 || v > i32::MAX as f64 {
                    return Err(error(format!("{key} must be a finite integer")));
                }
                Ok(integer(v as i64))
            })
            .transpose()?;
        replace_parameter(&mut parameters, key, value);
    }
    set(raw, "customParameters", Plist::Array(parameters));
    Ok(())
}

fn replace_parameter(parameters: &mut Vec<Plist>, key: &str, value: Option<Plist>) {
    parameters.retain(|p| text(p, "name") != Some(key));
    if let Some(value) = value {
        let mut item = dict();
        set(&mut item, "name", string(key));
        set(&mut item, "value", value);
        parameters.push(item);
    }
}
