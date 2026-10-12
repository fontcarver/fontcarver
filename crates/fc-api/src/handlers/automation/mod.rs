//! Explicit local script discovery, script transactions and subprocess execution.

use std::{
    fs::{self, File},
    io::Read,
    path::{Component, Path, PathBuf},
    sync::{atomic::AtomicBool, Mutex},
    time::{Duration, UNIX_EPOCH},
};

use fc_doc::{DocId, EditToken};
use sha2::{Digest, Sha256};

use crate::{
    Api, ApiError, AutomationLanguage, AutomationPythonParams, AutomationPythonResult,
    AutomationScriptEntry, AutomationScriptFolder, AutomationScriptFolderKind,
    AutomationScriptList, AutomationScriptListParams, AutomationScriptReadParams,
    AutomationScriptSource, AutomationSettings, AutomationSettingsParams, AutomationTransaction,
    AutomationTransactionEnd, AutomationTransactionEndParams, AutomationTransactionParams,
    EmptyParams,
};

mod python;

const SOURCE_LIMIT: usize = 64 * 1024;
const SCRIPT_LIMIT: usize = 256;
const ENTRY_LIMIT: usize = 4096;
const SETTINGS_FILE: &str = "automation.json";
const MAX_LABEL: usize = 120;

/// Bounds a script transaction the host opened but may never close.
const MAX_TRANSACTION_MS: u32 = 120_000;
const DEFAULT_TRANSACTION_MS: u32 = 60_000;

#[derive(Default)]
pub(crate) struct AutomationState {
    scripts: Mutex<Option<(Option<DocId>, AutomationScriptList)>>,
    running: AtomicBool,
    /// Script transactions this host opened and has not yet closed.
    ///
    /// A reloaded page or a lost transport leaves an entry behind; the next run
    /// supersedes it, and every listing reaps the ones whose deadline passed.
    transactions: Mutex<Vec<(DocId, u64)>>,
}

/// One searched script root, resolved before any directory is read.
struct Root {
    kind: AutomationScriptFolderKind,
    path: PathBuf,
}

fn real_directory(path: &Path) -> Result<bool, ApiError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(metadata.is_dir()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(ApiError::io(error.to_string())),
    }
}

fn preferences_dir(api: &Api) -> Result<PathBuf, ApiError> {
    let directory = super::prefs::config_dir(api)?.join("scripts");
    if fs::symlink_metadata(&directory).is_ok_and(|metadata| !metadata.is_dir()) {
        return Err(ApiError::invalid_params(
            "scripts directory must be a real directory, not a symlink",
        ));
    }
    Ok(directory)
}

fn settings_path(api: &Api) -> Result<PathBuf, ApiError> {
    Ok(super::prefs::config_dir(api)?.join(SETTINGS_FILE))
}

/// Reads the persisted scripting preferences, tolerating an absent file.
pub(crate) fn settings_get(
    api: &Api,
    _params: EmptyParams,
) -> Result<AutomationSettings, ApiError> {
    let path = settings_path(api)?;
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(AutomationSettings::default());
    };
    // A corrupt preference file must not make the console unusable.
    Ok(serde_json::from_str(&text).unwrap_or_default())
}

/// Validates and stores the extra script folder.
pub(crate) fn settings_set(
    api: &Api,
    params: AutomationSettingsParams,
) -> Result<AutomationSettings, ApiError> {
    let settings = match params
        .scripts_directory
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        None => AutomationSettings {
            scripts_directory: None,
        },
        Some(value) => {
            let path = Path::new(value);
            if !path.is_absolute() {
                return Err(ApiError::invalid_params(
                    "the scripts folder must be an absolute path",
                ));
            }
            if !real_directory(path)? {
                return Err(ApiError::invalid_params(
                    "the scripts folder must be an existing directory and not a symlink",
                ));
            }
            AutomationSettings {
                scripts_directory: Some(path.to_string_lossy().into_owned()),
            }
        }
    };
    let path = settings_path(api)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| ApiError::io(error.to_string()))?;
    }
    let text = serde_json::to_vec_pretty(&settings)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    fs::write(&path, text).map_err(|error| ApiError::io(error.to_string()))?;
    invalidate(api)?;
    Ok(settings)
}

/// A project script folder beside the captured document, when one exists.
fn project_dir(api: &Api, doc: Option<DocId>) -> Result<Option<PathBuf>, ApiError> {
    let Some(doc) = doc else { return Ok(None) };
    let Some(path) = api
        .session
        .with_doc(doc, |document| document.path.clone())
        .map_err(ApiError::from)?
    else {
        return Ok(None);
    };
    // A directory project owns its folder; a single file shares its parent.
    let root = if real_directory(&path)? {
        path
    } else {
        match path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
            _ => return Ok(None),
        }
    };
    Ok(Some(root.join("scripts")))
}

