//! Parsing of "Add Glyphs" recipes into glyph names and Unicode values.
//!
//! Supported items, separated by whitespace, commas, or semicolons:
//! `A`, `uni4E00`, `U+0041` (single glyphs); `A-Z`, `uni4E00-uni4E1F`,
//! `U+0041-U+005A` (inclusive ranges); `a.sc`, `f_i` (unencoded variants and
//! ligatures); and `@charset:GB2312` (a legacy CJK repertoire).

use fw_glyphdata::{cjk_charset_membership, name_for_unicode, unicode_for_name, CjkCharsets};
use fw_model::GlyphName;

/// One resolved item of a recipe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RecipeItem {
    /// A glyph to create, with its Unicode value when the name implies one.
    Glyph {
        /// Name of the glyph.
        name: GlyphName,
        /// Unicode scalar value, absent for suffixed and ligature names.
        unicode: Option<u32>,
    },
    /// An item that could not be understood.
    Unknown(String),
}

/// The highest scalar value scanned when expanding a legacy CJK charset.
const CHARSET_SCAN_LIMIT: u32 = 0xFFFF;

/// Parses a recipe into the glyphs it asks for, in recipe order.
pub(crate) fn parse(recipe: &str) -> Vec<RecipeItem> {
    let mut items = Vec::new();
    for token in recipe.split([' ', '\t', '\r', '\n', ',', ';']) {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        if let Some(charset) = token.strip_prefix("@charset:") {
            expand_charset(charset, token, &mut items);
            continue;
        }
        if let Some(range) = range(token) {
            items.extend(range);
            continue;
        }
        items.push(single(token));
    }
    items
}

fn single(token: &str) -> RecipeItem {
    if let Some(unicode) = literal_scalar(token) {
        return encoded(unicode);
    }
    if !is_glyph_name(token) {
        return RecipeItem::Unknown(token.to_owned());
    }
    // A suffix or a ligature underscore names a variant, never an encoded glyph.
    let unicode = if token.contains('.') || token.contains('_') {
        None
    } else {
        unicode_for_name(token)
    };
    RecipeItem::Glyph {
        name: GlyphName::from(token),
        unicode,
    }
}

fn range(token: &str) -> Option<Vec<RecipeItem>> {
    let (start, end) = token.split_once('-')?;
    let start = scalar(start)?;
    let end = scalar(end)?;
    if start > end {
        return None;
    }
    Some(
        (start..=end)
            .filter_map(char::from_u32)
            .map(|character| encoded(u32::from(character)))
            .collect(),
    )
}

fn expand_charset(name: &str, token: &str, items: &mut Vec<RecipeItem>) {
    let Some(charset) = charset(name) else {
        items.push(RecipeItem::Unknown(token.to_owned()));
        return;
    };
    let before = items.len();
    for unicode in 0..=CHARSET_SCAN_LIMIT {
        if !(cjk_charset_membership(unicode) & charset).is_empty() {
            items.push(encoded(unicode));
        }
    }
    if items.len() == before {
        // A repertoire whose table is not compiled in yet yields nothing.
        items.push(RecipeItem::Unknown(token.to_owned()));
    }
}

fn charset(name: &str) -> Option<CjkCharsets> {
    let normalized: String = name
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    Some(match normalized.as_str() {
        "gb2312" => CjkCharsets::GB2312,
        "gb18030" | "gb18030level1" => CjkCharsets::GB18030_LEVEL1,
        "big5" => CjkCharsets::BIG5,
        "jis" | "jisx0208" => CjkCharsets::JIS_X_0208,
        "ks" | "ksx1001" => CjkCharsets::KS_X_1001,
        _ => return None,
    })
}

fn encoded(unicode: u32) -> RecipeItem {
    RecipeItem::Glyph {
        name: GlyphName::from(name_for_unicode(unicode)),
        unicode: Some(unicode),
    }
}

/// Resolves one range endpoint to a scalar value.
fn scalar(token: &str) -> Option<u32> {
    let token = token.trim();
    if let Some(unicode) = literal_scalar(token) {
        return Some(unicode);
    }
    let mut characters = token.chars();
    match (characters.next(), characters.next()) {
        (Some(character), None) => Some(u32::from(character)),
        _ => unicode_for_name(token),
    }
}

/// Parses `U+0041` and `u+0041` spellings.
fn literal_scalar(token: &str) -> Option<u32> {
    let digits = token
        .strip_prefix("U+")
        .or_else(|| token.strip_prefix("u+"))?;
    let unicode = u32::from_str_radix(digits, 16).ok()?;
    char::from_u32(unicode).map(u32::from)
}

fn is_glyph_name(token: &str) -> bool {
    let mut characters = token.chars();
    let first = characters.next();
    if !first.is_some_and(|first| first.is_ascii_alphabetic() || first == '_' || first == '.') {
        return false;
    }
    token
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '_'))
}

#[cfg(test)]
mod tests {
    use super::{parse, RecipeItem};

    fn names(recipe: &str) -> Vec<String> {
        parse(recipe)
            .into_iter()
            .map(|item| match item {
                RecipeItem::Glyph { name, .. } => name.to_string(),
                RecipeItem::Unknown(token) => format!("?{token}"),
            })
            .collect()
    }

    #[test]
    fn expands_latin_and_production_ranges() {
        assert_eq!(names("A-C"), ["A", "B", "C"]);
        assert_eq!(names("uni4E00-uni4E02"), ["uni4E00", "uni4E01", "uni4E02"]);
        assert_eq!(names("U+0041-U+0042"), ["A", "B"]);
    }

    #[test]
    fn keeps_suffixed_and_ligature_names_unencoded() {
        assert_eq!(
            parse("a.sc f_i"),
            vec![
                RecipeItem::Glyph {
                    name: "a.sc".into(),
                    unicode: None
                },
                RecipeItem::Glyph {
                    name: "f_i".into(),
                    unicode: None
                },
            ]
        );
    }

    #[test]
    fn resolves_plain_names_to_their_unicode_value() {
        assert_eq!(
            parse("Aacute"),
            vec![RecipeItem::Glyph {
                name: "Aacute".into(),
                unicode: Some(0x00C1)
            }]
        );
    }

    #[test]
    fn reports_unknown_items() {
        assert_eq!(
            names("@charset:Klingon 12three"),
            ["?@charset:Klingon", "?12three"]
        );
    }

    #[test]
    fn expands_a_legacy_charset() {
        let items = parse("@charset:GB2312");
        assert!(
            items.len() > 6_000,
            "GB 2312 expanded to {} items",
            items.len()
        );
        assert!(items.contains(&RecipeItem::Glyph {
            name: "uni4E00".into(),
            unicode: Some(0x4E00)
        }));
    }
}
