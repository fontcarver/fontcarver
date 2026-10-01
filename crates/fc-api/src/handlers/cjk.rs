//! Unicode-backed CJK search, filters and metadata.
use crate::{
    Api, ApiError, CjkComponentUse, CjkFilter, CjkRadicalEntry, CjkRadicalForm, CjkRadicalLibrary,
    CjkRadicalsParams, UnihanInfo, UnihanParams,
};
use fc_glyphdata::{cjk_charset_membership, kangxi_radicals, unihan, CjkCharsets};
use std::collections::{BTreeMap, HashMap, HashSet};

/// Largest component-base list the radical library returns.
const MAX_COMPONENT_BASES: usize = 2_000;

pub(crate) fn info(_: &Api, params: UnihanParams) -> Result<Option<UnihanInfo>, ApiError> {
    if char::from_u32(params.unicode).is_none() {
        return Err(ApiError::invalid_params("invalid Unicode scalar"));
    }
    Ok(unihan(params.unicode).map(|data| UnihanInfo {
        version: fc_glyphdata::UNIHAN_VERSION.into(),
        unicode: data.unicode,
        radical_strokes: data.radical_strokes.into(),
        total_strokes: data.total_strokes.into(),
        mandarin: data.mandarin.into(),
        cantonese: data.cantonese.into(),
        japanese_on: data.japanese_on.into(),
        japanese_kun: data.japanese_kun.into(),
        korean: data.korean.into(),
        iicore: data.iicore.into(),
    }))
}

pub(crate) fn validate(filter: &CjkFilter) -> Result<(), ApiError> {
    if filter.radical.is_some_and(|r| !(1..=214).contains(&r))
        || filter.strokes.is_some_and(|[a, b]| a > b || b > 255)
        || filter.reading.as_ref().is_some_and(|r| r.len() > 128)
        || filter
            .language
            .as_deref()
            .is_some_and(|l| !["mandarin", "cantonese", "japanese", "korean", "any"].contains(&l))
        || filter.charsets.len() > 6
        || filter
            .charsets
            .iter()
            .any(|c| c != "iicore" && charset(c).is_none())
    {
        return Err(ApiError::invalid_params(
            "invalid CJK filter range, reading language or charset",
        ));
    }
    Ok(())
}

pub(crate) fn matches(cp: u32, filter: &CjkFilter) -> bool {
    let data = unihan(cp);
    if !filter.charsets.is_empty()
        && !filter.charsets.iter().any(|name| {
            if name == "iicore" {
                data.is_some_and(|d| !d.iicore.is_empty())
            } else {
                charset(name).is_some_and(|flag| cjk_charset_membership(cp).contains(flag))
            }
        })
    {
        return false;
    }
    if filter
        .radical
        .is_some_and(|radical| !data.is_some_and(|d| d.radicals().any(|r| r.radical == radical)))
    {
        return false;
    }
    if filter.strokes.is_some_and(|[min, max]| {
        !data.is_some_and(|d| d.strokes().any(|count| (min..=max).contains(&count)))
    }) {
        return false;
    }
    if let Some(reading) = filter.reading.as_deref().filter(|r| !r.trim().is_empty()) {
        if !data.is_some_and(|d| {
            d.matches_reading(
                reading.trim(),
                filter.language.as_deref().unwrap_or("mandarin"),
            )
        }) {
            return false;
        }
    }
    true
}

pub(crate) fn query(glyph: &fc_model::Glyph, text: &str) -> Option<bool> {
    if !text.split_whitespace().any(|token| {
        ["radical:", "strokes:", "pinyin:", "reading:", "charset:"]
            .iter()
            .any(|prefix| token.starts_with(prefix))
    }) {
        return None;
    }
    let mut filters = Vec::new();
    for token in text.split_whitespace() {
        // Each condition must hold for the same encoding, including glyphs with
        // multiple Unicode assignments. Repeated conditions remain conjunctive.
        let parsed = (|| {
            let Some((key, value)) = token.split_once(':') else {
                return false;
            };
            let mut filter = CjkFilter::default();
            match key {
                "radical" => {
                    let Ok(radical) = value.parse() else {
                        return false;
                    };
                    filter.radical = Some(radical);
                }
                "strokes" => {
                    let (min, max) = value.split_once('-').unwrap_or((value, value));
                    let (Ok(min), Ok(max)) = (min.parse(), max.parse()) else {
                        return false;
                    };
                    filter.strokes = Some([min, max]);
                }
                "pinyin" | "reading" => {
                    filter.reading = Some(value.into());
                    filter.language = Some(if key == "pinyin" { "mandarin" } else { "any" }.into());
                }
                "charset" => filter.charsets.push(value.into()),
                _ => return false,
            }
            if validate(&filter).is_err() {
                return false;
            }
            filters.push(filter);
            true
        })();
        if !parsed {
            return Some(false);
        }
    }
    Some(
        glyph
            .unicodes
            .iter()
            .any(|cp| filters.iter().all(|filter| matches(*cp, filter))),
    )
}

