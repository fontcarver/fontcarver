//! Stable names and identifiers used by the document model.

use std::{fmt, ops::Deref, str::FromStr};

use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

use crate::Error;

/// A persistent glyph name.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GlyphName(#[cfg_attr(feature = "ts", ts(type = "String"))] pub SmolStr);

impl fmt::Display for GlyphName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl From<&str> for GlyphName {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

impl From<String> for GlyphName {
    fn from(value: String) -> Self {
        Self(value.into())
    }
}

impl From<&GlyphName> for String {
    fn from(value: &GlyphName) -> Self {
        value.to_string()
    }
}

impl Deref for GlyphName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

impl AsRef<str> for GlyphName {
    fn as_ref(&self) -> &str {
        self
    }
}

impl std::borrow::Borrow<str> for GlyphName {
    fn borrow(&self) -> &str {
        self
    }
}

/// A session-only stable glyph handle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GlyphId(pub u32);

/// A persistent design-space master identifier.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MasterId(#[cfg_attr(feature = "ts", ts(type = "String"))] pub SmolStr);

impl MasterId {
    /// Generates a UUID v4 identifier without separators.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().simple().to_string().into())
    }
}

impl Default for MasterId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for MasterId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// A persistent glyph layer identifier.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayerId(#[cfg_attr(feature = "ts", ts(type = "String"))] pub SmolStr);

impl LayerId {
    /// Generates a UUID v4 identifier without separators.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().simple().to_string().into())
    }
}

impl Default for LayerId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for LayerId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// A four-byte OpenType table or variation-axis tag.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(transparent)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Tag(pub [u8; 4]);

impl fmt::Display for Tag {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{}", char::from(byte))?;
        }
        Ok(())
    }
}

impl FromStr for Tag {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut characters = value.chars();
        let mut tag = [0; 4];
        for byte in &mut tag {
            let Some(character) = characters.next() else {
                return Err(Error::InvalidTag {
                    tag: value.to_owned(),
                });
            };
            let codepoint = u32::from(character);
            if codepoint > u32::from(u8::MAX) {
                return Err(Error::InvalidTag {
                    tag: value.to_owned(),
                });
            }
            *byte = codepoint as u8;
        }
        if characters.next().is_some() {
            return Err(Error::InvalidTag {
                tag: value.to_owned(),
            });
        }
        Ok(Self(tag))
    }
}
