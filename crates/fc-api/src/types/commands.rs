//! Document-mutation API types.

use fc_doc::{Command, DocId};
use serde::{de::Error as _, Deserialize, Deserializer, Serialize};
use ts_rs::TS;

/// Parameters for applying one labelled command.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ApplyParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Mutation to apply.
    pub command: Command,
    /// Undo label.
    pub label: String,
}

impl<'de> Deserialize<'de> for ApplyParams {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct WireParams {
            doc: DocId,
            command: serde_json::Value,
            label: String,
        }

        let params = WireParams::deserialize(deserializer)?;
        Ok(Self {
            doc: params.doc,
            command: command_from_value(params.command).map_err(D::Error::custom)?,
            label: params.label,
        })
    }
}

/// Parameters for starting a coalesced edit.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct EditBeginParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Undo label for the edit gesture.
    pub label: String,
}

/// Transport-safe identifier for a coalesced edit gesture.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[serde(transparent)]
#[ts(export)]
pub struct EditTokenId(#[ts(type = "number")] pub u64);

/// Parameters for applying a command within a coalesced edit.
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct EditApplyParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Active edit token.
    pub token: EditTokenId,
    /// Mutation to apply.
    pub command: Command,
}

impl<'de> Deserialize<'de> for EditApplyParams {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct WireParams {
            doc: DocId,
            token: EditTokenId,
            command: serde_json::Value,
        }

        let params = WireParams::deserialize(deserializer)?;
        Ok(Self {
            doc: params.doc,
            token: params.token,
            command: command_from_value(params.command).map_err(D::Error::custom)?,
        })
    }
}

fn command_from_value(value: serde_json::Value) -> serde_json::Result<Command> {
    let mut parse_value = value.clone();
    replace_explicit_nulls(&mut parse_value);
    let mut command: Command = serde_json::from_value(parse_value)?;
    restore_double_options(&mut command, &value)?;
    Ok(command)
}

fn replace_explicit_nulls(value: &mut serde_json::Value) {
    if let Some(fields) = value
        .get_mut("SetGlyphMeta")
        .and_then(serde_json::Value::as_object_mut)
    {
        if fields
            .get("color_label")
            .is_some_and(serde_json::Value::is_null)
        {
            fields.insert("color_label".to_owned(), serde_json::Value::from(0));
        }
    }
    if let Some(values) = value
        .get_mut("Batch")
        .and_then(serde_json::Value::as_array_mut)
    {
        for value in values {
            replace_explicit_nulls(value);
        }
    }
}

fn restore_double_options(
    command: &mut Command,
    value: &serde_json::Value,
) -> serde_json::Result<()> {
    match command {
        Command::SetGlyphMeta { color_label, .. } => {
            let field = value
                .get("SetGlyphMeta")
                .and_then(serde_json::Value::as_object)
                .and_then(|fields| fields.get("color_label"));
            *color_label = fc_doc::double_option::from_value(field)?;
        }
        Command::Batch(commands) => {
            if let Some(values) = value.get("Batch").and_then(serde_json::Value::as_array) {
                for (command, value) in commands.iter_mut().zip(values) {
                    restore_double_options(command, value)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

/// Parameters for ending a coalesced edit.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct EditEndParams {
    /// Open document identifier.
    pub doc: DocId,
    /// Active edit token.
    pub token: EditTokenId,
}
