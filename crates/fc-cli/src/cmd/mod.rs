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
pub(crate) mod collection;
pub(crate) mod convert;
pub(crate) mod glyphs;
pub(crate) mod inspect;
pub(crate) mod profiles;
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
        "svg" => Some(fc_formats::vector::read_svg(path)),
        "eps" => Some(fc_formats::vector::read_eps(path)),
        "afm" => Some(fc_formats::metrics::read_afm_path(path)),
        "pfm" => Some(fc_formats::metrics::pfm::read_pfm_path(path)),
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
        let options = fc_sfnt::ReadOptions {
            font_index: face_index,
            keep_original_bytes: true,
            decode_bitmaps: true,
        };
        let outcome = match source_extension(path).as_str() {
            "ttx" => fc_sfnt::ttx::read_path(path, &options),
            "cff" if face_index.is_none_or(|index| index == 0) => {
                fc_sfnt::cff_interchange::read_path(path)
            }
            "t42" | "p42" | "type42" => {
                let bytes = fc_formats::type42::read_path(path)
                    .map_err(|error| fc_doc::Error::Sfnt(error.to_string()))?;
                fc_sfnt::read_bytes(&bytes, &options)
            }
            _ => fc_sfnt::read_path(path, &options),
        }
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
    let doc = if matches!(
        source_extension(path).as_str(),
        "ttx" | "cff" | "t42" | "p42" | "type42"
    ) {
        // Session's binary path gate covers sfnt containers. These interchange
        // formats must reach the CLI decoder before attaching the decoded font.
        let font = opener
            .open_path(path, face_index)
            .and_then(|fonts| {
                fonts.into_iter().next().ok_or_else(|| {
                    fc_doc::Error::Internal(format!(
                        "font reader returned no faces for `{}`",
                        path.display()
                    ))
                })
            })
            .map_err(|source| Error::Open {
                path: path.to_path_buf(),
                source,
            })?;
        session.open_font(font, Some(path.to_path_buf()))
    } else {
        open_in(&session, path, face_index)?
    };
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
    if matches!(source_extension(path).as_str(), "afm" | "pfm") {
        warnings.push(DocWarning {
            code: "metrics-only-source".into(),
            message: "This file contains font metrics only; glyph outlines are empty.".into(),
        });
    }
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

/// Prints one document problem the way `fontcarver check` prints it.
pub(crate) fn print_problem(issue: &fc_api::ProblemIssue) {
    let mut parts = Vec::new();
    if let Some(name) = &issue.glyph_name {
        parts.push(name.to_string());
    }
    if let Some(master) = &issue.master {
        parts.push(format!("master={master}"));
    }
    if let Some(layer) = &issue.layer {
        parts.push(format!("layer={layer}"));
    }
    for (label, index) in [
        ("color", issue.color_layer),
        ("contour", issue.contour),
        ("point", issue.point),
    ] {
        if let Some(index) = index {
            parts.push(format!("{label}={index}"));
        }
    }
    let location = if parts.is_empty() {
        String::new()
    } else {
        format!(" {}", parts.join(" "))
    };
    println!(
        "{:?} [{}]{location}: {}",
        issue.severity, issue.code, issue.message
    );
}

/// Prints the same preflight report the export dialog shows; true when it has errors.
pub(crate) fn preflight_report(opened: &OpenedDocument) -> Result<bool, Error> {
    let report: fc_api::FindProblemsResult = api_call(
        &opened.api,
        "doc.find_problems",
        fc_api::FindProblemsParams {
            doc: opened.doc,
            limit: Some(200),
        },
    )?;
    println!("preflight:");
    for issue in &report.issues {
        print_problem(issue);
    }
    println!(
        "{} problems detected; {} shown{}",
        report.total,
        report.issues.len(),
        if report.truncated { " (truncated)" } else { "" }
    );
    Ok(report.has_errors)
}