fn roots(api: &Api, doc: Option<DocId>) -> Result<Vec<Root>, ApiError> {
    let mut roots = Vec::new();
    if let Some(configured) = settings_get(api, EmptyParams::default())?.scripts_directory {
        roots.push(Root {
            kind: AutomationScriptFolderKind::Configured,
            path: PathBuf::from(configured),
        });
    }
    roots.push(Root {
        kind: AutomationScriptFolderKind::Preferences,
        path: preferences_dir(api)?,
    });
    if let Some(project) = project_dir(api, doc)? {
        roots.push(Root {
            kind: AutomationScriptFolderKind::Project,
            path: project,
        });
    }
    // Identical paths would list every script twice.
    let mut seen = Vec::new();
    roots.retain(|root| {
        let duplicate = seen.contains(&root.path);
        if !duplicate {
            seen.push(root.path.clone());
        }
        !duplicate
    });
    Ok(roots)
}

fn root_path(
    api: &Api,
    doc: Option<DocId>,
    kind: AutomationScriptFolderKind,
) -> Result<PathBuf, ApiError> {
    roots(api, doc)?
        .into_iter()
        .find(|root| root.kind == kind)
        .map(|root| root.path)
        .ok_or_else(|| ApiError::not_found("that scripts folder is not configured"))
}

fn recorded(api: &Api) -> Result<Vec<(DocId, u64)>, ApiError> {
    Ok(api
        .automation
        .transactions
        .lock()
        .map_err(|_| ApiError::internal("script transaction lock is poisoned"))?
        .clone())
}

fn forget(api: &Api, doc: DocId, token: u64) -> Result<(), ApiError> {
    api.automation
        .transactions
        .lock()
        .map_err(|_| ApiError::internal("script transaction lock is poisoned"))?
        .retain(|entry| entry != &(doc, token));
    Ok(())
}

/// Closes recorded transactions this host can no longer own.
///
/// `supersede` closes every one of them, which is what a new run does. Without
/// it only the ones whose deadline has passed are reaped, so calling this while
/// a run is live never disturbs it.
fn reap(api: &Api, supersede: bool) -> Result<(), ApiError> {
    for (doc, token) in recorded(api)? {
        let closed = api
            .session
            .with_doc_mut(doc, |document| {
                if supersede {
                    document.finalize_transaction()
                } else {
                    document.finalize_expired_transaction()
                }
            })
            // A closed document takes its transaction with it.
            .unwrap_or(true);
        if closed {
            forget(api, doc, token)?;
        }
    }
    Ok(())
}

/// Returns the cached listing for these roots, discovering it when necessary.
pub(crate) fn list(
    api: &Api,
    params: AutomationScriptListParams,
) -> Result<AutomationScriptList, ApiError> {
    reap(api, false)?;
    if let Some((doc, cached)) = api
        .automation
        .scripts
        .lock()
        .map_err(|_| ApiError::internal("script cache lock is poisoned"))?
        .clone()
    {
        if doc == params.doc {
            return Ok(cached);
        }
    }
    reload(api, params)
}

/// Rediscovers every searched root without executing a script.
pub(crate) fn reload(
    api: &Api,
    params: AutomationScriptListParams,
) -> Result<AutomationScriptList, ApiError> {
    reap(api, false)?;
    let roots = roots(api, params.doc)?;
    let mut result = AutomationScriptList {
        folders: Vec::new(),
        scripts: Vec::new(),
        warnings: Vec::new(),
        fingerprint: String::new(),
    };
    let mut digest = Sha256::new();
    for root in &roots {
        // Only the preferences folder is created; the others are the user's own.
        if root.kind == AutomationScriptFolderKind::Preferences {
            fs::create_dir_all(&root.path).map_err(|error| ApiError::io(error.to_string()))?;
        }
        let exists = real_directory(&root.path)?;
        result.folders.push(AutomationScriptFolder {
            kind: root.kind,
            path: root.path.to_string_lossy().into_owned(),
            exists,
        });
        digest.update(root.path.to_string_lossy().as_bytes());
        digest.update([u8::from(exists)]);
        if !exists {
            continue;
        }
        scan(root, &mut result, &mut digest)?;
    }
    result.scripts.sort_by(|a, b| {
        folder_order(a.folder)
            .cmp(&folder_order(b.folder))
            .then_with(|| a.name.cmp(&b.name))
    });
    result.fingerprint = hex(digest.finalize().as_slice());
    *api.automation
        .scripts
        .lock()
        .map_err(|_| ApiError::internal("script cache lock is poisoned"))? =
        Some((params.doc, result.clone()));
    Ok(result)
}

