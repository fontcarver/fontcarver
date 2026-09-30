//! Explicit local script discovery and trusted subprocess execution.

use std::{
    fs::{self, File},
    io::Read,
    path::{Component, Path, PathBuf},
    sync::{atomic::AtomicBool, Mutex},
};

use crate::{
    Api, ApiError, AutomationLanguage, AutomationPythonParams, AutomationPythonResult,
    AutomationScriptEntry, AutomationScriptList, AutomationScriptReadParams,
    AutomationScriptSource, EmptyParams,
};

mod python;

const SOURCE_LIMIT: usize = 64 * 1024;
const SCRIPT_LIMIT: usize = 256;
const ENTRY_LIMIT: usize = 4096;

#[derive(Default)]
pub(crate) struct AutomationState {
    scripts: Mutex<Option<AutomationScriptList>>,
    running: AtomicBool,
}

fn scripts_dir(api: &Api) -> Result<PathBuf, ApiError> {
    let directory = super::prefs::config_dir(api)?.join("scripts");
    match fs::symlink_metadata(&directory) {
        Ok(metadata) if !metadata.is_dir() => {
            return Err(ApiError::invalid_params(
                "scripts directory must be a real directory, not a symlink",
            ));
        }
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            return Err(ApiError::io(error.to_string()));
        }
        _ => {}
    }
    Ok(directory)
}

pub(crate) fn list(api: &Api, params: EmptyParams) -> Result<AutomationScriptList, ApiError> {
    if let Some(cached) = api
        .automation
        .scripts
        .lock()
        .map_err(|_| ApiError::internal("script cache lock is poisoned"))?
        .clone()
    {
        return Ok(cached);
    }
    reload(api, params)
}

pub(crate) fn reload(api: &Api, _params: EmptyParams) -> Result<AutomationScriptList, ApiError> {
    let directory = scripts_dir(api)?;
    fs::create_dir_all(&directory).map_err(|error| ApiError::io(error.to_string()))?;
    let mut result = AutomationScriptList {
        directory: directory.to_string_lossy().into_owned(),
        scripts: Vec::new(),
        warnings: Vec::new(),
    };
    for (index, entry) in fs::read_dir(&directory)
        .map_err(|error| ApiError::io(error.to_string()))?
        .enumerate()
    {
        if index >= ENTRY_LIMIT || result.scripts.len() >= SCRIPT_LIMIT {
            result.warnings.push(format!(
                "Script discovery stopped at the {SCRIPT_LIMIT}-script / {ENTRY_LIMIT}-entry limit"
            ));
            break;
        }
        let entry = entry.map_err(|error| ApiError::io(error.to_string()))?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let Some(language) = language(&name) else {
            continue;
        };
        let metadata =
            fs::symlink_metadata(entry.path()).map_err(|error| ApiError::io(error.to_string()))?;
        if !metadata.is_file() || metadata.len() > SOURCE_LIMIT as u64 {
            if result.warnings.len() < 32 {
                result.warnings.push(format!(
                    "Ignored {name}: scripts must be regular files of at most {SOURCE_LIMIT} bytes"
                ));
            }
            continue;
        }
        result.scripts.push(AutomationScriptEntry {
            name,
            language,
            bytes: metadata.len() as u32,
        });
    }
    result.scripts.sort_by(|a, b| a.name.cmp(&b.name));
    *api.automation
        .scripts
        .lock()
        .map_err(|_| ApiError::internal("script cache lock is poisoned"))? = Some(result.clone());
    Ok(result)
}

pub(crate) fn read(
    api: &Api,
    params: AutomationScriptReadParams,
) -> Result<AutomationScriptSource, ApiError> {
    let path = Path::new(&params.name);
    if params.name.contains(['/', '\\'])
        || path.components().count() != 1
        || !matches!(path.components().next(), Some(Component::Normal(_)))
    {
        return Err(ApiError::invalid_params(
            "script name must be a direct filename",
        ));
    }
    let language = language(&params.name)
        .ok_or_else(|| ApiError::invalid_params("script must have a .js or .py extension"))?;
    let directory = scripts_dir(api)?;
    let path = directory.join(&params.name);
    let metadata = fs::symlink_metadata(&path).map_err(|error| ApiError::io(error.to_string()))?;
    if !metadata.is_file() || metadata.len() > SOURCE_LIMIT as u64 {
        return Err(ApiError::invalid_params(
            "script must be a regular file of at most 65536 bytes; symlinks are not followed",
        ));
    }
    // Check canonical containment as well as the direct-name restriction.
    let root = directory
        .canonicalize()
        .map_err(|error| ApiError::io(error.to_string()))?;
    let canonical = path
        .canonicalize()
        .map_err(|error| ApiError::io(error.to_string()))?;
    if canonical.parent() != Some(root.as_path()) {
        return Err(ApiError::invalid_params(
            "script is outside the scripts directory",
        ));
    }
    let mut source = String::new();
    File::open(path)
        .map_err(|error| ApiError::io(error.to_string()))?
        .take(SOURCE_LIMIT as u64 + 1)
        .read_to_string(&mut source)
        .map_err(|error| ApiError::io(error.to_string()))?;
    if source.len() > SOURCE_LIMIT {
        return Err(ApiError::invalid_params(
            "script source exceeds 65536 bytes",
        ));
    }
    Ok(AutomationScriptSource {
        name: params.name,
        language,
        source,
    })
}

fn language(name: &str) -> Option<AutomationLanguage> {
    match Path::new(name).extension()?.to_str()? {
        "js" => Some(AutomationLanguage::Javascript),
        "py" => Some(AutomationLanguage::Python),
        _ => None,
    }
}

pub(crate) fn run_python(
    api: &Api,
    params: AutomationPythonParams,
) -> Result<AutomationPythonResult, ApiError> {
    python::run(api, params)
}
