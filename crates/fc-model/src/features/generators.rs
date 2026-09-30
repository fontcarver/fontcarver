//! Deterministic name- and model-derived feature rule recipes.
//!
//! These recipes are pure queries over the document: they read exported glyph
//! names, glyph metadata, anchors and component structure, and return feature
//! syntax. They never parse or compile; the compiler crate owns conflict
//! detection, validation and activation. Keeping them here lets the model test
//! every family against a synthetic inventory without a compiler.

use std::collections::BTreeSet;

use crate::{FeatureGenerationSettings, FeatureGenerator, Font, Glyph, GlyphName};

/// Reserved prefix for generated glyph classes and lookup names.
///
/// User source must not define these names; the feature compiler reports the
/// duplicate definition when it does.
pub const GENERATED_NAME_PREFIX: &str = "fcGen";

/// Whether a generated source item is a tagged feature or a named glyph class.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum GeneratedItemKind {
    /// A tagged feature block.
    Feature,
    /// A named glyph class definition.
    Class,
}

/// Rule lines and the glyphs one generated source item depends on.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GeneratedRules {
    /// Feature-syntax lines; callers join them with newlines.
    pub code: Vec<String>,
    /// Glyphs responsible for these rules, in deterministic order.
    pub glyphs: BTreeSet<GlyphName>,
}

/// A generation problem that must block activation of the candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenerationIssue {
    /// Stable machine-readable category.
    pub code: &'static str,
    /// Human-readable explanation.
    pub message: String,
}

/// Everything one generator family produced for one font.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GeneratedFamily {
    /// Items keyed by feature tag or class name, in canonical emission order.
    pub items: Vec<(String, GeneratedRules)>,
    /// Problems encountered while generating; any issue blocks activation.
    pub issues: Vec<GenerationIssue>,
}

impl GeneratedFamily {
    fn issue(&mut self, code: &'static str, message: impl Into<String>) {
        self.issues.push(GenerationIssue {
            code,
            message: message.into(),
        });
    }

    fn item(&mut self, name: &str) -> &mut GeneratedRules {
        if let Some(index) = self.items.iter().position(|(key, _)| key == name) {
            return &mut self.items[index].1;
        }
        self.items
            .push((name.to_owned(), GeneratedRules::default()));
        &mut self.items.last_mut().expect("just pushed").1
    }
}

/// Whether this family owns feature blocks or glyph class definitions.
pub fn family_kind(generator: FeatureGenerator) -> GeneratedItemKind {
    match generator {
        FeatureGenerator::Classes => GeneratedItemKind::Class,
        _ => GeneratedItemKind::Feature,
    }
}

/// Every source item name the family may own, for conflict and switch decisions.
///
/// `Classes` reports the explicitly requested class names, so an unrequested
/// registry name never counts as generated.
pub fn family_names(
    generator: FeatureGenerator,
    settings: &FeatureGenerationSettings,
) -> Vec<String> {
    match generator {
        FeatureGenerator::SmallCaps => vec!["smcp".into()],
        FeatureGenerator::StylisticSets => (1..=20).map(|i| format!("ss{i:02}")).collect(),
        FeatureGenerator::CaseForms => vec!["case".into()],
        FeatureGenerator::VerticalForms => vec!["vert".into(), "vrt2".into()],
        FeatureGenerator::Ligatures => vec!["liga".into()],
        FeatureGenerator::Localized => vec!["locl".into()],
        FeatureGenerator::DiscretionaryLigatures => vec!["dlig".into()],
        FeatureGenerator::CapsToSmallCaps => vec!["c2sc".into()],
        FeatureGenerator::FigureStyles => FIGURE_STYLES
            .iter()
            .map(|(tag, _)| (*tag).to_owned())
            .collect(),
        FeatureGenerator::PositionalFigures => POSITIONAL_FIGURES
            .iter()
            .map(|(tag, _)| (*tag).to_owned())
            .collect(),
        FeatureGenerator::Fractions => vec!["frac".into()],
        FeatureGenerator::Ordinals => vec!["ordn".into()],
        FeatureGenerator::Composition => vec!["ccmp".into()],
        FeatureGenerator::AccessAllAlternates => vec!["aalt".into()],
        FeatureGenerator::Classes => {
            let mut names: Vec<String> = settings.automatic_classes.clone();
            names.sort();
            names.dedup();
            names
        }
    }
}

/// Every glyph class name an automatic class entry may be switched to.
pub fn automatic_class_registry() -> &'static [&'static str] {
    &[
        "Uppercase",
        "Lowercase",
        "Letters",
        "Figures",
        "Marks",
        "Punctuation",
        "Symbols",
    ]
}

