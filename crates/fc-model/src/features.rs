//! Editable OpenType feature source.

use serde::{Deserialize, Serialize};

/// Authoritative representation for OpenType layout data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum FeaturesSource {
    /// Editable Adobe feature syntax.
    #[default]
    Fea,
    /// Tables imported without a canonical feature source.
    ImportedTables,
}

/// A feature-file prefix or tagged feature block.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeatureBlock {
    /// Feature tag or prefix name.
    #[serde(default)]
    pub tag: String,
    /// Feature syntax source.
    #[serde(default)]
    pub code: String,
    /// Whether the block is generated automatically.
    #[serde(default)]
    pub automatic: bool,
    /// Whether the block is disabled.
    #[serde(default)]
    pub disabled: bool,
    /// Free-form editor notes.
    #[serde(default)]
    pub notes: String,
}

/// A named feature-file glyph class.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FeatureClass {
    /// Class name without the leading at-sign.
    #[serde(default)]
    pub name: String,
    /// Class contents in feature syntax.
    #[serde(default)]
    pub code: String,
    /// Whether the class is generated automatically.
    #[serde(default)]
    pub automatic: bool,
}

/// All editable feature source associated with a font.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Features {
    /// Prefix blocks emitted before classes and features.
    #[serde(default)]
    pub prefixes: Vec<FeatureBlock>,
    /// Named glyph classes.
    #[serde(default)]
    pub classes: Vec<FeatureClass>,
    /// Tagged feature blocks.
    #[serde(default)]
    pub features: Vec<FeatureBlock>,
    /// Which representation is authoritative.
    #[serde(default)]
    pub source_of_truth: FeaturesSource,
}

impl Features {
    /// Assemble the effective FEA used by compilation and source queries.
    ///
    /// Imported-table authority emits no FEA. Disabled prefixes/features are
    /// omitted; `automatic` is metadata and does not suppress stored source.
    /// This preserves the existing source spelling without parsing or loading
    /// includes. Callers must validate the result before interpreting it.
    pub fn to_fea(&self) -> String {
        if self.source_of_truth != FeaturesSource::Fea {
            return String::new();
        }
        let mut source = String::new();
        for prefix in self.prefixes.iter().filter(|block| !block.disabled) {
            source.push_str(&prefix.code);
            source.push('\n');
        }
        for class in &self.classes {
            source.push('@');
            source.push_str(class.name.trim_start_matches('@'));
            source.push_str(" = ");
            append_class_body(&mut source, &class.code);
            source.push_str(";\n");
        }
        for feature in self.features.iter().filter(|block| !block.disabled) {
            let trimmed = feature.code.trim();
            if trimmed.starts_with("feature ") {
                source.push_str(trimmed);
                source.push('\n');
            } else {
                source.push_str("feature ");
                source.push_str(&feature.tag);
                source.push_str(" {\n");
                source.push_str(&feature.code);
                source.push_str("\n} ");
                source.push_str(&feature.tag);
                source.push_str(";\n");
            }
        }
        source
    }
}

// FeatureClass::code is a body, but older documents may contain the complete
// bracketed class. Preserve either spelling verbatim so class references,
// comments, and whitespace are not damaged by character trimming.
fn append_class_body(target: &mut String, body: &str) {
    let trimmed = body.trim();
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        target.push_str(body);
    } else {
        target.push('[');
        target.push_str(body);
        target.push(']');
    }
}

#[cfg(test)]
mod tests {
    use super::append_class_body;

    #[test]
    fn feature_class_bodies_are_not_trimmed_by_bracket_characters() {
        let mut bracketed = String::new();
        append_class_body(&mut bracketed, "[ A B ]");
        assert_eq!(bracketed, "[ A B ]");

        let mut body = String::new();
        append_class_body(&mut body, " A @Uppercase # keep this comment\n");
        assert_eq!(body, "[ A @Uppercase # keep this comment\n]");
    }
}
