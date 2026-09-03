//! Preference and recent-file persistence.

use std::{
    collections::HashSet,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{Api, ApiError, EmptyParams, Prefs, SetPrefsParams};

const MAX_RECENT_FILES: usize = 20;
static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

pub(crate) fn get(api: &Api, _params: EmptyParams) -> Result<Prefs, ApiError> {
    let _guard = api
        .config_lock
        .lock()
        .map_err(|_| ApiError::internal("API config lock is poisoned"))?;
    read_prefs(api)
}

pub(crate) fn set(api: &Api, params: SetPrefsParams) -> Result<Prefs, ApiError> {
    let _guard = api
        .config_lock
        .lock()
        .map_err(|_| ApiError::internal("API config lock is poisoned"))?;
    write_json(api, "prefs.json", &params.prefs)?;
    Ok(params.prefs)
}

pub(crate) fn recent(api: &Api) -> Result<Vec<String>, ApiError> {
    let _guard = api
        .config_lock
        .lock()
        .map_err(|_| ApiError::internal("API config lock is poisoned"))?;
    read_recent_limited(api)
}

pub(crate) fn record_recent(api: &Api, path: String) -> Result<(), ApiError> {
    let _guard = api
        .config_lock
        .lock()
        .map_err(|_| ApiError::internal("API config lock is poisoned"))?;
    let mut recent = read_recent_raw(api)?;
    recent.retain(|candidate| candidate != &path);
    recent.insert(0, path);
    recent.truncate(MAX_RECENT_FILES);
    write_json(api, "recent.json", &recent)
}

fn read_recent_limited(api: &Api) -> Result<Vec<String>, ApiError> {
    let mut recent = read_recent_raw(api)?;
    recent.truncate(recent_limit(api)?);
    Ok(recent)
}

fn recent_limit(api: &Api) -> Result<usize, ApiError> {
    Ok((read_prefs(api)?.recent_limit as usize).min(MAX_RECENT_FILES))
}

fn read_prefs(api: &Api) -> Result<Prefs, ApiError> {
    match read_json(api, "prefs.json") {
        Ok(Some(prefs)) => Ok(prefs),
        Ok(None) => Ok(Prefs::default()),
        Err(error) => {
            tracing::warn!(%error, "failed to load preferences; using defaults");
            Ok(Prefs::default())
        }
    }
}

fn read_recent_raw(api: &Api) -> Result<Vec<String>, ApiError> {
    let recent: Vec<String> = read_json(api, "recent.json")?.unwrap_or_default();
    let mut seen = HashSet::new();
    Ok(recent
        .into_iter()
        .filter(|path| seen.insert(path.clone()))
        .take(MAX_RECENT_FILES)
        .collect())
}

fn read_json<T: serde::de::DeserializeOwned>(api: &Api, name: &str) -> Result<Option<T>, ApiError> {
    let path = config_dir(api)?.join(name);
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

fn write_json(api: &Api, name: &str, value: &impl serde::Serialize) -> Result<(), ApiError> {
    let dir = config_dir(api)?;
    fs::create_dir_all(&dir)
        .map_err(|error| ApiError::io(format!("failed to create `{}`: {error}", dir.display())))?;
    let path = dir.join(name);
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| {
        ApiError::internal(format!("failed to serialize `{}`: {error}", path.display()))
    })?;
    let sequence = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
    let temp_path = dir.join(format!(".{name}.{}.{}.tmp", std::process::id(), sequence));
    fs::write(&temp_path, bytes).map_err(|error| {
        ApiError::io(format!(
            "failed to write temporary config `{}`: {error}",
            temp_path.display()
        ))
    })?;
    if let Err(error) = fs::rename(&temp_path, &path) {
        let _ = fs::remove_file(&temp_path);
        return Err(ApiError::io(format!(
            "failed to replace `{}`: {error}",
            path.display()
        )));
    }
    Ok(())
}

fn config_dir(api: &Api) -> Result<PathBuf, ApiError> {
    if let Some(path) = api.config_dir() {
        return Ok(path.to_owned());
    }
    if let Some(path) = std::env::var_os("FW_CONFIG_DIR") {
        return Ok(PathBuf::from(path));
    }
    dirs::config_dir()
        .map(|path| path.join("fontworkshop"))
        .ok_or_else(|| ApiError::internal("could not determine the user config directory"))
}