/// Feature tags `aalt` aggregates, in the order their alternates are offered.
pub fn aalt_feature_order() -> Vec<String> {
    let mut tags: Vec<String> = ["salt", "swsh", "titl", "hist", "nalt", "ornm"]
        .iter()
        .map(|tag| (*tag).to_owned())
        .collect();
    tags.extend((1..=20).map(|i| format!("ss{i:02}")));
    tags.extend(
        [
            "smcp", "c2sc", "pcap", "unic", "ital", "ruby", "onum", "lnum", "pnum", "tnum", "sups",
            "sinf", "numr", "dnom", "ordn", "subs",
        ]
        .iter()
        .map(|tag| (*tag).to_owned()),
    );
    tags
}

/// Suffixes that select one figure-style feature, most specific first.
const FIGURE_STYLES: [(&str, &[&str]); 4] = [
    ("lnum", &[".lf", ".lnum"]),
    ("onum", &[".osf", ".oldstyle", ".onum"]),
    ("tnum", &[".tf", ".tnum"]),
    ("pnum", &[".pf", ".pnum", ".prop"]),
];

/// Suffixes that select one positional-figure feature.
const POSITIONAL_FIGURES: [(&str, &[&str]); 4] = [
    ("sups", &[".sups", ".superior", ".sup"]),
    ("sinf", &[".sinf", ".inferior"]),
    ("numr", &[".numr"]),
    ("dnom", &[".dnom"]),
];

/// Code-point ranges of combining marks that sit above their base.
///
/// These are the Unicode combining-class-230 ranges a dotless Latin base can
/// meet. A mark outside them, such as a cedilla or a dot below, leaves the dot
/// of `i` and `j` in place.
const ABOVE_MARK_RANGES: [(u32, u32); 8] = [
    (0x0300, 0x0314),
    (0x031A, 0x031A),
    (0x033D, 0x0344),
    (0x0346, 0x0346),
    (0x034A, 0x034C),
    (0x0350, 0x0352),
    (0x0357, 0x0357),
    (0x0363, 0x036F),
];

/// Glyph pairs whose dotless form replaces the base before a mark in `ccmp`.
const DOTLESS_PAIRS: [(&str, &str); 4] = [
    ("i", "idotless"),
    ("j", "jdotless"),
    ("i", "dotlessi"),
    ("j", "dotlessj"),
];

/// Build every rule one family contributes, without touching the document.
///
/// `present_tags` lists feature tags already present in the candidate source
/// (user-owned plus generated earlier in the same run); only `aalt` reads it.
pub fn generate_family(
    font: &Font,
    generator: FeatureGenerator,
    settings: &FeatureGenerationSettings,
    present_tags: &BTreeSet<String>,
) -> GeneratedFamily {
    let mut result = GeneratedFamily::default();
    let names = exported_names(font);
    match generator {
        FeatureGenerator::CapsToSmallCaps => caps_to_small_caps(font, &names, &mut result),
        FeatureGenerator::FigureStyles => {
            for (tag, suffixes) in FIGURE_STYLES {
                suffix_family(&names, tag, suffixes, &mut result);
            }
        }
        FeatureGenerator::PositionalFigures => {
            for (tag, suffixes) in POSITIONAL_FIGURES {
                suffix_family(&names, tag, suffixes, &mut result);
            }
        }
        FeatureGenerator::Fractions => fractions(font, &names, &mut result),
        FeatureGenerator::Ordinals => ordinals(font, &names, &mut result),
        FeatureGenerator::Composition => composition(font, &names, &mut result),
        FeatureGenerator::AccessAllAlternates => {
            access_all_alternates(present_tags, &mut result);
        }
        FeatureGenerator::Classes => classes(font, settings, &mut result),
        // The compiler crate owns the seven families accepted in P2b.
        FeatureGenerator::SmallCaps
        | FeatureGenerator::StylisticSets
        | FeatureGenerator::CaseForms
        | FeatureGenerator::VerticalForms
        | FeatureGenerator::Ligatures
        | FeatureGenerator::Localized
        | FeatureGenerator::DiscretionaryLigatures => {}
    }
    result.items.retain(|(_, rules)| !rules.code.is_empty());
    result
}

fn exported_names(font: &Font) -> BTreeSet<GlyphName> {
    font.glyphs
        .values()
        .filter(|glyph| glyph.export)
        .map(|glyph| glyph.name.clone())
        .collect()
}

/// Escape a glyph name for feature syntax, rejecting names FEA cannot express.
///
/// A backslash before a numeric-leading name would be read as a CID, so those
/// names are refused instead of silently changing meaning.
fn glyph_token(name: &GlyphName) -> Result<String, String> {
    let mut chars = name.chars();
    if !chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || matches!(c, '_' | '.'))
        || !chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
    {
        return Err(format!(
            "Glyph name {name:?} cannot be represented safely in generated feature syntax."
        ));
    }
    Ok(format!("\\{name}"))
}

