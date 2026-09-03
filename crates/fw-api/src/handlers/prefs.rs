//! Preference and recent-file persistence.

use std::{collections::HashSet, fs, path::PathBuf};

use crate::{Api, ApiError, EmptyParams, Prefs, SetPrefsParams};

const MAX_RECENT_FILES: usize = 20;

pub(crate) fn get(api: &Api, _params: EmptyParams) -> Result<Prefs, ApiError> {
    let _guard = api
        .config_lock
        .lock()
        .map_err(|_| ApiError::internal("API config lock is poisoned"))?;
    read_prefs()
}

pub(crate) fn set(api: &Api, params: SetPrefsParams) -> Result<Prefs, ApiError> {
    let _guard = api
        .config_lock
        .lock()
        .map_err(|_| ApiError::internal("API config lock is poisoned"))?;
    write_json("prefs.json", &params.prefs)?;
    Ok(params.prefs)
}

pub(crate) fn recent(api: &Api) -> Result<Vec<String>, ApiError> {
    let _guard = api
        .config_lock
        .lock()
        .map_err(|_| ApiError::internal("API config lock is poisoned"))?;
    read_recent_limited()
}

pub(crate) fn record_recent(api: &Api, path: String) -> Result<(), ApiError> {
    let _guard = api
        .config_lock
        .lock()
        .map_err(|_| ApiError::internal("API config lock is poisoned"))?;
    let mut recent = read_recent_raw()?;
    recent.retain(|candidate| candidate != &path);
    recent.insert(0, path);
    let limit = recent_limit()?;
    recent.truncate(limit);
    write_json("recent.json", &recent)
}

fn read_recent_limited() -> Result<Vec<String>, ApiError> {
    let mut recent = read_recent_raw()?;
    recent.truncate(recent_limit()?);
    Ok(recent)
}

fn recent_limit() -> Result<usize, ApiError> {
    Ok((read_prefs()?.recent_limit as usize).min(MAX_RECENT_FILES))
}

fn read_prefs() -> Result<Prefs, ApiError> {
    read_json("prefs.json")?.map_or_else(|| Ok(Prefs::default()), Ok)
}

fn read_recent_raw() -> Result<Vec<String>, ApiError> {
    let recent: Vec<String> = read_json("recent.json")?.unwrap_or_default();
    let mut seen = HashSet::new();
    Ok(recent
        .into_iter()
        .filter(|path| seen.insert(path.clone()))
        .take(MAX_RECENT_FILES)
        .collect())
}

fn read_json<T: serde::de::DeserializeOwned>(name: &str) -> Result<Option<T>, ApiError> {
    let path = config_dir()?.join(name);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(ApiError::io(format!(
                "failed to read `{}`: {error}",
                path.display()
            )))
        }
    };
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|error| ApiError::parse(format!("failed to parse `{}`: {error}", path.display())))
}

fn write_json(name: &str, value: &impl serde::Serialize) -> Result<(), ApiError> {
    let dir = config_dir()?;
    fs::create_dir_all(&dir)
        .map_err(|error| ApiError::io(format!("failed to create `{}`: {error}", dir.display())))?;
    let path = dir.join(name);
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| {
        ApiError::internal(format!("failed to serialize `{}`: {error}", path.display()))
    })?;
    fs::write(&path, bytes)
        .map_err(|error| ApiError::io(format!("failed to write `{}`: {error}", path.display())))
}

fn config_dir() -> Result<PathBuf, ApiError> {
    if let Some(path) = std::env::var_os("FW_CONFIG_DIR") {
        return Ok(PathBuf::from(path));
    }
    dirs::config_dir()
        .map(|path| path.join("fontworkshop"))
        .ok_or_else(|| ApiError::internal("could not determine the user config directory"))
}
