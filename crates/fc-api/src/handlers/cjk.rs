//! Unicode-backed CJK search, filters and metadata.
use crate::{Api, ApiError, CjkFilter, UnihanInfo, UnihanParams};
use fc_glyphdata::{cjk_charset_membership, unihan, CjkCharsets};

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
