//! Interoperable writer ownership for flattened UFO feature source.

use fc_model::FeatureWriterPolicy;
use plist::{Dictionary, Value};

use crate::Error;

const WRITERS_KEY: &str = "com.github.googlei18n.ufo2ft.featureWriters";
const WRITERS: [&str; 4] = [
    "KernFeatureWriter",
    "MarkFeatureWriter",
    "CursFeatureWriter",
    "GdefFeatureWriter",
];

/// Read the subset of ufo2ft writer settings expressible by the active model.
/// Unsupported configurations fail instead of silently enabling extra rules.
pub(crate) fn read_policy(lib: &Dictionary) -> Result<Option<FeatureWriterPolicy>, Error> {
    let Some(value) = lib.get(WRITERS_KEY) else {
        return Ok(None);
    };
    let entries = value
        .as_array()
        .ok_or_else(|| unsupported("expected a writer list"))?;
    let mut enabled = [false; 4];
    for entry in entries {
        let entry = entry
            .as_dictionary()
            .ok_or_else(|| unsupported("expected a writer dictionary"))?;
        let class = entry
            .get("class")
            .and_then(Value::as_string)
            .ok_or_else(|| unsupported("missing writer class"))?;
        let index = WRITERS
            .iter()
            .position(|known| *known == class)
            .ok_or_else(|| unsupported(format!("unrepresentable writer {class}")))?;
        if enabled[index] {
            return Err(unsupported(format!("duplicate writer {class}")));
        }
        if let Some(module) = entry.get("module") {
            let expected = match index {
                0 => "ufo2ft.featureWriters.kernFeatureWriter",
                1 => "ufo2ft.featureWriters.markFeatureWriter",
                2 => "ufo2ft.featureWriters.cursFeatureWriter",
                _ => "ufo2ft.featureWriters.gdefFeatureWriter",
            };
            if !matches!(module.as_string(), Some("ufo2ft.featureWriters"))
                && module.as_string() != Some(expected)
            {
                return Err(unsupported(format!("unrepresentable module for {class}")));
            }
        }
        if let Some(options) = entry.get("options") {
            let options = options
                .as_dictionary()
                .ok_or_else(|| unsupported("expected writer options dictionary"))?;
            for (key, value) in options {
                let compatible = match key.as_str() {
                    "mode" => value.as_string() == Some("skip"),
                    "ignoreMarks" if index == 0 => value.as_boolean() == Some(true),
                    "quantization" if index <= 1 => value.as_signed_integer() == Some(1),
                    "groupMarkClasses" if index == 1 => value.as_boolean() == Some(false),
                    _ => false,
                };
                if !compatible {
                    return Err(unsupported(format!("unrepresentable {class} option {key}")));
                }
            }
        }
        enabled[index] = true;
    }
    if enabled[1] != enabled[2] || enabled[1] != enabled[3] {
        return Err(unsupported(
            "independent mark, cursive and GDEF writers require a finer writer policy",
        ));
    }
    Ok(Some(FeatureWriterPolicy {
        kerning: enabled[0],
        anchors: enabled[1],
    }))
}

/// Export the same built-in Skip writers used by `fc-compile`'s source adapter.
pub(crate) fn write_policy(policy: &FeatureWriterPolicy, lib: &mut Dictionary) {
    if policy == &FeatureWriterPolicy::default() && !lib.contains_key(WRITERS_KEY) {
        return;
    }
    let entries = WRITERS
        .iter()
        .enumerate()
        .filter(|(index, _)| {
            if *index == 0 {
                policy.kerning
            } else {
                policy.anchors
            }
        })
        .map(|(_, class)| {
            Value::Dictionary(
                [
                    ("class".to_owned(), Value::String((*class).to_owned())),
                    (
                        "options".to_owned(),
                        Value::Dictionary(
                            [("mode".to_owned(), Value::String("skip".to_owned()))]
                                .into_iter()
                                .collect(),
                        ),
                    ),
                ]
                .into_iter()
                .collect(),
            )
        })
        .collect();
    lib.insert(WRITERS_KEY.to_owned(), Value::Array(entries));
}

fn unsupported(reason: impl std::fmt::Display) -> Error {
    Error::UnsupportedPlist(format!("{WRITERS_KEY}: {reason}"))
}
