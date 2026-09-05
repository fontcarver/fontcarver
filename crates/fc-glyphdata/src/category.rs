//! GlyphData-style categories derived from Unicode properties.

use icu_properties::props::{GeneralCategory, Script};
use icu_properties::CodePointMapData;

use crate::unicode::script_value;

/// A broad glyph category suitable for glyph-grid filtering.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Category {
    /// Letters in any script.
    Letter,
    /// Decimal digits and other numeric characters.
    Number,
    /// Punctuation characters.
    Punctuation,
    /// Mathematical, currency, and other symbols.
    Symbol,
    /// Space, line, and paragraph separators.
    Separator,
    /// Combining and spacing marks.
    Mark,
    /// Invisible formatting controls.
    Format,
    /// Private-use characters.
    Private,
    /// Unassigned, control, surrogate, and otherwise uncategorized values.
    Other,
}

#[derive(Clone, Copy)]
struct CategoryMapping {
    general: GeneralCategory,
    category: Category,
    sub_category: &'static str,
}

const CATEGORY_MAPPINGS: &[CategoryMapping] = &[
    mapping(GeneralCategory::Unassigned, Category::Other, "Other"),
    mapping(
        GeneralCategory::UppercaseLetter,
        Category::Letter,
        "Uppercase",
    ),
    mapping(
        GeneralCategory::LowercaseLetter,
        Category::Letter,
        "Lowercase",
    ),
    mapping(
        GeneralCategory::TitlecaseLetter,
        Category::Letter,
        "Uppercase",
    ),
    mapping(
        GeneralCategory::ModifierLetter,
        Category::Letter,
        "Modifier",
    ),
    mapping(GeneralCategory::OtherLetter, Category::Letter, "Other"),
    mapping(
        GeneralCategory::NonspacingMark,
        Category::Mark,
        "Nonspacing",
    ),
    mapping(GeneralCategory::SpacingMark, Category::Mark, "Spacing"),
    mapping(GeneralCategory::EnclosingMark, Category::Mark, "Enclosing"),
    mapping(GeneralCategory::DecimalNumber, Category::Number, "Decimal"),
    mapping(GeneralCategory::LetterNumber, Category::Number, "Other"),
    mapping(GeneralCategory::OtherNumber, Category::Number, "Other"),
    mapping(
        GeneralCategory::SpaceSeparator,
        Category::Separator,
        "Space",
    ),
    mapping(GeneralCategory::LineSeparator, Category::Separator, "Space"),
    mapping(
        GeneralCategory::ParagraphSeparator,
        Category::Separator,
        "Space",
    ),
    mapping(GeneralCategory::Control, Category::Other, "Other"),
    mapping(GeneralCategory::Format, Category::Format, "Other"),
    mapping(GeneralCategory::PrivateUse, Category::Private, "Other"),
    mapping(GeneralCategory::Surrogate, Category::Other, "Other"),
    mapping(
        GeneralCategory::DashPunctuation,
        Category::Punctuation,
        "Dash",
    ),
    mapping(
        GeneralCategory::OpenPunctuation,
        Category::Punctuation,
        "Parenthesis",
    ),
    mapping(
        GeneralCategory::ClosePunctuation,
        Category::Punctuation,
        "Parenthesis",
    ),
    mapping(
        GeneralCategory::ConnectorPunctuation,
        Category::Punctuation,
        "Other",
    ),
    mapping(
        GeneralCategory::InitialPunctuation,
        Category::Punctuation,
        "Quote",
    ),
    mapping(
        GeneralCategory::FinalPunctuation,
        Category::Punctuation,
        "Quote",
    ),
    mapping(
        GeneralCategory::OtherPunctuation,
        Category::Punctuation,
        "Other",
    ),
    mapping(GeneralCategory::MathSymbol, Category::Symbol, "Math"),
    mapping(
        GeneralCategory::CurrencySymbol,
        Category::Symbol,
        "Currency",
    ),
    mapping(GeneralCategory::ModifierSymbol, Category::Symbol, "Other"),
    mapping(GeneralCategory::OtherSymbol, Category::Symbol, "Other"),
];

