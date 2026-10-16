//! Preference and recent-file persistence.

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
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
    validate(&params.prefs)?;
    {
        let _guard = api
            .config_lock
            .lock()
            .map_err(|_| ApiError::internal("API config lock is poisoned"))?;
        write_json(api, "prefs.json", &params.prefs)?;
    }
    // Autosave timing belongs to these preferences, so it takes effect now
    // instead of at the next launch. Reconciling needs the configuration lock
    // itself, so the guard above is released first.
    api.reconcile_autosave()?;
    Ok(params.prefs)
}

fn validate(prefs: &Prefs) -> Result<(), ApiError> {
    for (name, value, min, max) in [
        ("snap_distance", prefs.snap_distance, 0.0, 64.0),
        ("hit_distance", prefs.hit_distance, 1.0, 64.0),
        ("handle_size", prefs.handle_size, 2.0, 20.0),
        ("node_size", prefs.node_size, 2.0, 20.0),
    ] {
        if !value.is_finite() || !(min..=max).contains(&value) {
            return Err(ApiError::invalid_params(format!(
                "{name} must be finite and between {min} and {max} CSS pixels"
            )));
        }
    }
    let distances = &prefs.snap.distances;
    for (name, value) in [
        ("snap.distances.nodes", distances.nodes),
        ("snap.distances.handles", distances.handles),
        ("snap.distances.anchors", distances.anchors),
        ("snap.distances.guides", distances.guides),
        ("snap.distances.metrics", distances.metrics),
        ("snap.distances.grid", distances.grid),
        ("snap.distances.components", distances.components),
        ("snap.distances.zones", distances.zones),
        ("snap.distances.background", distances.background),
    ] {
        if !value.is_finite() || !(0.0..=64.0).contains(&value) {
            return Err(ApiError::invalid_params(format!(
                "{name} must be finite and between 0 and 64 CSS pixels, where 0 uses snap_distance"
            )));
        }
    }
    if !(1..=100).contains(&prefs.grid_subdivision) {
        return Err(ApiError::invalid_params(
            "grid_subdivision must be between 1 and 100",
        ));
    }
    Ok(())
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

pub(crate) fn config_dir(api: &Api) -> Result<PathBuf, ApiError> {
    if let Some(path) = api.config_dir() {
        return Ok(path.to_owned());
    }
    if let Some(path) = std::env::var_os("FC_CONFIG_DIR") {
        return Ok(PathBuf::from(path));
    }
    let root = dirs::config_dir()
        .ok_or_else(|| ApiError::internal("could not determine the user config directory"))?;
    product_config_dir(&root)
}

fn product_config_dir(root: &Path) -> Result<PathBuf, ApiError> {
    let current = root.join("fontcarver");
    let legacy = root.join("fontcutter");
    // Move preferences, recent files and recovery snapshots together on first launch.
    if !current.exists() && legacy.is_dir() {
        if let Err(error) = fs::rename(&legacy, &current) {
            // Another running instance may have completed the same migration.
            if !current.is_dir() {
                return Err(ApiError::io(format!(
                    "failed to migrate `{}` to `{}`: {error}",
                    legacy.display(),
                    current.display()
                )));
            }
        }
    }
    Ok(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_preferences_default_new_snap_families_and_distances() {
        let root = tempfile::tempdir().unwrap();
        let api = Api::with_config_dir(root.path());
        let legacy = serde_json::json!({
            "theme": "dark", "language": "en", "recent_limit": 12,
            "autosave_seconds": 30, "grid_size": 8.0,
            "snap": { "nodes": false, "guides": false }
        });
        fs::write(root.path().join("prefs.json"), legacy.to_string()).unwrap();

        let loaded = serde_json::to_value(get(&api, EmptyParams {}).unwrap()).unwrap();
        assert_eq!(loaded["theme"], "dark");
        assert_eq!(loaded["snap"]["nodes"], false);
        assert_eq!(loaded["snap"]["guides"], false);
        for family in ["handles", "anchors", "zones", "background"] {
            assert_eq!(loaded["snap"][family], true, "{family}");
        }
        assert_eq!(loaded["snap_distance"], 6.0);
        assert_eq!(loaded["hit_distance"], 5.0);
        assert_eq!(loaded["handle_size"], 3.5);
        assert_eq!(loaded["node_size"], 5.0);
        assert_eq!(loaded["grid_subdivision"], 1);
    }

    #[test]
    fn snapping_preferences_persist_across_api_instances() {
        let root = tempfile::tempdir().unwrap();
        let api = Api::with_config_dir(root.path());
        let mut requested = serde_json::to_value(Prefs::default()).unwrap();
        requested["snap_distance"] = serde_json::json!(9.5);
        requested["hit_distance"] = serde_json::json!(7.25);
        requested["handle_size"] = serde_json::json!(9.0);
        requested["node_size"] = serde_json::json!(12.0);
        requested["grid_subdivision"] = serde_json::json!(8);
        for family in ["handles", "anchors", "zones", "background"] {
            requested["snap"][family] = serde_json::json!(false);
        }
        let prefs = serde_json::from_value(requested.clone()).unwrap();
        let saved = set(&api, SetPrefsParams { prefs }).unwrap();
        assert_eq!(serde_json::to_value(saved).unwrap(), requested);

        let reopened = Api::with_config_dir(root.path());
        let loaded = get(&reopened, EmptyParams {}).unwrap();
        assert_eq!(serde_json::to_value(loaded).unwrap(), requested);
    }

    #[test]
    fn invalid_snapping_preferences_preserve_existing_file() {
        let root = tempfile::tempdir().unwrap();
        let api = Api::with_config_dir(root.path());
        set(
            &api,
            SetPrefsParams {
                prefs: Prefs::default(),
            },
        )
        .unwrap();
        let path = root.path().join("prefs.json");
        let original = fs::read(&path).unwrap();
        for (field, invalid) in [
            ("snap_distance", serde_json::json!(-0.1)),
            ("snap_distance", serde_json::json!(64.1)),
            ("hit_distance", serde_json::json!(0.9)),
            ("hit_distance", serde_json::json!(64.1)),
            ("handle_size", serde_json::json!(1.9)),
            ("handle_size", serde_json::json!(20.1)),
            ("node_size", serde_json::json!(1.9)),
            ("node_size", serde_json::json!(20.1)),
            ("grid_subdivision", serde_json::json!(0)),
            ("grid_subdivision", serde_json::json!(101)),
        ] {
            let mut value = serde_json::to_value(Prefs::default()).unwrap();
            value[field] = invalid;
            let prefs = serde_json::from_value(value).unwrap();
            let error = set(&api, SetPrefsParams { prefs }).unwrap_err();
            assert_eq!(error.code, "invalid_params");
            assert!(error.message.contains(field), "{error:?}");
            assert_eq!(fs::read(&path).unwrap(), original, "{field}");
        }
    }

    #[test]
    fn snapping_preference_boundaries_are_inclusive() {
        let root = tempfile::tempdir().unwrap();
        let api = Api::with_config_dir(root.path());
        for prefs in [
            Prefs {
                snap_distance: 0.0,
                hit_distance: 1.0,
                handle_size: 2.0,
                node_size: 2.0,
                grid_subdivision: 1,
                ..Prefs::default()
            },
            Prefs {
                snap_distance: 64.0,
                hit_distance: 64.0,
                handle_size: 20.0,
                node_size: 20.0,
                grid_subdivision: 100,
                ..Prefs::default()
            },
        ] {
            set(
                &api,
                SetPrefsParams {
                    prefs: prefs.clone(),
                },
            )
            .unwrap();
            assert_eq!(get(&api, EmptyParams {}).unwrap(), prefs);
        }
    }

    #[test]
    fn nonfinite_snapping_preferences_preserve_existing_file() {
        let root = tempfile::tempdir().unwrap();
        let api = Api::with_config_dir(root.path());
        set(
            &api,
            SetPrefsParams {
                prefs: Prefs::default(),
            },
        )
        .unwrap();
        let path = root.path().join("prefs.json");
        let original = fs::read(&path).unwrap();
        for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            for (field, prefs) in [
                (
                    "snap_distance",
                    Prefs {
                        snap_distance: invalid,
                        ..Prefs::default()
                    },
                ),
                (
                    "hit_distance",
                    Prefs {
                        hit_distance: invalid,
                        ..Prefs::default()
                    },
                ),
                (
                    "handle_size",
                    Prefs {
                        handle_size: invalid,
                        ..Prefs::default()
                    },
                ),
                (
                    "node_size",
                    Prefs {
                        node_size: invalid,
                        ..Prefs::default()
                    },
                ),
            ] {
                let error = set(&api, SetPrefsParams { prefs }).unwrap_err();
                assert_eq!(error.code, "invalid_params");
                assert!(error.message.contains(field), "{error:?}");
                assert_eq!(fs::read(&path).unwrap(), original, "{field}");
            }
        }
    }

    #[test]
    fn migrates_preferences_and_recovery_together() {
        let root = tempfile::tempdir().unwrap();
        let legacy = root.path().join("fontcutter");
        fs::create_dir_all(legacy.join("autosave")).unwrap();
        fs::write(legacy.join("prefs.json"), b"preferences").unwrap();
        fs::write(legacy.join("recent.json"), b"recent files").unwrap();
        fs::write(legacy.join("autosave/work.fcf"), b"unsaved work").unwrap();

        let current = product_config_dir(root.path()).unwrap();
        assert_eq!(current, root.path().join("fontcarver"));
        assert_eq!(
            fs::read(current.join("prefs.json")).unwrap(),
            b"preferences"
        );
        assert_eq!(
            fs::read(current.join("recent.json")).unwrap(),
            b"recent files"
        );
        assert_eq!(
            fs::read(current.join("autosave/work.fcf")).unwrap(),
            b"unsaved work"
        );
        assert!(!legacy.exists());
        assert_eq!(product_config_dir(root.path()).unwrap(), current);
    }

    #[test]
    fn existing_current_configuration_is_not_overwritten() {
        let root = tempfile::tempdir().unwrap();
        let legacy = root.path().join("fontcutter");
        let current = root.path().join("fontcarver");
        fs::create_dir(&legacy).unwrap();
        fs::create_dir(&current).unwrap();
        fs::write(legacy.join("prefs.json"), b"legacy").unwrap();
        fs::write(current.join("prefs.json"), b"current").unwrap();

        assert_eq!(product_config_dir(root.path()).unwrap(), current);
        assert_eq!(fs::read(current.join("prefs.json")).unwrap(), b"current");
        assert_eq!(fs::read(legacy.join("prefs.json")).unwrap(), b"legacy");
    }
}