fn tokens(glyphs: &[GlyphName], result: &mut GeneratedFamily) -> Option<Vec<String>> {
    let mut list = Vec::with_capacity(glyphs.len());
    for name in glyphs {
        match glyph_token(name) {
            Ok(token) => list.push(token),
            Err(message) => {
                result.issue("feature-generation-glyph-name", message);
                return None;
            }
        }
    }
    Some(list)
}

fn substitution(
    from: &GlyphName,
    to: &GlyphName,
    result: &mut GeneratedFamily,
) -> Option<(String, [GlyphName; 2])> {
    if from == to {
        result.issue(
            "feature-generation-duplicate",
            format!("Substitution {from} → {to} has identical input and output."),
        );
        return None;
    }
    let pair = tokens(std::slice::from_ref(from), result)?;
    let target = tokens(std::slice::from_ref(to), result)?;
    Some((
        format!("sub {} by {};", pair[0], target[0]),
        [from.clone(), to.clone()],
    ))
}

/// Emit `sub base by base<suffix>` for every exported suffixed glyph.
fn suffix_family(
    names: &BTreeSet<GlyphName>,
    tag: &str,
    suffixes: &[&str],
    result: &mut GeneratedFamily,
) {
    for target in names {
        let Some(base) = suffixes
            .iter()
            .find_map(|suffix| target.strip_suffix(suffix))
            .map(GlyphName::from)
        else {
            continue;
        };
        if !names.contains(&base) {
            result.issue(
                "feature-generation-missing-base",
                format!("Both glyphs must exist and export for substitution {base} → {target}."),
            );
            continue;
        }
        if let Some((code, glyphs)) = substitution(&base, target, result) {
            let rules = result.item(tag);
            rules.code.push(code);
            rules.glyphs.extend(glyphs);
        }
    }
}

fn glyph_of<'a>(font: &'a Font, name: &GlyphName) -> Option<&'a Glyph> {
    font.glyph(name.as_ref()).filter(|glyph| glyph.export)
}

/// Whether the glyph is a combining mark for generated layout purposes.
///
/// Either the document's own classification says so, or the default master
/// layer carries a mark-attachment anchor, which is exactly the signal the
/// automatic mark writer uses.
fn is_mark(font: &Font, glyph: &Glyph) -> bool {
    if glyph.category.as_deref() == Some("Mark") {
        return true;
    }
    glyph
        .master_layer(&font.default_master().id)
        .is_some_and(|layer| {
            layer
                .anchors
                .iter()
                .any(|anchor| anchor.name.starts_with('_'))
        })
}

/// Whether the mark attaches above its base.
///
/// An explicit mark-attachment anchor decides it when the document has one;
/// otherwise the mark's own code point does. A mark with a below anchor and no
/// top anchor is never treated as an above mark.
fn is_above_mark(font: &Font, glyph: &Glyph) -> bool {
    let anchors: Vec<&str> = glyph
        .master_layer(&font.default_master().id)
        .map(|layer| {
            layer
                .anchors
                .iter()
                .map(|anchor| anchor.name.as_str())
                .filter(|name| name.starts_with('_'))
                .collect()
        })
        .unwrap_or_default();
    if !anchors.is_empty() {
        return anchors.iter().any(|name| name.starts_with("_top"));
    }
    glyph.unicodes.iter().any(|code| {
        ABOVE_MARK_RANGES
            .iter()
            .any(|(first, last)| (first..=last).contains(&code))
    })
}

/// Whether the glyph represents a decimal digit.
fn is_figure(glyph: &Glyph) -> bool {
    glyph.category.as_deref() == Some("Number")
        || glyph
            .unicodes
            .iter()
            .filter_map(|code| char::from_u32(*code))
            .any(|c| c.is_ascii_digit())
}

/// The exported glyph encoding the uppercase form of this glyph's code point.
fn uppercase_of(font: &Font, glyph: &Glyph) -> Option<GlyphName> {
    let lower = glyph
        .unicodes
        .iter()
        .filter_map(|code| char::from_u32(*code))
        .find(|c| c.is_lowercase())?;
    let mut upper = lower.to_uppercase();
    let (first, rest) = (upper.next()?, upper.next());
    if rest.is_some() {
        return None;
    }
    let code = u32::from(first);
    font.glyph_order()
        .filter_map(|name| glyph_of(font, name))
        .find(|candidate| candidate.unicodes.contains(&code))
        .map(|candidate| candidate.name.clone())
}

