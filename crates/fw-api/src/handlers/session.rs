//! Session lifecycle handlers.

use std::{fs, path::Path};

use fw_doc::{DocId, Document};

use crate::{
    handlers::prefs, Api, ApiError, AxisSummary, CloseParams, DocSummary, EmptyParams, FacesParams,
    MasterSummary, NewParams, OpenParams, SaveParams,
};

pub(crate) fn open(api: &Api, params: OpenParams) -> Result<DocSummary, ApiError> {
    let path = Path::new(&params.path);
    let warnings = if params.face_index.is_none() && is_collection_path(path) {
        let face_count = face_names(path)?.len();
        if face_count > 1 {
            vec![format!("collection has {face_count} faces; opened face 0")]
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    let doc = api.session.open_path(path, params.face_index)?;
    api.warnings
        .lock()
        .map_err(|_| ApiError::internal("API warning cache lock is poisoned"))?
        .insert(doc, warnings);
    if let Err(error) = prefs::record_recent(api, params.path) {
        let _ = api.session.close(doc);
        if let Ok(mut warnings) = api.warnings.lock() {
            warnings.remove(&doc);
        }
        return Err(error);
    }
    summary_for(api, doc)
}

pub(crate) fn faces(_api: &Api, params: FacesParams) -> Result<Vec<String>, ApiError> {
    face_names(Path::new(&params.path))
}

pub(crate) fn new_document(api: &Api, params: NewParams) -> Result<DocSummary, ApiError> {
    let doc = api
        .session
        .new_document(&params.family, params.units_per_em);
    summary_for(api, doc)
}

pub(crate) fn close(api: &Api, params: CloseParams) -> Result<(), ApiError> {
    api.session.close(params.doc)?;
    api.sidebar_cache
        .lock()
        .map_err(|_| ApiError::internal("API sidebar cache lock is poisoned"))?
        .remove(&params.doc);
    api.warnings
        .lock()
        .map_err(|_| ApiError::internal("API warning cache lock is poisoned"))?
        .remove(&params.doc);
    Ok(())
}

pub(crate) fn list(api: &Api, _params: EmptyParams) -> Result<Vec<DocSummary>, ApiError> {
    api.session
        .ids()
        .into_iter()
        .map(|doc| summary_for(api, doc))
        .collect()
}

pub(crate) fn save(api: &Api, params: SaveParams) -> Result<DocSummary, ApiError> {
    let path = match params.path {
        Some(path) => path,
        None => {
            let current = api
                .session
                .with_doc(params.doc, |document| document.path.clone())?;
            current
                .ok_or_else(|| ApiError::invalid_params("an unsaved document requires a path"))?
                .to_string_lossy()
                .into_owned()
        }
    };
    api.session
        .with_doc_mut(params.doc, |document| document.save(Path::new(&path)))??;
    prefs::record_recent(api, path)?;
    summary_for(api, params.doc)
}

pub(crate) fn recent(api: &Api, _params: EmptyParams) -> Result<Vec<String>, ApiError> {
    prefs::recent(api)
}

pub(crate) fn summary_for(api: &Api, doc: DocId) -> Result<DocSummary, ApiError> {
    let warnings = api
        .warnings
        .lock()
        .map_err(|_| ApiError::internal("API warning cache lock is poisoned"))?
        .get(&doc)
        .cloned()
        .unwrap_or_default();
    api.session
        .with_doc(doc, |document| build_summary(doc, document, warnings))
        .map_err(ApiError::from)
}

fn build_summary(doc: DocId, document: &Document, warnings: Vec<String>) -> DocSummary {
    DocSummary {
        doc,
        path: document
            .path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned()),
        dirty: document.dirty,
        family_name: document.font.info.family_name.clone(),
        style_name: document.font.info.style_name.clone(),
        units_per_em: document.font.info.units_per_em,
        glyph_count: u32::try_from(document.font.glyphs.len()).unwrap_or(u32::MAX),
        masters: document
            .font
            .masters
            .iter()
            .map(|master| MasterSummary {
                id: master.id.clone(),
                name: master.name.clone(),
            })
            .collect(),
        axes: document
            .font
            .axes
            .iter()
            .map(|axis| AxisSummary {
                tag: axis.tag.to_string(),
                name: axis.name.clone(),
                min: axis.min,
                default: axis.default,
                max: axis.max,
                hidden: axis.hidden,
            })
            .collect(),
        undo_label: document.undo_label().map(str::to_owned),
        redo_label: document.redo_label().map(str::to_owned),
        format: document_format(document),
        warnings,
    }
}

fn document_format(document: &Document) -> String {
    if document.path.as_deref().is_some_and(is_fwf_path) {
        return "fwf".to_owned();
    }
    document
        .font
        .extra
        .source
        .as_ref()
        .map(|source| source.format.clone())
        .filter(|format| !format.is_empty())
        .unwrap_or_else(|| "fwf".to_owned())
}

fn face_names(path: &Path) -> Result<Vec<String>, ApiError> {
    let bytes = fs::read(path)
        .map_err(|error| ApiError::io(format!("failed to read `{}`: {error}", path.display())))?;
    fw_sfnt::face_names(&bytes).map_err(ApiError::from)
}

fn is_fwf_path(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    name.ends_with(".fwf") || name.ends_with(".fwf.gz")
}

fn is_collection_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .is_some_and(|extension| matches!(extension.as_str(), "ttc" | "otc"))
}
