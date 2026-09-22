use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use fc_api::{Api, DocWarning};
use fc_doc::{DocId, FontOpener, Session};
use serde::{de::DeserializeOwned, Serialize};

use fc_cli::Error;

pub(crate) mod build;
pub(crate) mod check;
pub(crate) mod convert;
pub(crate) mod glyphs;
pub(crate) mod inspect;
pub(crate) mod shape;

pub(crate) struct OpenedDocument {
    pub(crate) session: Arc<Session>,
    pub(crate) api: Api,
    pub(crate) doc: DocId,
    pub(crate) warnings: Vec<DocWarning>,
}

/// Opens a binary font, a native document, or a UFO source into `session`.
fn open_in(session: &Session, path: &Path, face_index: Option<u32>) -> Result<DocId, Error> {
    if fc_formats::bitmap::is_bitmap_path(path) {
        let font = fc_formats::bitmap::read(path).map_err(|source| Error::OpenSource {
            path: path.to_path_buf(),
            source,
        })?;
        return Ok(session.open_font(font, Some(path.to_path_buf())));
    }
    let source = match source_extension(path).as_str() {
        "ufo" => Some(fc_formats::ufo::read_ufo(path)),
        "designspace" => Some(fc_formats::ufo::read_designspace(path)),
        "glyphs" => Some(fc_formats::glyphs::read_glyphs(path)),
        "sfd" => Some(fc_formats::sfd::read_sfd(path)),
        "pfa" | "pfb" => Some(fc_formats::type1::read_type1(path)),
        _ => None,
    };
    match source {
        Some(font) => Ok(session.open_font(
            font.map_err(|source| Error::OpenSource {
                path: path.to_path_buf(),
                source,
            })?,
            Some(path.to_path_buf()),
        )),
        None => session
            .open_path(path, face_index)
            .map_err(|source| Error::Open {
                path: path.to_path_buf(),
                source,
            }),
    }
}

fn source_extension(path: &Path) -> String {
    path.extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
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
    ) -> Result<Vec<fc_model::Font>, fc_doc::Error> {
        let outcome = fc_sfnt::read_path(
            path,
            &fc_sfnt::ReadOptions {
                font_index: face_index,
                keep_original_bytes: true,
                decode_bitmaps: true,
            },
        )
        .map_err(|error| fc_doc::Error::Sfnt(error.to_string()))?;
        let collection_warning = face_index
            .is_none()
            .then(|| {
                outcome
                    .fonts
                    .first()
                    .and_then(|font| font.extra.source.as_ref())
                    .and_then(|source| source.original_bytes.as_deref())
                    .and_then(|bytes| fc_sfnt::collection_warning(bytes.as_slice()))
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
    let doc = open_in(&session, path, face_index)?;
    let api = Api::new(Arc::clone(&session));
    let mut warnings = opener.take_warnings();
    warnings.extend(
        session
            .with_doc(doc, |document| {
                fc_formats::type1::read_warnings(&document.font)
            })?
            .into_iter()
            .map(|warning| DocWarning {
                code: warning.code.into(),
                message: warning.message,
            }),
    );
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
