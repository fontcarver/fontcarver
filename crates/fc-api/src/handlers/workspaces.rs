//! Named shell workspaces stored beside the document they belong to.
//!
//! A workspace is interface state, not font data, so it must not dirty the
//! document or take a place in its undo history. It is still part of the
//! project: a saved arrangement travels with a `.fcfp` directory through Git
//! and follows a `.fcf` file when it is copied. The records therefore live in a
//! deterministic JSON sidecar — `workspaces.json` inside a directory project,
//! `<name>.fcf.workspaces.json` next to a single-file document — written with
//! the same publish-then-rename discipline as the documents themselves.
//!
//! Documents without native storage (an imported `.ttf`, an unsaved document)
//! have no project to store anything in; the interface keeps their arrangements
//! in its own per-machine storage and this API reports `path: null` for them.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use fc_doc::DocId;

use crate::{Api, ApiError, DocParams, DocumentWorkspaces, NamedWorkspace, SetWorkspacesParams};

/// Most named workspaces one document may store.
const MAX_WORKSPACES: usize = 32;
/// Longest workspace name, in characters.
const MAX_NAME_CHARS: usize = 80;
/// Largest serialized sidecar, so a runaway interface cannot fill a disk.
const MAX_BYTES: usize = 256 * 1024;
/// File name used inside a `.fcfp` directory project.
const PROJECT_FILE: &str = "workspaces.json";
/// Suffix appended to a single-file document's own name.
const FILE_SUFFIX: &str = ".workspaces.json";

static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

pub(crate) fn get(api: &Api, params: DocParams) -> Result<DocumentWorkspaces, ApiError> {
    let Some(path) = sidecar_path(api, params.doc)? else {
        return Ok(DocumentWorkspaces {
            workspaces: Vec::new(),
            path: None,
        });
    };
    let workspaces = read_sidecar(&path)?;
    Ok(DocumentWorkspaces {
        workspaces,
        path: Some(path.to_string_lossy().into_owned()),
    })
}

pub(crate) fn set(api: &Api, params: SetWorkspacesParams) -> Result<DocumentWorkspaces, ApiError> {
    validate(&params.workspaces)?;
    let path = sidecar_path(api, params.doc)?.ok_or_else(|| {
        ApiError::invalid_params(
            "save the document as a FontCarver document or project before storing workspaces in it",
        )
    })?;
    let bytes = serialize(&params.workspaces)?;
    if params.workspaces.is_empty() {
        // An empty list means "store nothing", and leaving an empty file behind
        // would keep showing up in a project's Git status for no reason.
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(ApiError::io(format!(
                    "failed to remove `{}`: {error}",
                    path.display()
                )))
            }
        }
    } else {
        atomic_write(&path, &bytes)?;
    }
    Ok(DocumentWorkspaces {
        workspaces: params.workspaces,
        path: Some(path.to_string_lossy().into_owned()),
    })
}

/// Where this document's workspaces live, or `None` without native storage.
fn sidecar_path(api: &Api, doc: DocId) -> Result<Option<PathBuf>, ApiError> {
    let path = api
        .session
        .with_doc(doc, |document| document.path.clone())
        .map_err(ApiError::from)?;
    Ok(path.as_deref().and_then(sidecar_for))
}

/// The sidecar for a document path, or `None` when the format has no project.
pub(crate) fn sidecar_for(path: &Path) -> Option<PathBuf> {
    if fc_doc::fcf::is_project_path(path) {
        return Some(path.join(PROJECT_FILE));
    }
    if !fc_doc::fcf::is_native_path(path) {
        return None;
    }
    let name = path.file_name()?.to_str()?;
    Some(path.with_file_name(format!("{name}{FILE_SUFFIX}")))
}

fn validate(workspaces: &[NamedWorkspace]) -> Result<(), ApiError> {
    if workspaces.len() > MAX_WORKSPACES {
        return Err(ApiError::invalid_params(format!(
            "a document can store at most {MAX_WORKSPACES} named workspaces"
        )));
    }
    for (index, workspace) in workspaces.iter().enumerate() {
        let name = workspace.name.trim();
        if name.is_empty() || name.chars().count() > MAX_NAME_CHARS {
            return Err(ApiError::invalid_params(format!(
                "workspace name must be between 1 and {MAX_NAME_CHARS} characters"
            )));
        }
        if name != workspace.name {
            return Err(ApiError::invalid_params(
                "workspace names must not begin or end with whitespace",
            ));
        }
        if !workspace.layout.is_object() {
            return Err(ApiError::invalid_params(format!(
                "workspace `{name}` must carry a JSON object layout"
            )));
        }
        if workspaces[..index]
            .iter()
            .any(|earlier| earlier.name == workspace.name)
        {
            return Err(ApiError::invalid_params(format!(
                "workspace `{name}` is listed twice"
            )));
        }
    }
    Ok(())
}

/// Pretty JSON with a final newline, so a re-save produces no Git diff.
fn serialize(workspaces: &[NamedWorkspace]) -> Result<Vec<u8>, ApiError> {
    let mut bytes = serde_json::to_vec_pretty(&serde_json::json!({
        "format": "fontcarver-workspaces",
        "version": 1,
        "workspaces": workspaces,
    }))
    .map_err(|error| ApiError::internal(format!("failed to serialize workspaces: {error}")))?;
    bytes.push(b'\n');
    if bytes.len() > MAX_BYTES {
        return Err(ApiError::invalid_params(format!(
            "stored workspaces must stay below {} KiB; this request is {} KiB",
            MAX_BYTES / 1024,
            bytes.len() / 1024
        )));
    }
    Ok(bytes)
}