#[derive(Clone, Copy)]
struct ScriptOverride {
    general: GeneralCategory,
    script: Script,
    sub_category: &'static str,
}

const SCRIPT_OVERRIDES: &[ScriptOverride] = &[
    script_override(GeneralCategory::OtherLetter, Script::Han, "Ideograph"),
    script_override(GeneralCategory::OtherLetter, Script::Hiragana, "Kana"),
    script_override(GeneralCategory::OtherLetter, Script::Katakana, "Kana"),
    script_override(GeneralCategory::OtherLetter, Script::Hangul, "Hangul"),
];

#[derive(Clone, Copy)]
struct CodepointOverride {
    start: u32,
    end: u32,
    category: Category,
    sub_category: &'static str,
}

const CODEPOINT_OVERRIDES: &[CodepointOverride] = &[
    codepoint_override(0x0022, 0x0022, Category::Punctuation, "Quote"),
    codepoint_override(0x0027, 0x0027, Category::Punctuation, "Quote"),
    codepoint_override(0x00BC, 0x00BE, Category::Number, "Fraction"),
    codepoint_override(0x09F4, 0x09F9, Category::Number, "Fraction"),
    codepoint_override(0x0B72, 0x0B77, Category::Number, "Fraction"),
    codepoint_override(0x0D58, 0x0D5E, Category::Number, "Fraction"),
    codepoint_override(0x0F2A, 0x0F33, Category::Number, "Fraction"),
    codepoint_override(0x2150, 0x215E, Category::Number, "Fraction"),
    codepoint_override(0x2189, 0x2189, Category::Number, "Fraction"),
    codepoint_override(0x2CFD, 0x2CFD, Category::Number, "Fraction"),
    codepoint_override(0xA830, 0xA835, Category::Number, "Fraction"),
    codepoint_override(0x2190, 0x21FF, Category::Symbol, "Arrow"),
    codepoint_override(0x27F0, 0x27FF, Category::Symbol, "Arrow"),
    codepoint_override(0x2900, 0x297F, Category::Symbol, "Arrow"),
    codepoint_override(0x2B00, 0x2BFF, Category::Symbol, "Arrow"),
    codepoint_override(0x1F800, 0x1F8FF, Category::Symbol, "Arrow"),
];

/// Returns the broad Unicode category for `cp`.
pub fn category(cp: u32) -> Category {
    mapping_for(general_category(cp))
        .map(|entry| entry.category)
        .unwrap_or(Category::Other)
}

/// Returns the GlyphData-style sub-category bucket for `cp`.
pub fn sub_category(cp: u32) -> &'static str {
    let general = general_category(cp);
    let Some(mapping) = mapping_for(general) else {
        return "Other";
    };

    if let Some(rule) = CODEPOINT_OVERRIDES
        .iter()
        .find(|rule| rule.category == mapping.category && (rule.start..=rule.end).contains(&cp))
    {
        return rule.sub_category;
    }

    if let Some(rule) = SCRIPT_OVERRIDES
        .iter()
        .find(|rule| rule.general == general && rule.script == script_value(cp))
    {
        return rule.sub_category;
    }

    mapping.sub_category
}

fn general_category(cp: u32) -> GeneralCategory {
    if char::from_u32(cp).is_none() {
        return GeneralCategory::Unassigned;
    }
    CodePointMapData::<GeneralCategory>::new().get32(cp)
}

fn mapping_for(general: GeneralCategory) -> Option<&'static CategoryMapping> {
    CATEGORY_MAPPINGS
        .iter()
        .find(|entry| entry.general == general)
}

const fn mapping(
    general: GeneralCategory,
    category: Category,
    sub_category: &'static str,
) -> CategoryMapping {
    CategoryMapping {
        general,
        category,
        sub_category,
    }
}

const fn script_override(
    general: GeneralCategory,
    script: Script,
    sub_category: &'static str,
) -> ScriptOverride {
    ScriptOverride {
        general,
        script,
        sub_category,
    }
}

const fn codepoint_override(
    start: u32,
    end: u32,
    category: Category,
    sub_category: &'static str,
) -> CodepointOverride {
    CodepointOverride {
        start,
        end,
        category,
        sub_category,
    }
}