/// Map uppercase letters onto the small-capital glyphs shared with `smcp`.
///
/// `X.c2sc` is used directly when present. Otherwise a lowercase-based small
/// capital such as `a.sc` also serves the uppercase letter, which is the
/// ordinary naming convention; uppercase-based names such as `A.sc` are already
/// covered by the `smcp` recipe and are skipped here.
fn caps_to_small_caps(font: &Font, names: &BTreeSet<GlyphName>, result: &mut GeneratedFamily) {
    suffix_family(names, "c2sc", &[".c2sc"], result);
    for target in names {
        let Some(base) = [".sc", ".smcp", ".small"]
            .iter()
            .find_map(|suffix| target.strip_suffix(suffix))
            .map(GlyphName::from)
        else {
            continue;
        };
        let Some(glyph) = glyph_of(font, &base) else {
            continue;
        };
        let Some(upper) = uppercase_of(font, glyph) else {
            continue;
        };
        if let Some((code, glyphs)) = substitution(&upper, target, result) {
            let rules = result.item("c2sc");
            rules.code.push(code);
            rules.glyphs.extend(glyphs);
        }
    }
}

/// Exported digits in font order, plus the figure glyphs a suffix selects.
fn figure_run(font: &Font, suffix: &str) -> (Vec<GlyphName>, Vec<GlyphName>) {
    let mut bases = Vec::new();
    let mut derived = Vec::new();
    for name in font.glyph_order() {
        let Some(glyph) = glyph_of(font, name) else {
            continue;
        };
        if !is_figure(glyph) {
            continue;
        }
        let variant = GlyphName::from(format!("{name}{suffix}"));
        if glyph_of(font, &variant).is_some() {
            bases.push(name.clone());
            derived.push(variant);
        }
    }
    (bases, derived)
}

/// Build `frac` from numerator and denominator figures around a fraction slash.
///
/// Numerators use a reverse-chaining lookup so any number of leading digits is
/// converted in one pass; denominators chain forward from the fraction slash.
fn fractions(font: &Font, names: &BTreeSet<GlyphName>, result: &mut GeneratedFamily) {
    let (numerator_bases, numerators) = figure_run(font, ".numr");
    let (denominator_bases, denominators) = figure_run(font, ".dnom");
    if numerator_bases.is_empty() || denominator_bases.is_empty() {
        result.issue(
            "feature-generation-missing-base",
            "Generating frac needs exported .numr and .dnom figures.",
        );
        return;
    }
    let fraction = GlyphName::from("fraction");
    if !names.contains(&fraction) {
        result.issue(
            "feature-generation-missing-base",
            "Generating frac needs an exported fraction glyph.",
        );
        return;
    }
    let Some(numerator_base_tokens) = tokens(&numerator_bases, result) else {
        return;
    };
    let Some(numerator_tokens) = tokens(&numerators, result) else {
        return;
    };
    let Some(denominator_base_tokens) = tokens(&denominator_bases, result) else {
        return;
    };
    let Some(denominator_tokens) = tokens(&denominators, result) else {
        return;
    };
    let mut code = vec![
        format!(
            "@{GENERATED_NAME_PREFIX}FracNumeratorInput = [{}];",
            numerator_base_tokens.join(" ")
        ),
        format!(
            "@{GENERATED_NAME_PREFIX}FracNumerator = [{}];",
            numerator_tokens.join(" ")
        ),
        format!(
            "@{GENERATED_NAME_PREFIX}FracDenominatorInput = [{}];",
            denominator_base_tokens.join(" ")
        ),
        format!(
            "@{GENERATED_NAME_PREFIX}FracDenominator = [{}];",
            denominator_tokens.join(" ")
        ),
    ];
    // A slash between digits becomes the fraction slash before the digits are
    // rewritten, so the numerator and denominator contexts match one glyph.
    let slash = GlyphName::from("slash");
    if names.contains(&slash) {
        code.push(format!(
            "lookup {GENERATED_NAME_PREFIX}FracSlash {{\nsub @{GENERATED_NAME_PREFIX}FracNumeratorInput \\slash' @{GENERATED_NAME_PREFIX}FracDenominatorInput by \\fraction;\n}} {GENERATED_NAME_PREFIX}FracSlash;"
        ));
    }
    code.push(format!(
        "lookup {GENERATED_NAME_PREFIX}FracNumerators {{\nrsub @{GENERATED_NAME_PREFIX}FracNumeratorInput' \\fraction by @{GENERATED_NAME_PREFIX}FracNumerator;\nrsub @{GENERATED_NAME_PREFIX}FracNumeratorInput' @{GENERATED_NAME_PREFIX}FracNumerator by @{GENERATED_NAME_PREFIX}FracNumerator;\n}} {GENERATED_NAME_PREFIX}FracNumerators;"
    ));
    code.push(format!(
        "lookup {GENERATED_NAME_PREFIX}FracDenominators {{\nsub \\fraction @{GENERATED_NAME_PREFIX}FracDenominatorInput' by @{GENERATED_NAME_PREFIX}FracDenominator;\nsub @{GENERATED_NAME_PREFIX}FracDenominator @{GENERATED_NAME_PREFIX}FracDenominatorInput' by @{GENERATED_NAME_PREFIX}FracDenominator;\n}} {GENERATED_NAME_PREFIX}FracDenominators;"
    ));
    let rules = result.item("frac");
    rules.code = code;
    rules.glyphs.extend(numerator_bases);
    rules.glyphs.extend(numerators);
    rules.glyphs.extend(denominator_bases);
    rules.glyphs.extend(denominators);
    rules.glyphs.insert(fraction);
    if names.contains(&slash) {
        rules.glyphs.insert(slash);
    }
}

