//! Input bounds and name/property checks for glyph metadata overlays.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::{self, Write};

use serde::de::{Error, MapAccess, Visitor};
use serde::Deserializer;

use super::{GlyphDataOverlay, GlyphInfoOverride};

pub(super) const MAX_DOCUMENT_BYTES: usize = 4 * 1024 * 1024;
pub(super) const MAX_COMPONENTS: usize = 64;
const MAX_RECORDS: usize = 65_536;
const MAX_ANCHORS: usize = 128;

pub(super) fn deserialize_glyphs<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<String, GlyphInfoOverride>, D::Error>
where
    D: Deserializer<'de>,
{
    struct GlyphsVisitor;

    impl<'de> Visitor<'de> for GlyphsVisitor {
        type Value = BTreeMap<String, GlyphInfoOverride>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a map of unique glyph names to partial metadata")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut glyphs = BTreeMap::new();
            while let Some((name, record)) = map.next_entry::<String, GlyphInfoOverride>()? {
                if glyphs.len() >= MAX_RECORDS {
                    return Err(A::Error::custom("too many glyph data records"));
                }
                if glyphs.insert(name.clone(), record).is_some() {
                    return Err(A::Error::custom(format!("duplicate glyph name: {name}")));
                }
            }
            Ok(glyphs)
        }
    }

    deserializer.deserialize_map(GlyphsVisitor)
}

pub(super) fn validate(overlay: &GlyphDataOverlay) -> Result<(), String> {
    if overlay.version != 1 {
        return Err(format!(
            "unsupported glyph data version: {}",
            overlay.version
        ));
    }
    if overlay.glyphs.len() > MAX_RECORDS {
        return Err("glyph data exceeds 65,536 records".to_owned());
    }
    for (name, record) in &overlay.glyphs {
        identifier(name).map_err(|reason| format!("invalid glyph name {name:?}: {reason}"))?;
        validate_record(record).map_err(|reason| format!("glyph {name:?}: {reason}"))?;
    }
    // Count serialized bytes without allocating a second document-sized buffer.
    serde_json::to_writer(ByteBudget(0), overlay)
        .map_err(|error| format!("glyph data exceeds its document budget: {error}"))
}

fn validate_record(record: &GlyphInfoOverride) -> Result<(), String> {
    if let Some(values) = &record.unicodes {
        if values.len() > MAX_COMPONENTS {
            return Err("more than 64 Unicode values".to_owned());
        }
        if values.iter().any(|value| char::from_u32(*value).is_none()) {
            return Err("Unicode values must be scalar values".to_owned());
        }
        if values.iter().collect::<BTreeSet<_>>().len() != values.len() {
            return Err("duplicate Unicode value".to_owned());
        }
    }
    if let Some(category) = &record.category {
        if ![
            "Letter",
            "Number",
            "Punctuation",
            "Symbol",
            "Separator",
            "Mark",
            "Format",
            "Private",
            "Other",
        ]
        .contains(&category.as_str())
        {
            return Err(format!("unknown category {category:?}"));
        }
    }
    if let Some(script) = &record.script {
        if !crate::all_scripts().contains(&script.as_str()) {
            return Err(format!("unknown ISO 15924 script {script:?}"));
        }
    }
    if let Some(value) = &record.sub_category {
        if value.is_empty()
            || value.len() > 64
            || value.trim() != value
            || value.chars().any(char::is_control)
        {
            return Err("sub_category must contain 1–64 printable bytes".to_owned());
        }
    }
    if let Some(value) = &record.production_name {
        identifier(value).map_err(|reason| format!("invalid production_name: {reason}"))?;
    }
    for (field, values, limit) in [
        ("anchors", record.anchors.as_ref(), MAX_ANCHORS),
        ("composition", record.composition.as_ref(), MAX_COMPONENTS),
    ] {
        let Some(values) = values else { continue };
        if values.len() > limit {
            return Err(format!("{field} exceeds {limit} entries"));
        }
        for value in values {
            identifier(value).map_err(|reason| format!("invalid {field} name: {reason}"))?;
        }
        // Repeated component names are valid, for example the two f's in f_f_i.
        if field == "anchors" && values.iter().collect::<BTreeSet<_>>().len() != values.len() {
            return Err("duplicate anchor name".to_owned());
        }
    }
    Ok(())
}

fn identifier(value: &str) -> Result<(), &'static str> {
    if value.is_empty() || value.len() > 255 {
        return Err("name must contain 1–255 ASCII bytes");
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err("names permit only ASCII letters, digits, period, underscore, and hyphen");
    }
    Ok(())
}

struct ByteBudget(usize);

impl Write for ByteBudget {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0 = self.0.saturating_add(bytes.len());
        if self.0 > MAX_DOCUMENT_BYTES {
            return Err(io::Error::other("maximum size is 4 MiB"));
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
