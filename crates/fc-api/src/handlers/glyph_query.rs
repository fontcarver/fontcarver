//! Bounded, compiled glyph-name, Unicode and note searches.

use fc_model::Glyph;
use regex::{Regex, RegexBuilder};

use crate::{ApiError, GlyphFilter, GlyphQueryScope};

pub(crate) struct GlyphQuery {
    query: String,
    scope: GlyphQueryScope,
    case_sensitive: bool,
    regex: Option<Regex>,
}

impl GlyphQuery {
    pub(crate) fn new(filter: &GlyphFilter) -> Result<Option<Self>, ApiError> {
        let Some(query) = &filter.query else {
            return Ok(None);
        };
        if query.len() > 4096 {
            return Err(ApiError::invalid_params("glyph search exceeds 4096 bytes"));
        }
        let case_sensitive = filter.case_sensitive.unwrap_or(false);
        let regex = if filter.regex.unwrap_or(false) {
            Some(
                RegexBuilder::new(query)
                    .case_insensitive(!case_sensitive)
                    .size_limit(1 << 20)
                    .dfa_size_limit(1 << 20)
                    .nest_limit(64)
                    .build()
                    .map_err(|error| {
                        ApiError::invalid_params(format!("invalid glyph search regex: {error}"))
                    })?,
            )
        } else {
            None
        };
        Ok(Some(Self {
            query: if regex.is_some() {
                query.clone()
            } else {
                query.trim().to_owned()
            },
            scope: filter.query_scope.unwrap_or_default(),
            case_sensitive,
            regex,
        }))
    }

    pub(crate) fn rank(&self, glyph: &Glyph) -> Option<u8> {
        if self.query.is_empty() {
            return Some(0);
        }
        if matches!(self.scope, GlyphQueryScope::All | GlyphQueryScope::Unicode)
            && self.regex.is_none()
        {
            if let Some(matches) = super::cjk::query(glyph, &self.query) {
                return matches.then_some(0);
            }
        }
        if matches!(self.scope, GlyphQueryScope::All | GlyphQueryScope::Name) {
            if let Some(rank) = self.text_rank(glyph.name.as_ref()) {
                return Some(rank);
            }
        }
        if matches!(self.scope, GlyphQueryScope::All | GlyphQueryScope::Unicode)
            && self.unicode_matches(glyph)
        {
            return Some(4);
        }
        if matches!(self.scope, GlyphQueryScope::All | GlyphQueryScope::Note)
            && self.text_rank(&glyph.note).is_some()
        {
            return Some(5);
        }
        None
    }

    fn text_rank(&self, haystack: &str) -> Option<u8> {
        if let Some(regex) = &self.regex {
            return regex.is_match(haystack).then_some(3);
        }
        if haystack == self.query {
            return Some(0);
        }
        if self.case_sensitive {
            return haystack.contains(&self.query).then_some(3);
        }
        let lower = self.query.to_lowercase();
        let haystack = haystack.to_lowercase();
        if haystack == lower {
            Some(1)
        } else if haystack.starts_with(&lower) {
            Some(2)
        } else {
            haystack.contains(&lower).then_some(3)
        }
    }

    fn unicode_matches(&self, glyph: &Glyph) -> bool {
        if let Some(regex) = &self.regex {
            return glyph.unicodes.iter().any(|scalar| {
                regex.is_match(&format!("U+{scalar:04X}"))
                    || char::from_u32(*scalar).is_some_and(|ch| regex.is_match(&ch.to_string()))
            });
        }
        let lower = self.query.to_lowercase();
        if let Some(digits) = lower.strip_prefix("u+") {
            return u32::from_str_radix(digits, 16)
                .ok()
                .filter(|cp| char::from_u32(*cp).is_some())
                .is_some_and(|cp| glyph.unicodes.contains(&cp));
        }
        if lower.starts_with("uni") {
            if let Some(cp) = fc_glyphdata::unicode_for_name(&lower) {
                return glyph.unicodes.contains(&cp);
            }
        }
        let mut characters = self.query.chars();
        if let (Some(character), None) = (characters.next(), characters.next()) {
            return glyph.unicodes.contains(&u32::from(character));
        }
        // Plain hexadecimal notation is useful in the explicit Unicode scope.
        self.scope == GlyphQueryScope::Unicode
            && u32::from_str_radix(&self.query, 16)
                .ok()
                .filter(|cp| char::from_u32(*cp).is_some())
                .is_some_and(|cp| glyph.unicodes.contains(&cp))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glyph() -> Glyph {
        Glyph {
            unicodes: vec![0x41],
            note: "Review Uppercase shape".into(),
            ..Glyph::new("A.sc")
        }
    }

    fn query(query: &str, scope: GlyphQueryScope, case_sensitive: bool, regex: bool) -> GlyphQuery {
        GlyphQuery::new(&GlyphFilter {
            query: Some(query.into()),
            query_scope: Some(scope),
            case_sensitive: Some(case_sensitive),
            regex: Some(regex),
            ..GlyphFilter::default()
        })
        .unwrap()
        .unwrap()
    }

    #[test]
    fn glyph_query_searches_scopes_notes_case_and_unicode() {
        let glyph = glyph();
        assert!(query("review", GlyphQueryScope::All, false, false)
            .rank(&glyph)
            .is_some());
        assert!(query("review", GlyphQueryScope::Name, false, false)
            .rank(&glyph)
            .is_none());
        assert!(query("review", GlyphQueryScope::Note, true, false)
            .rank(&glyph)
            .is_none());
        assert!(query("Review", GlyphQueryScope::Note, true, false)
            .rank(&glyph)
            .is_some());
        assert!(query("a.sc", GlyphQueryScope::Name, true, false)
            .rank(&glyph)
            .is_none());
        for text in ["U+0041", "uni0041", "0041", "A"] {
            assert!(
                query(text, GlyphQueryScope::Unicode, false, false)
                    .rank(&glyph)
                    .is_some(),
                "{text}"
            );
        }
        assert!(query("U+D800", GlyphQueryScope::Unicode, false, false)
            .rank(&glyph)
            .is_none());
    }

    #[test]
    fn glyph_query_regex_is_scoped_bounded_and_preserves_whitespace() {
        let glyph = glyph();
        assert!(query(r"^a\.sc$", GlyphQueryScope::Name, false, true)
            .rank(&glyph)
            .is_some());
        assert!(query(r"^a\.sc$", GlyphQueryScope::Name, true, true)
            .rank(&glyph)
            .is_none());
        assert!(query(" Uppercase ", GlyphQueryScope::Note, true, true)
            .rank(&glyph)
            .is_some());
        assert!(query("^U\\+00[34]1$", GlyphQueryScope::Unicode, true, true)
            .rank(&glyph)
            .is_some());
        assert!(GlyphQuery::new(&GlyphFilter {
            query: Some("[".into()),
            regex: Some(true),
            ..GlyphFilter::default()
        })
        .is_err());
        assert!(GlyphQuery::new(&GlyphFilter {
            query: Some("x".repeat(4097)),
            ..GlyphFilter::default()
        })
        .is_err());
    }
}