/// Build `ordn` from ordinal glyphs following a figure.
fn ordinals(font: &Font, names: &BTreeSet<GlyphName>, result: &mut GeneratedFamily) {
    let figures: Vec<GlyphName> = font
        .glyph_order()
        .filter(|name| glyph_of(font, name).is_some_and(is_figure))
        .cloned()
        .collect();
    if figures.is_empty() {
        result.issue(
            "feature-generation-missing-base",
            "Generating ordn needs exported figures.",
        );
        return;
    }
    let Some(figure_tokens) = tokens(&figures, result) else {
        return;
    };
    let mut rules = Vec::new();
    let mut used: BTreeSet<GlyphName> = BTreeSet::new();
    for (letters, ordinal) in [(["a", "A"], "ordfeminine"), (["o", "O"], "ordmasculine")] {
        let ordinal = GlyphName::from(ordinal);
        if !names.contains(&ordinal) {
            continue;
        }
        let present: Vec<GlyphName> = letters
            .iter()
            .map(|letter| GlyphName::from(*letter))
            .filter(|letter| names.contains(letter))
            .collect();
        if present.is_empty() {
            continue;
        }
        let Some(letter_tokens) = tokens(&present, result) else {
            return;
        };
        let Some(ordinal_token) = tokens(std::slice::from_ref(&ordinal), result) else {
            return;
        };
        rules.push(format!(
            "sub [{}] [{}]' by {};",
            figure_tokens.join(" "),
            letter_tokens.join(" "),
            ordinal_token[0]
        ));
        used.extend(present);
        used.insert(ordinal);
    }
    for target in names {
        let Some(base) = target.strip_suffix(".ordn").map(GlyphName::from) else {
            continue;
        };
        if !names.contains(&base) {
            result.issue(
                "feature-generation-missing-base",
                format!("Both glyphs must exist and export for substitution {base} → {target}."),
            );
            continue;
        }
        let Some(pair) = tokens(&[base.clone(), target.clone()], result) else {
            return;
        };
        rules.push(format!(
            "sub [{}] {}' by {};",
            figure_tokens.join(" "),
            pair[0],
            pair[1]
        ));
        used.extend([base, target.clone()]);
    }
    if rules.is_empty() {
        result.issue(
            "feature-generation-missing-base",
            "Generating ordn needs ordfeminine/ordmasculine or .ordn glyphs.",
        );
        return;
    }
    let item = result.item("ordn");
    item.code = rules;
    item.glyphs.extend(figures);
    item.glyphs.extend(used);
}

/// Build `ccmp` from real component structure and dotless base forms.
fn composition(font: &Font, names: &BTreeSet<GlyphName>, result: &mut GeneratedFamily) {
    let master = font.default_master().id.clone();
    // Only a mark that lands above the base may remove the dot of `i` or `j`.
    let marks: Vec<GlyphName> = font
        .glyph_order()
        .filter(|name| {
            glyph_of(font, name)
                .is_some_and(|glyph| is_mark(font, glyph) && is_above_mark(font, glyph))
        })
        .cloned()
        .collect();
    let mut rules = Vec::new();
    let mut used: BTreeSet<GlyphName> = BTreeSet::new();
    for name in font.glyph_order() {
        let Some(glyph) = glyph_of(font, name) else {
            continue;
        };
        let Some(layer) = glyph.master_layer(&master) else {
            continue;
        };
        if layer.components.len() < 2 || !layer.contours.is_empty() {
            continue;
        }
        // Only a plain translation keeps the composed sequence equivalent to
        // the precomposed glyph; scaled or mirrored references are not composed.
        if layer.components.iter().any(|component| {
            let [a, b, c, d, _, _] = component.transform.as_coeffs();
            a != 1.0 || b != 0.0 || c != 0.0 || d != 1.0
        }) {
            continue;
        }
        let bases: Vec<GlyphName> = layer
            .components
            .iter()
            .map(|component| component.base.clone())
            .collect();
        if bases.iter().any(|base| !names.contains(base)) {
            continue;
        }
        let first = bases.first().and_then(|base| glyph_of(font, base));
        let composable = first.is_some_and(|glyph| !is_mark(font, glyph))
            && bases[1..]
                .iter()
                .all(|base| glyph_of(font, base).is_some_and(|glyph| is_mark(font, glyph)));
        if !composable {
            continue;
        }
        let Some(sequence) = tokens(&bases, result) else {
            return;
        };
        let Some(target) = tokens(std::slice::from_ref(name), result) else {
            return;
        };
        rules.push(format!("sub {} by {};", sequence.join(" "), target[0]));
        used.extend(bases);
        used.insert(name.clone());
    }
    if !marks.is_empty() {
        let Some(mark_tokens) = tokens(&marks, result) else {
            return;
        };
        for (base, dotless) in DOTLESS_PAIRS {
            let base = GlyphName::from(base);
            let dotless = GlyphName::from(dotless);
            if !names.contains(&base) || !names.contains(&dotless) {
                continue;
            }
            let Some(pair) = tokens(&[base.clone(), dotless.clone()], result) else {
                return;
            };
            rules.push(format!(
                "sub {}' [{}] by {};",
                pair[0],
                mark_tokens.join(" "),
                pair[1]
            ));
            used.extend([base, dotless]);
            used.extend(marks.iter().cloned());
        }
    }
    if rules.is_empty() {
        result.issue(
            "feature-generation-missing-base",
            "Generating ccmp needs composed base-and-mark glyphs or a dotless form.",
        );
        return;
    }
    let item = result.item("ccmp");
    item.code = rules;
    item.glyphs.extend(used);
}

