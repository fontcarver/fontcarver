//! Versioned glyph metadata and validated project-local overrides.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use crate::{category, category_name, name_for_unicode, script, sub_category, unicode_for_name};

#[path = "database_validation.rs"]
mod validation;

/// The original FontCarver curated metadata, serialized as a version 1 overlay.
pub const BUNDLED_DATABASE_JSON: &str = include_str!("../data/glyphdata.json");

static BUNDLED: LazyLock<GlyphDataOverlay> = LazyLock::new(|| {
    // Tests validate the committed file; avoid panics if packaging changes it.
    parse_overlay(BUNDLED_DATABASE_JSON).unwrap_or_default()
});

/// Metadata resolved from bundled data, Unicode properties, and local overrides.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct GlyphInfo {
    /// Requested glyph name, including any suffix.
    pub name: String,
    /// Encoded Unicode scalar values; suffix variants normally remain unencoded.
    #[serde(default)]
    pub unicodes: Vec<u32>,
    /// Broad category, using the established [`Category`] spelling.
    #[serde(default)]
    pub category: Option<String>,
    /// More specific category, such as Uppercase, Nonspacing, or Ideograph.
    #[serde(default)]
    pub sub_category: Option<String>,
    /// ISO 15924 script code, such as Latn or Hani.
    #[serde(default)]
    pub script: Option<String>,
    /// Recommended production glyph name.
    #[serde(default)]
    pub production_name: Option<String>,
    /// Suggested anchor names; these carry no positions.
    #[serde(default)]
    pub anchors: Vec<String>,
    /// Ordered component glyph names for building a composite.
    #[serde(default)]
    pub composition: Vec<String>,
}

/// A partial glyph record. Missing or null fields inherit existing values.
///
/// An explicit empty vector clears that field. Scalar string values must be
/// nonempty; clearing inherited scalar fields is not supported in version 1.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct GlyphInfoOverride {
    /// Replacement Unicode scalar values.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unicodes: Option<Vec<u32>>,
    /// Replacement broad category.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Replacement sub-category.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_category: Option<String>,
    /// Replacement ISO 15924 script code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    /// Replacement production glyph name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub production_name: Option<String>,
    /// Replacement suggested anchor names.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchors: Option<Vec<String>>,
    /// Replacement ordered composition glyph names.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub composition: Option<Vec<String>>,
}

/// A versioned project-local glyph metadata overlay.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct GlyphDataOverlay {
    /// JSON schema version. The current supported value is 1.
    pub version: u32,
    /// Partial metadata records keyed by exact glyph name.
    #[serde(deserialize_with = "validation::deserialize_glyphs")]
    pub glyphs: BTreeMap<String, GlyphInfoOverride>,
}

impl Default for GlyphDataOverlay {
    fn default() -> Self {
        Self {
            version: 1,
            glyphs: BTreeMap::new(),
        }
    }
}

/// Parses and validates a version 1 overlay of at most 4 MiB.
///
/// Unknown JSON fields are accepted for forward compatibility. Duplicate glyph
/// keys, unsupported versions, invalid names/scalars, and excessive lists fail.
pub fn parse_overlay(json: &str) -> Result<GlyphDataOverlay, String> {
    if json.len() > validation::MAX_DOCUMENT_BYTES {
        return Err("glyph data document exceeds 4 MiB".to_owned());
    }
    let overlay: GlyphDataOverlay =
        serde_json::from_str(json).map_err(|error| format!("invalid glyph data JSON: {error}"))?;
    validate_overlay(&overlay)?;
    Ok(overlay)
}

/// Validates an overlay deserialized from a project or constructed in memory.
///
/// Limits are 65,536 records, 255-byte names, 64 encodings/components per record,
/// 128 anchors, and the same 4 MiB compact JSON budget as [`parse_overlay`].
pub fn validate_overlay(overlay: &GlyphDataOverlay) -> Result<(), String> {
    validation::validate(overlay)
}

/// Resolves glyph metadata with partial local overrides taking precedence.
///
/// Known names determine Unicode before the supplied fallback encodings. A
/// suffix inherits base metadata and appends itself to the production name,
/// but receives no Unicode unless its exact local record supplies it. The
/// leading dot in names such as `.notdef` is not a suffix. Invalid fallback
/// scalar values are ignored and duplicates are removed in original order.
/// Call [`validate_overlay`] before passing externally constructed overlays.
pub fn resolve_info(name: &str, unicodes: &[u32], overlay: &GlyphDataOverlay) -> GlyphInfo {
    let (base_name, suffix) = split_suffix(name);
    let bundled = BUNDLED.glyphs.get(base_name);
    let local = overlay.glyphs.get(base_name);
    let encodings = local
        .and_then(|entry| entry.unicodes.as_deref())
        .or_else(|| bundled.and_then(|entry| entry.unicodes.as_deref()))
        .map(sanitize_unicodes)
        .unwrap_or_else(|| {
            unicode_for_name(base_name)
                .map(|scalar| vec![scalar])
                .unwrap_or_else(|| sanitize_unicodes(unicodes))
        });
    let mut info = unicode_info(base_name, encodings);
    if let Some(record) = bundled {
        apply(&mut info, record);
    }
    if let Some(record) = local {
        apply(&mut info, record);
    }

    if let Some(suffix) = suffix {
        info.name = name.to_owned();
        info.unicodes.clear();
        if let Some(production) = &mut info.production_name {
            production.push_str(suffix);
        }
        if let Some(record) = BUNDLED.glyphs.get(name) {
            apply(&mut info, record);
        }
        if let Some(record) = overlay.glyphs.get(name) {
            apply(&mut info, record);
        }
    }
    info
}

