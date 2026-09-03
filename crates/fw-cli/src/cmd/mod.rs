use std::{
    fs,
    path::Path,
    sync::{Arc, Mutex},
};

use fw_api::{Api, DocWarning};
use fw_doc::{DocId, FontOpener, Session};
use serde::{de::DeserializeOwned, Serialize};

use fw_cli::Error;

pub(crate) mod convert;
pub(crate) mod glyphs;
pub(crate) mod inspect;

pub(crate) struct OpenedDocument {
    pub(crate) session: Arc<Session>,
    pub(crate) api: Api,
    pub(crate) doc: DocId,
    pub(crate) warnings: Vec<DocWarning>,
}

#[derive(Default)]
struct CliFontOpener {
    warnings: Mutex<Vec<DocWarning>>,
}

impl CliFontOpener {
    fn take_warnings(&self) -> Vec<DocWarning> {
        let mut warnings = self
            .warnings
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        std::mem::take(&mut *warnings)
    }
}

impl FontOpener for CliFontOpener {
    fn open_path(
        &self,
        path: &Path,
        face_index: Option<u32>,
    ) -> Result<Vec<fw_model::Font>, fw_doc::Error> {
        let outcome = fw_sfnt::read_path(
            path,
            &fw_sfnt::ReadOptions {
                font_index: face_index,
                keep_original_bytes: true,
                decode_bitmaps: true,
            },
        )
        .map_err(|error| fw_doc::Error::Sfnt(error.to_string()))?;
        let mut warnings = self
            .warnings
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *warnings = outcome
            .warnings
            .into_iter()
            .map(|warning| DocWarning {
                code: warning.code.to_owned(),
                message: warning.message,
            })
            .collect();
        Ok(outcome.fonts)
    }
}

pub(crate) fn open_document(path: &Path, face_index: Option<u32>) -> Result<OpenedDocument, Error> {
    let collection_warning = collection_warning(path, face_index)?;
    let opener = Arc::new(CliFontOpener::default());
    let session = Arc::new(Session::with_opener(opener.clone()));
    let doc = session
        .open_path(path, face_index)
        .map_err(|source| Error::Open {
            path: path.to_path_buf(),
            source,
        })?;
    let api = Api::new(Arc::clone(&session));
    let mut warnings = opener.take_warnings();
    warnings.extend(collection_warning);
    Ok(OpenedDocument {
        session,
        api,
        doc,
        warnings,
    })
}

pub(crate) fn api_call<P, R>(api: &Api, method: &str, params: P) -> Result<R, Error>
where
    P: Serialize,
    R: DeserializeOwned,
{
    let params = serde_json::to_value(params)?;
    let result = api.call(method, params)?;
    serde_json::from_value(result).map_err(Error::from)
}

fn collection_warning(path: &Path, face_index: Option<u32>) -> Result<Option<DocWarning>, Error> {
    if face_index.is_some() || !is_collection_path(path) {
        return Ok(None);
    }
    let bytes = fs::read(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let faces = fw_sfnt::face_names(&bytes).map_err(|source| Error::Faces {
        path: path.to_path_buf(),
        source,
    })?;
    Ok((faces.len() > 1).then(|| DocWarning {
        code: "collection-face-opened".to_owned(),
        message: format!("collection has {} faces; opened face 0", faces.len()),
    }))
}

fn is_collection_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .is_some_and(|extension| matches!(extension.as_str(), "ttc" | "otc"))
}