/// Aggregate alternates from the features actually present into `aalt`.
fn access_all_alternates(present_tags: &BTreeSet<String>, result: &mut GeneratedFamily) {
    let referenced: Vec<String> = aalt_feature_order()
        .into_iter()
        .filter(|tag| present_tags.contains(tag))
        .collect();
    if referenced.is_empty() {
        result.issue(
            "feature-generation-missing-base",
            "Generating aalt needs at least one alternate-producing feature.",
        );
        return;
    }
    let item = result.item("aalt");
    item.code = referenced
        .into_iter()
        .map(|tag| format!("feature {tag};"))
        .collect();
}

/// Materialize the explicitly requested metadata-derived glyph classes.
fn classes(font: &Font, settings: &FeatureGenerationSettings, result: &mut GeneratedFamily) {
    let mut requested: Vec<String> = settings.automatic_classes.clone();
    requested.sort();
    requested.dedup();
    for name in requested {
        if !automatic_class_registry().contains(&name.as_str()) {
            result.issue(
                "feature-generation-unknown-class",
                format!("No automatic recipe defines the glyph class {name}."),
            );
            continue;
        }
        let members: Vec<GlyphName> = font
            .glyph_order()
            .filter(|glyph_name| {
                glyph_of(font, glyph_name).is_some_and(|glyph| class_matches(font, &name, glyph))
            })
            .cloned()
            .collect();
        if members.is_empty() {
            result.issue(
                "feature-generation-empty-class",
                format!("No exported glyph belongs to the automatic class {name}."),
            );
            continue;
        }
        let Some(member_tokens) = tokens(&members, result) else {
            return;
        };
        let item = result.item(&name);
        item.code = vec![member_tokens.join(" ")];
        item.glyphs.extend(members);
    }
}

