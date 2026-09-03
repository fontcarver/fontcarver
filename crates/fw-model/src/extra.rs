//! Opaque imported data preserved for lossless round trips.

use std::{collections::BTreeMap, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{GlyphName, Tag};

/// Provenance of the last imported source font.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SourceProvenance {
    /// Original path, when one was available.
    #[serde(default)]
    pub path: Option<String>,
    /// Source format name.
    #[serde(default)]
    pub format: String,
    /// Hexadecimal SHA-256 digest of the source.
    #[serde(default)]
    pub sha256: String,
    /// Tags present in the source font.
    #[serde(default)]
    pub table_tags: Vec<Tag>,
    /// Glyph names in source glyph-ID order.
    #[serde(default)]
    pub glyph_order: Vec<GlyphName>,
    /// In-memory source bytes for preservation merges; never persisted.
    #[serde(skip)]
    #[cfg_attr(feature = "ts", ts(skip))]
    pub original_bytes: Option<Arc<Vec<u8>>>,
}

/// Opaque data not represented by first-class model fields.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Extra {
    /// Raw sfnt tables keyed by tag and serialized as base64 strings.
    #[serde(default, with = "raw_tables_serde")]
    #[cfg_attr(feature = "ts", ts(type = "Record<string, string>"))]
    pub raw_tables: BTreeMap<Tag, Vec<u8>>,
    /// Last import provenance.
    #[serde(default)]
    pub source: Option<SourceProvenance>,
    /// Opaque UFO/Glyphs/SFD library entries.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(type = "Record<string, unknown>"))]
    pub lib: BTreeMap<String, serde_json::Value>,
}

mod raw_tables_serde {
    use std::{collections::BTreeMap, str::FromStr};

    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};

    use crate::Tag;

    pub(super) fn serialize<S>(
        tables: &BTreeMap<Tag, Vec<u8>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        tables
            .iter()
            .map(|(tag, bytes)| (tag.to_string(), STANDARD.encode(bytes)))
            .collect::<BTreeMap<_, _>>()
            .serialize(serializer)
    }

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<BTreeMap<Tag, Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        BTreeMap::<String, String>::deserialize(deserializer)?
            .into_iter()
            .map(|(tag, encoded)| {
                let tag = Tag::from_str(&tag).map_err(D::Error::custom)?;
                let bytes = STANDARD.decode(encoded).map_err(D::Error::custom)?;
                Ok((tag, bytes))
            })
            .collect()
    }
}
