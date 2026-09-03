use std::{
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
                decode_bitmaps: false,
            },
        )
        .map_err(|error| fw_doc::Error::Sfnt(error.to_string()))?;
        let collection_warning = face_index
            .is_none()
            .then(|| {
                outcome
                    .fonts
                    .first()
                    .and_then(|font| font.extra.source.as_ref())
                    .and_then(|source| source.original_bytes.as_deref())
                    .and_then(|bytes| fw_sfnt::collection_warning(bytes.as_slice()))
            })
            .flatten();
        let mut warnings = self
            .warnings
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *warnings = collection_warning
            .into_iter()
            .chain(outcome.warnings)
            .map(|warning| DocWarning {
                code: warning.code.to_owned(),
                message: warning.message,
            })
            .collect();
        Ok(outcome.fonts)
    }
}

pub(crate) fn open_document(path: &Path, face_index: Option<u32>) -> Result<OpenedDocument, Error> {
    let opener = Arc::new(CliFontOpener::default());
    let session = Arc::new(Session::with_opener(opener.clone()));
    let doc = session
        .open_path(path, face_index)
        .map_err(|source| Error::Open {
            path: path.to_path_buf(),
            source,
        })?;
    let api = Api::new(Arc::clone(&session));
    let warnings = opener.take_warnings();
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