fn read_sidecar(path: &Path) -> Result<Vec<NamedWorkspace>, ApiError> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(ApiError::io(format!(
                "failed to read `{}`: {error}",
                path.display()
            )))
        }
    };
    if bytes.len() > MAX_BYTES {
        return Err(ApiError::parse(format!(
            "`{}` is larger than the {} KiB workspace limit",
            path.display(),
            MAX_BYTES / 1024
        )));
    }
    let document: Sidecar = serde_json::from_slice(&bytes).map_err(|error| {
        ApiError::parse(format!("failed to parse `{}`: {error}", path.display()))
    })?;
    if document.format != "fontcarver-workspaces" {
        return Err(ApiError::parse(format!(
            "`{}` is not a FontCarver workspace file",
            path.display()
        )));
    }
    if document.version > 1 {
        return Err(ApiError::parse(format!(
            "`{}` uses workspace format version {}, which this build cannot read",
            path.display(),
            document.version
        )));
    }
    validate(&document.workspaces)?;
    Ok(document.workspaces)
}

#[derive(serde::Deserialize)]
struct Sidecar {
    #[serde(default)]
    format: String,
    #[serde(default)]
    version: u32,
    #[serde(default)]
    workspaces: Vec<NamedWorkspace>,
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), ApiError> {
    let parent = path.parent().ok_or_else(|| {
        ApiError::invalid_params(format!("`{}` has no parent directory", path.display()))
    })?;
    let sequence = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
    let temporary = parent.join(format!(
        ".fontcarver-workspaces-{}-{sequence}.tmp",
        std::process::id()
    ));
    fs::write(&temporary, bytes).map_err(|error| {
        ApiError::io(format!(
            "failed to write `{}`: {error}",
            temporary.display()
        ))
    })?;
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(ApiError::io(format!(
            "failed to replace `{}`: {error}",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace(name: &str) -> NamedWorkspace {
        NamedWorkspace {
            name: name.to_owned(),
            layout: serde_json::json!({ "tabs": [], "panels": { "sidebar": true } }),
        }
    }

    #[test]
    fn sidecar_lives_inside_a_project_and_beside_a_file() {
        assert_eq!(
            sidecar_for(Path::new("/fonts/Family.fcfp")),
            Some(PathBuf::from("/fonts/Family.fcfp/workspaces.json"))
        );
        assert_eq!(
            sidecar_for(Path::new("/fonts/Family.fcf")),
            Some(PathBuf::from("/fonts/Family.fcf.workspaces.json"))
        );
        assert_eq!(
            sidecar_for(Path::new("/fonts/Family.fcf.gz")),
            Some(PathBuf::from("/fonts/Family.fcf.gz.workspaces.json"))
        );
    }

    #[test]
    fn documents_without_native_storage_have_no_sidecar() {
        for path in ["/fonts/Family.ttf", "/fonts/Family.ufo", "/fonts/Family"] {
            assert_eq!(sidecar_for(Path::new(path)), None, "{path}");
        }
    }

    #[test]
    fn a_saved_list_round_trips_byte_identically() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("workspaces.json");
        let workspaces = vec![workspace("Two docks"), workspace("Proofing")];

        let bytes = serialize(&workspaces).unwrap();
        atomic_write(&path, &bytes).unwrap();
        assert_eq!(read_sidecar(&path).unwrap(), workspaces);
        assert!(bytes.ends_with(b"}\n"));
        assert_eq!(serialize(&read_sidecar(&path).unwrap()).unwrap(), bytes);
    }

    #[test]
    fn a_missing_sidecar_reads_as_an_empty_list() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_sidecar(&dir.path().join("workspaces.json"))
            .unwrap()
            .is_empty());
    }

    #[test]
    fn corrupt_and_unsupported_sidecars_are_reported_rather_than_ignored() {
        let dir = tempfile::tempdir().unwrap();
        for contents in [
            "not json",
            r#"{"format":"other","version":1,"workspaces":[]}"#,
            r#"{"format":"fontcarver-workspaces","version":2,"workspaces":[]}"#,
            r#"{"format":"fontcarver-workspaces","version":1,"workspaces":[{"name":"","layout":{}}]}"#,
            r#"{"format":"fontcarver-workspaces","version":1,"workspaces":[{"name":"a","layout":3}]}"#,
        ] {
            let path = dir.path().join("workspaces.json");
            fs::write(&path, contents).unwrap();
            assert!(read_sidecar(&path).is_err(), "{contents}");
        }
    }

    #[test]
    fn invalid_lists_are_rejected_before_anything_is_written() {
        let long = "n".repeat(MAX_NAME_CHARS + 1);
        let cases = vec![
            vec![workspace("")],
            vec![workspace(" padded ")],
            vec![workspace(&long)],
            vec![workspace("same"), workspace("same")],
            (0..=MAX_WORKSPACES)
                .map(|index| workspace(&format!("w{index}")))
                .collect(),
            vec![NamedWorkspace {
                name: "bad".to_owned(),
                layout: serde_json::json!([1, 2, 3]),
            }],
        ];
        for workspaces in cases {
            assert!(validate(&workspaces).is_err(), "{workspaces:?}");
        }
        assert!(validate(&[workspace("good")]).is_ok());
    }

    #[test]
    fn an_oversized_list_is_refused() {
        let workspaces = vec![NamedWorkspace {
            name: "huge".to_owned(),
            layout: serde_json::json!({ "text": "x".repeat(MAX_BYTES) }),
        }];
        assert!(serialize(&workspaces).is_err());
    }
}
