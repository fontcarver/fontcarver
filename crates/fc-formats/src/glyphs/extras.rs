//! Narrowly scoped editor metadata, never an alternative native font payload.

use super::{
    error,
    plist::{dict, set, string},
    METADATA,
};
use crate::Error;
use glyphs_reader::Plist;
use serde::{de::DeserializeOwned, Serialize};

pub(super) const GLYPH: &[&str] = &["variation_selectors", "tags", "locked", "user_data"];
pub(super) const MASTER: &[&str] = &[
    "metrics",
    "alignment_zones",
    "stems",
    "guides",
    "private_dict",
    "tt",
    "user_data",
];
pub(super) const LAYER: &[&str] = &[
    "id",
    "background_image",
    "hints",
    "tt_instructions",
    "user_data",
];
pub(super) const FONT: &[&str] = &[
    "info",
    "feature_settings",
    "feature_revision",
    "user_data",
    "guides",
    "extra",
];

pub(super) fn store<T: Serialize>(raw: &mut Plist, value: &T, keys: &[&str]) -> Result<(), Error> {
    let value = serde_json::to_value(value).map_err(|err| error(err.to_string()))?;
    let mut selected = serde_json::Map::new();
    for key in keys {
        if let Some(value) = value.get(*key) {
            let mut value = value.clone();
            if *key == "extra" {
                if let Some(extra) = value.as_object_mut() {
                    extra.remove("source");
                    if let Some(lib) = extra
                        .get_mut("lib")
                        .and_then(serde_json::Value::as_object_mut)
                    {
                        lib.remove(super::SHADOW);
                    }
                }
            }
            selected.insert((*key).into(), value);
        }
    }
    let mut user_data = raw.get("userData").cloned().unwrap_or_else(dict);
    if user_data.as_dict().is_none() {
        return Err(error("userData must be a dictionary"));
    }
    set(
        &mut user_data,
        METADATA,
        string(serde_json::Value::Object(selected).to_string()),
    );
    set(raw, "userData", user_data);
    Ok(())
}

pub(super) fn restore<T: Default + Serialize + DeserializeOwned>(
    raw: &Plist,
    keys: &[&str],
) -> Result<T, Error> {
    let Some(payload) = raw
        .get("userData")
        .and_then(|v| v.get(METADATA))
        .and_then(Plist::as_str)
    else {
        return Ok(T::default());
    };
    let payload: serde_json::Value = serde_json::from_str(payload)
        .map_err(|err| error(format!("invalid editor metadata: {err}")))?;
    let mut value = serde_json::to_value(T::default()).map_err(|err| error(err.to_string()))?;
    let Some(object) = value.as_object_mut() else {
        return Err(error("editor metadata must describe an object"));
    };
    for key in keys {
        if let Some(value) = payload.get(*key) {
            object.insert((*key).into(), value.clone());
        }
    }
    serde_json::from_value(value).map_err(|err| error(format!("invalid editor metadata: {err}")))
}

pub(super) fn preserve_contour_metadata(
    raw: &mut Plist,
    contour: &fc_model::Contour,
) -> Result<(), Error> {
    #[derive(Serialize)]
    struct Extras<'a> {
        name: &'a Option<String>,
        id: &'a Option<String>,
        points: Vec<serde_json::Value>,
    }
    let extras = Extras {
        name: &contour.name,
        id: &contour.id,
        points: contour
            .points
            .iter()
            .map(|p| serde_json::json!({"name":p.name,"tt_index":p.tt_index}))
            .collect(),
    };
    store(raw, &extras, &["name", "id", "points"])
}

pub(super) fn restore_contour_metadata(
    raw: &Plist,
    contour: &mut fc_model::Contour,
) -> Result<(), Error> {
    #[derive(Default, Serialize, serde::Deserialize)]
    struct Extras {
        name: Option<String>,
        id: Option<String>,
        points: Vec<serde_json::Value>,
    }
    let extras: Extras = restore(raw, &["name", "id", "points"])?;
    contour.name = extras.name;
    contour.id = extras.id;
    if extras.points.len() == contour.points.len() {
        for (point, extra) in contour.points.iter_mut().zip(extras.points) {
            point.name = extra
                .get("name")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            if let Some(value) = extra.get("tt_index") {
                point.tt_index =
                    serde_json::from_value(value.clone()).map_err(|err| error(err.to_string()))?;
            }
        }
    }
    Ok(())
}