fn charset(name: &str) -> Option<CjkCharsets> {
    Some(match name {
        "gb2312" => CjkCharsets::GB2312,
        "gb18030level1" => CjkCharsets::GB18030_LEVEL1,
        "big5" => CjkCharsets::BIG5,
        "jisx0208" => CjkCharsets::JIS_X_0208,
        "ksx1001" => CjkCharsets::KS_X_1001,
        _ => return None,
    })
}

/// Builds the radical library: pinned Unihan radicals plus the font's components.
///
/// Both halves come from data rather than a hard-coded list: the radicals and
/// their written forms are derived from the pinned `kRSUnicode` values, and the
/// component bases are the glyphs this document's layers actually reference.
pub(crate) fn radicals(
    api: &Api,
    params: CjkRadicalsParams,
) -> Result<CjkRadicalLibrary, ApiError> {
    api.session
        .with_doc(params.doc, |document| {
            let font = &document.font;
            // Only the radical scalars need a reverse encoding map, so the
            // whole 65,535-glyph cmap is never materialised.
            let mut wanted: HashSet<u32> = HashSet::new();
            for radical in kangxi_radicals() {
                wanted.insert(radical.kangxi);
                wanted.extend(radical.forms.iter().map(|form| form.unicode));
            }
            let mut encoded: HashMap<u32, &str> = HashMap::new();
            let mut members = vec![0u32; kangxi_radicals().len()];
            let mut uses: BTreeMap<&str, u32> = BTreeMap::new();
            let mut seen = Vec::new();
            for (name, glyph) in &font.glyphs {
                for unicode in &glyph.unicodes {
                    if wanted.contains(unicode) {
                        // The first glyph in persistent order wins, so the
                        // library is stable across reloads.
                        encoded.entry(*unicode).or_insert(name.as_ref());
                    }
                }
                // A glyph with two encodings of the same radical counts once.
                seen.clear();
                for unicode in &glyph.unicodes {
                    let Some(record) = unihan(*unicode) else {
                        continue;
                    };
                    for value in record.radicals() {
                        if seen.contains(&value.radical) {
                            continue;
                        }
                        seen.push(value.radical);
                        if let Some(slot) =
                            members.get_mut(usize::from(value.radical).wrapping_sub(1))
                        {
                            *slot += 1;
                        }
                    }
                }
                for layer in &glyph.layers {
                    for component in &layer.components {
                        *uses.entry(component.base.as_ref()).or_default() += 1;
                    }
                }
            }
            let radicals = kangxi_radicals()
                .iter()
                .zip(&members)
                .map(|(radical, members)| CjkRadicalEntry {
                    number: radical.number,
                    kangxi: radical.kangxi,
                    kangxi_glyph: encoded.get(&radical.kangxi).map(ToString::to_string),
                    forms: radical
                        .forms
                        .iter()
                        .map(|form| CjkRadicalForm {
                            unicode: form.unicode,
                            strokes: form.strokes,
                            glyph: encoded.get(&form.unicode).map(ToString::to_string),
                        })
                        .collect(),
                    members: *members,
                })
                .collect();
            let mut components: Vec<CjkComponentUse> = uses
                .into_iter()
                .map(|(name, uses)| CjkComponentUse {
                    name: name.to_owned(),
                    uses,
                    unicode: font
                        .glyph(name)
                        .and_then(|glyph| glyph.unicodes.first().copied()),
                })
                .collect();
            // Most used first; the BTreeMap already settled the name order.
            components.sort_by(|left, right| right.uses.cmp(&left.uses));
            let components_truncated = components.len() > MAX_COMPONENT_BASES;
            components.truncate(MAX_COMPONENT_BASES);
            Ok(CjkRadicalLibrary {
                unihan_version: fc_glyphdata::UNIHAN_VERSION.into(),
                radicals,
                components,
                components_truncated,
            })
        })
        .map_err(ApiError::from)?
}