/// Returns the encodings the database states for this exact glyph name.
///
/// A name inherits metadata from its base — `A.alt` follows `A` — but never its
/// encodings, and a name the database does not list gets none at all. `None`
/// therefore means "the database has no opinion about what this glyph encodes",
/// which is different from "it encodes nothing": a caller refreshing a glyph
/// from the database must leave its own encodings alone rather than derive one
/// from its name. The overlay wins over the bundled record for the same name.
pub fn explicit_unicodes(name: &str, overlay: &GlyphDataOverlay) -> Option<Vec<u32>> {
    overlay
        .glyphs
        .get(name)
        .and_then(|record| record.unicodes.as_deref())
        .or_else(|| {
            BUNDLED
                .glyphs
                .get(name)
                .and_then(|record| record.unicodes.as_deref())
        })
        .map(sanitize_unicodes)
}

/// Resolves a Unicode scalar to a suggested glyph record.
///
/// Exact local records explicitly encoding the scalar take precedence, followed
/// by effective bundled records, in lexical name order. Suffix variants qualify
/// only when explicitly encoded. If no record encodes the scalar, the AGLFN or
/// production name and Unicode properties provide a generated suggestion.
/// Returns `None` for an invalid scalar or when the overlay clears or redirects
/// the canonical fallback name's encoding and no other candidate remains.
pub fn resolve_unicode(cp: u32, overlay: &GlyphDataOverlay) -> Option<GlyphInfo> {
    char::from_u32(cp)?;
    for (name, record) in &overlay.glyphs {
        if record
            .unicodes
            .as_ref()
            .is_some_and(|values| values.contains(&cp))
        {
            return Some(resolve_info(name, &[], overlay));
        }
    }
    for (name, record) in &BUNDLED.glyphs {
        if record
            .unicodes
            .as_ref()
            .is_some_and(|values| values.contains(&cp))
        {
            let info = resolve_info(name, &[], overlay);
            if info.unicodes.contains(&cp) {
                return Some(info);
            }
        }
    }
    let name = name_for_unicode(cp);
    let info = resolve_info(&name, &[cp], overlay);
    info.unicodes.contains(&cp).then_some(info)
}

fn split_suffix(name: &str) -> (&str, Option<&str>) {
    let search_from = usize::from(name.starts_with('.'));
    match name.get(search_from..).and_then(|rest| rest.find('.')) {
        Some(index) => {
            let index = index + search_from;
            (&name[..index], Some(&name[index..]))
        }
        None => (name, None),
    }
}

fn sanitize_unicodes(unicodes: &[u32]) -> Vec<u32> {
    let mut seen = BTreeSet::new();
    unicodes
        .iter()
        .copied()
        .filter(|&scalar| char::from_u32(scalar).is_some() && seen.insert(scalar))
        .collect()
}

fn unicode_info(name: &str, unicodes: Vec<u32>) -> GlyphInfo {
    let scalar = unicodes.first().copied();
    GlyphInfo {
        name: name.to_owned(),
        category: scalar.map(|value| category_name(category(value)).to_owned()),
        sub_category: scalar.map(|value| sub_category(value).to_owned()),
        script: scalar.map(|value| script(value).to_owned()),
        production_name: Some(
            scalar
                .map(name_for_unicode)
                .unwrap_or_else(|| name.to_owned()),
        ),
        unicodes,
        ..GlyphInfo::default()
    }
}

fn apply(info: &mut GlyphInfo, record: &GlyphInfoOverride) {
    if let Some(value) = &record.unicodes {
        info.unicodes = sanitize_unicodes(value);
    }
    if let Some(value) = &record.category {
        info.category = Some(value.clone());
    }
    if let Some(value) = &record.sub_category {
        info.sub_category = Some(value.clone());
    }
    if let Some(value) = &record.script {
        info.script = Some(value.clone());
    }
    if let Some(value) = &record.production_name {
        info.production_name = Some(value.clone());
    }
    if let Some(value) = &record.anchors {
        info.anchors.clone_from(value);
    }
    if let Some(value) = &record.composition {
        info.composition.clone_from(value);
    }
}