fn invalidate(api: &Api) -> Result<(), ApiError> {
    *api.automation
        .scripts
        .lock()
        .map_err(|_| ApiError::internal("script cache lock is poisoned"))? = None;
    Ok(())
}

fn folder_order(kind: AutomationScriptFolderKind) -> u8 {
    match kind {
        AutomationScriptFolderKind::Configured => 0,
        AutomationScriptFolderKind::Preferences => 1,
        AutomationScriptFolderKind::Project => 2,
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn scan(
    root: &Root,
    result: &mut AutomationScriptList,
    digest: &mut Sha256,
) -> Result<(), ApiError> {
    let mut found = Vec::new();
    for (index, entry) in fs::read_dir(&root.path)
        .map_err(|error| ApiError::io(error.to_string()))?
        .enumerate()
    {
        if index >= ENTRY_LIMIT || result.scripts.len() + found.len() >= SCRIPT_LIMIT {
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
        let modified = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |since| since.as_nanos());
        found.push((
            AutomationScriptEntry {
                name,
                folder: root.kind,
                language,
                bytes: metadata.len() as u32,
            },
            modified,
        ));
    }
    // Digest a stable order so an unchanged folder always produces one value.
    found.sort_by(|a, b| a.0.name.cmp(&b.0.name));
    for (entry, modified) in found {
        digest.update(entry.name.as_bytes());
        digest.update(entry.bytes.to_le_bytes());
        digest.update(modified.to_le_bytes());
        result.scripts.push(entry);
    }
    Ok(())
}

/// Reads one script's source on explicit request, without executing it.
pub(crate) fn read(
    api: &Api,
    params: AutomationScriptReadParams,
) -> Result<AutomationScriptSource, ApiError> {
    let name = Path::new(&params.name);
    if params.name.contains(['/', '\\'])
        || name.components().count() != 1
        || !matches!(name.components().next(), Some(Component::Normal(_)))
    {
        return Err(ApiError::invalid_params(
            "script name must be a direct filename",
        ));
    }
    let language = language(&params.name)
        .ok_or_else(|| ApiError::invalid_params("script must have a .js or .py extension"))?;
    let directory = root_path(api, params.doc, params.folder)?;
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
        folder: params.folder,
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

pub(crate) fn validate_label(label: &str) -> Result<String, ApiError> {
    let label = label.trim();
    if label.is_empty() || label.chars().count() > MAX_LABEL {
        return Err(ApiError::invalid_params(
            "a script transaction label must contain 1 to 120 characters",
        ));
    }
    Ok(label.to_owned())
}

/// Opens one script transaction so a whole run becomes one undo entry.
pub(crate) fn transaction_begin(
    api: &Api,
    params: AutomationTransactionParams,
) -> Result<AutomationTransaction, ApiError> {
    let label = validate_label(&params.label)?;
    let lifetime = params.timeout_ms.unwrap_or(DEFAULT_TRANSACTION_MS);
    if !(100..=MAX_TRANSACTION_MS).contains(&lifetime) {
        return Err(ApiError::invalid_params(format!(
            "a script transaction lifetime must be between 100 and {MAX_TRANSACTION_MS} ms"
        )));
    }
    // A new run supersedes any transaction an abandoned one left open.
    reap(api, true)?;
    let token = api
        .session
        .with_doc_mut(params.doc, |document| {
            document.begin_transaction(label, Duration::from_millis(lifetime.into()))
        })
        .map_err(ApiError::from)??;
    api.automation
        .transactions
        .lock()
        .map_err(|_| ApiError::internal("script transaction lock is poisoned"))?
        .push((params.doc, token.0));
    Ok(AutomationTransaction {
        doc: params.doc,
        token: token.0,
    })
}

/// Closes a script transaction, whether the run succeeded or failed.
pub(crate) fn transaction_end(
    api: &Api,
    params: AutomationTransactionEndParams,
) -> Result<AutomationTransactionEnd, ApiError> {
    forget(api, params.doc, params.token)?;
    api.session
        .with_doc_mut(params.doc, |document| {
            // A finished, superseded or never-opened token is reported, not an
            // error: the console always closes in a `finally` block.
            let recorded = document.end_transaction(EditToken(params.token));
            AutomationTransactionEnd {
                ended: recorded.is_some(),
                undo_label: recorded
                    .unwrap_or(false)
                    .then(|| document.undo_label().map(str::to_owned))
                    .flatten(),
            }
        })
        .map_err(ApiError::from)
}

pub(crate) fn run_python(
    api: &Api,
    params: AutomationPythonParams,
) -> Result<AutomationPythonResult, ApiError> {
    python::run(api, params)
}