fn class_matches(font: &Font, class: &str, glyph: &Glyph) -> bool {
    let chars: Vec<char> = glyph
        .unicodes
        .iter()
        .filter_map(|code| char::from_u32(*code))
        .collect();
    let category = glyph.category.as_deref();
    let sub_category = glyph.sub_category.as_deref();
    match class {
        "Uppercase" => {
            sub_category == Some("Uppercase")
                || (category != Some("Mark") && chars.iter().any(|c| c.is_uppercase()))
        }
        "Lowercase" => {
            sub_category == Some("Lowercase")
                || (category != Some("Mark") && chars.iter().any(|c| c.is_lowercase()))
        }
        "Letters" => {
            category == Some("Letter")
                || (category.is_none() && chars.iter().any(|c| c.is_alphabetic()))
        }
        "Figures" => is_figure(glyph),
        "Marks" => is_mark(font, glyph),
        "Punctuation" => {
            category == Some("Punctuation")
                || (category.is_none() && chars.iter().any(|c| c.is_ascii_punctuation()))
        }
        "Symbols" => category == Some("Symbol"),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Anchor, Component, Glyph, Layer};

    fn font_with(entries: &[(&str, Option<u32>)]) -> Font {
        let mut font = Font::new_empty("Generators", 1000);
        let master = font.default_master().id.clone();
        for (name, unicode) in entries {
            let mut glyph = Glyph::new(*name);
            glyph.unicodes.extend(*unicode);
            let mut layer = Layer::new_master(master.clone());
            layer.width = 500.0;
            glyph.layers.push(layer);
            font.insert_glyph(glyph).unwrap();
        }
        font
    }

    fn family(font: &Font, generator: FeatureGenerator) -> GeneratedFamily {
        generate_family(
            font,
            generator,
            &FeatureGenerationSettings::default(),
            &BTreeSet::new(),
        )
    }

    fn code(result: &GeneratedFamily, name: &str) -> String {
        result
            .items
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, rules)| rules.code.join("\n"))
            .unwrap_or_default()
    }

    #[test]
    fn caps_to_small_caps_uses_lowercase_based_small_capitals() {
        let font = font_with(&[
            ("a", Some(0x61)),
            ("A", Some(0x41)),
            ("a.sc", None),
            ("B", Some(0x42)),
            ("B.c2sc", None),
        ]);
        let result = family(&font, FeatureGenerator::CapsToSmallCaps);
        assert_eq!(result.issues, Vec::new());
        let generated = code(&result, "c2sc");
        assert!(generated.contains("sub \\B by \\B.c2sc;"), "{generated}");
        assert!(generated.contains("sub \\A by \\a.sc;"), "{generated}");
        assert!(!generated.contains("sub \\a by"), "{generated}");
    }

    #[test]
    fn figure_and_positional_families_cover_every_requested_tag() {
        let font = font_with(&[
            ("one", Some(0x31)),
            ("one.lf", None),
            ("one.osf", None),
            ("one.tf", None),
            ("one.pnum", None),
            ("one.sups", None),
            ("one.sinf", None),
            ("one.numr", None),
            ("one.dnom", None),
        ]);
        let figures = family(&font, FeatureGenerator::FigureStyles);
        assert_eq!(figures.issues, Vec::new());
        for (tag, expected) in [
            ("lnum", "sub \\one by \\one.lf;"),
            ("onum", "sub \\one by \\one.osf;"),
            ("tnum", "sub \\one by \\one.tf;"),
            ("pnum", "sub \\one by \\one.pnum;"),
        ] {
            assert_eq!(code(&figures, tag), expected, "{tag}");
        }
        let positional = family(&font, FeatureGenerator::PositionalFigures);
        assert_eq!(positional.issues, Vec::new());
        for (tag, expected) in [
            ("sups", "sub \\one by \\one.sups;"),
            ("sinf", "sub \\one by \\one.sinf;"),
            ("numr", "sub \\one by \\one.numr;"),
            ("dnom", "sub \\one by \\one.dnom;"),
        ] {
            assert_eq!(code(&positional, tag), expected, "{tag}");
        }
    }

    #[test]
    fn fractions_need_numerators_denominators_and_a_fraction_slash() {
        let mut font = font_with(&[("one", Some(0x31)), ("one.numr", None)]);
        let missing = family(&font, FeatureGenerator::Fractions);
        assert_eq!(missing.items, Vec::new());
        assert_eq!(missing.issues.len(), 1);
        for name in ["one.dnom", "fraction", "slash"] {
            let mut glyph = Glyph::new(name);
            if name == "slash" {
                glyph.unicodes.push(0x2f);
            }
            glyph
                .layers
                .push(Layer::new_master(font.default_master().id.clone()));
            font.insert_glyph(glyph).unwrap();
        }
        let result = family(&font, FeatureGenerator::Fractions);
        assert_eq!(result.issues, Vec::new());
        let generated = code(&result, "frac");
        assert!(
            generated.contains("@fcGenFracNumerator = [\\one.numr];"),
            "{generated}"
        );
        assert!(
            generated.contains("rsub @fcGenFracNumeratorInput' \\fraction by @fcGenFracNumerator;"),
            "{generated}"
        );
        assert!(
            generated
                .contains("sub \\fraction @fcGenFracDenominatorInput' by @fcGenFracDenominator;"),
            "{generated}"
        );
        assert!(
            generated.contains("\\slash' @fcGenFracDenominatorInput by \\fraction;"),
            "{generated}"
        );
    }

    #[test]
    fn ordinals_combine_classic_rules_with_suffixed_glyphs() {
        let font = font_with(&[
            ("one", Some(0x31)),
            ("a", Some(0x61)),
            ("o", Some(0x6f)),
            ("ordfeminine", Some(0xaa)),
            ("ordmasculine", Some(0xba)),
            ("period", Some(0x2e)),
            ("period.ordn", None),
        ]);
        let result = family(&font, FeatureGenerator::Ordinals);
        assert_eq!(result.issues, Vec::new());
        let generated = code(&result, "ordn");
        assert!(
            generated.contains("sub [\\one] [\\a]' by \\ordfeminine;"),
            "{generated}"
        );
        assert!(
            generated.contains("sub [\\one] [\\o]' by \\ordmasculine;"),
            "{generated}"
        );
        assert!(
            generated.contains("sub [\\one] \\period' by \\period.ordn;"),
            "{generated}"
        );
    }

    #[test]
    fn composition_reads_components_and_dotless_pairs() {
        let mut font = font_with(&[("a", Some(0x61)), ("i", Some(0x69)), ("idotless", None)]);
        let master = font.default_master().id.clone();
        let mut mark = Glyph::new("acutecomb");
        mark.unicodes.push(0x301);
        mark.category = Some("Mark".into());
        let mut mark_layer = Layer::new_master(master.clone());
        mark_layer.anchors.push(Anchor {
            name: "_top".into(),
            x: 0.0,
            y: 0.0,
        });
        mark.layers.push(mark_layer);
        font.insert_glyph(mark).unwrap();
        let mut composed = Glyph::new("aacute");
        composed.unicodes.push(0xe1);
        let mut layer = Layer::new_master(master);
        for base in ["a", "acutecomb"] {
            layer.components.push(Component {
                base: base.into(),
                ..Component::default()
            });
        }
        composed.layers.push(layer);
        font.insert_glyph(composed).unwrap();
        let result = family(&font, FeatureGenerator::Composition);
        assert_eq!(result.issues, Vec::new());
        let generated = code(&result, "ccmp");
        assert!(
            generated.contains("sub \\a \\acutecomb by \\aacute;"),
            "{generated}"
        );
        assert!(
            generated.contains("sub \\i' [\\acutecomb] by \\idotless;"),
            "{generated}"
        );
    }

    #[test]
    fn the_dotless_rule_only_fires_before_a_mark_that_sits_above() {
        let mut font = font_with(&[("i", Some(0x69)), ("idotless", None)]);
        let master = font.default_master().id.clone();
        // A below mark classified only by its code point, two above marks, and
        // a below mark that says so with an explicit anchor.
        for (name, unicode, anchor) in [
            ("dotbelowcomb", 0x0323_u32, None),
            ("acutecomb", 0x0301, None),
            ("cedillacomb", 0x0327, Some("_bottom")),
            ("hookabovecomb", 0x0309, Some("_top")),
        ] {
            let mut mark = Glyph::new(name);
            mark.unicodes.push(unicode);
            mark.category = Some("Mark".into());
            let mut layer = Layer::new_master(master.clone());
            if let Some(anchor) = anchor {
                layer.anchors.push(Anchor {
                    name: anchor.into(),
                    x: 0.0,
                    y: 0.0,
                });
            }
            mark.layers.push(layer);
            font.insert_glyph(mark).unwrap();
        }
        let generated = code(&family(&font, FeatureGenerator::Composition), "ccmp");
        assert!(generated.contains("\\acutecomb"), "{generated}");
        assert!(generated.contains("\\hookabovecomb"), "{generated}");
        assert!(!generated.contains("dotbelowcomb"), "{generated}");
        assert!(!generated.contains("cedillacomb"), "{generated}");
    }

    #[test]
    fn aalt_references_only_present_alternate_features_in_canonical_order() {
        let font = font_with(&[("a", Some(0x61))]);
        let present: BTreeSet<String> = ["ss02", "salt", "smcp", "kern"]
            .iter()
            .map(|tag| (*tag).to_owned())
            .collect();
        let result = generate_family(
            &font,
            FeatureGenerator::AccessAllAlternates,
            &FeatureGenerationSettings::default(),
            &present,
        );
        assert_eq!(result.issues, Vec::new());
        assert_eq!(
            code(&result, "aalt"),
            "feature salt;\nfeature ss02;\nfeature smcp;"
        );
        let empty = generate_family(
            &font,
            FeatureGenerator::AccessAllAlternates,
            &FeatureGenerationSettings::default(),
            &BTreeSet::new(),
        );
        assert_eq!(empty.items, Vec::new());
        assert_eq!(empty.issues.len(), 1);
    }

    #[test]
    fn automatic_classes_are_opt_in_and_report_unknown_or_empty_recipes() {
        let mut font = font_with(&[("A", Some(0x41)), ("a", Some(0x61)), ("one", Some(0x31))]);
        font.glyph_mut("A").unwrap().sub_category = Some("Uppercase".into());
        let settings = FeatureGenerationSettings {
            automatic_classes: vec!["Uppercase".into(), "Figures".into(), "Nonsense".into()],
            ..FeatureGenerationSettings::default()
        };
        let result = generate_family(
            &font,
            FeatureGenerator::Classes,
            &settings,
            &BTreeSet::new(),
        );
        assert_eq!(code(&result, "Uppercase"), "\\A");
        assert_eq!(code(&result, "Figures"), "\\one");
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.issues[0].code, "feature-generation-unknown-class");
        assert_eq!(
            family_names(FeatureGenerator::Classes, &settings),
            vec![
                "Figures".to_string(),
                "Nonsense".to_string(),
                "Uppercase".to_string()
            ]
        );
    }

    #[test]
    fn unsafe_glyph_names_are_refused_instead_of_emitted() {
        let font = font_with(&[("one", Some(0x31)), ("one.lf", None), ("1bad", Some(0x32))]);
        let mut settings = FeatureGenerationSettings::default();
        settings.automatic_classes.push("Figures".into());
        let result = generate_family(
            &font,
            FeatureGenerator::Classes,
            &settings,
            &BTreeSet::new(),
        );
        assert!(result.items.is_empty());
        assert_eq!(result.issues[0].code, "feature-generation-glyph-name");
    }
}
