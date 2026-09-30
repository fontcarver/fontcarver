//! Session lifecycle, import, and crash-recovery handlers.

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use fc_doc::{autosave, DocId, Document};

use crate::{
    handlers::prefs, Api, ApiError, AxisSummary, CloseParams, DocSummary, DocWarning, EmptyParams,
    FacesParams, MasterSummary, NewParams, OpenParams, RecoveryEntry, RecoveryParams, SaveParams,
};

pub(crate) fn open(api: &Api, params: OpenParams) -> Result<DocSummary, ApiError> {
    let path = Path::new(&params.path);
    let (doc, warnings) = if is_fcf_path(path) {
        (api.session.open_path(path, params.face_index)?, Vec::new())
    } else if fc_formats::bitmap::is_bitmap_path(path) {
        (
            api.session
                .open_font(fc_formats::bitmap::read(path)?, Some(path.to_path_buf())),
            Vec::new(),
        )
    } else if ["ttx", "cff", "t42", "p42", "type42"]
        .iter()
        .any(|extension| has_extension(path, extension))
    {
        open_binary_source(api, path, params.face_index)?
    } else if is_binary_font_path(path) {
        open_binary(api, path, params.face_index)?
    } else if is_ufo_path(path) {
        (open_source(api, path, SourceKind::Ufo)?, Vec::new())
    } else if is_designspace_path(path) {
        (open_source(api, path, SourceKind::Designspace)?, Vec::new())
    } else if has_extension(path, "glyphs") {
        (open_source(api, path, SourceKind::Glyphs)?, Vec::new())
    } else if has_extension(path, "sfd") {
        (open_source(api, path, SourceKind::Sfd)?, Vec::new())
    } else if has_extension(path, "svg") {
        (open_source(api, path, SourceKind::Svg)?, Vec::new())
    } else if has_extension(path, "eps") {
        (open_source(api, path, SourceKind::Eps)?, Vec::new())
    } else if has_extension(path, "afm") {
        (open_source(api, path, SourceKind::Afm)?, metrics_warning())
    } else if has_extension(path, "pfm") {
        (open_source(api, path, SourceKind::Pfm)?, metrics_warning())
    } else if has_extension(path, "pfa") || has_extension(path, "pfb") {
        (open_source(api, path, SourceKind::Type1)?, Vec::new())
    } else {
        (api.session.open_path(path, params.face_index)?, Vec::new())
    };
    api.warnings
        .lock()
        .map_err(|_| ApiError::internal("API warning cache lock is poisoned"))?
        .insert(doc, warnings);
    if let Err(error) = prefs::record_recent(api, params.path) {
        tracing::warn!(%error, doc = doc.0, "failed to record recently opened font");
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
    api.shaper.forget(params.doc.0);
    api.layout_cache
        .lock()
        .map_err(|_| ApiError::internal("layout cache lock is poisoned"))?
        .remove(&params.doc);
    api.conversion_tokens
        .lock()
        .map_err(|_| ApiError::internal("layout conversion token lock is poisoned"))?
        .remove_doc(params.doc);
    api.sidebar_cache
        .lock()
        .map_err(|_| ApiError::internal("API sidebar cache lock is poisoned"))?
        .remove(&params.doc);
    api.warnings
        .lock()
        .map_err(|_| ApiError::internal("API warning cache lock is poisoned"))?
        .remove(&params.doc);
    api.recovered
        .lock()
        .map_err(|_| ApiError::internal("API recovery marker lock is poisoned"))?
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
        Some(path) if is_fcf_path(Path::new(&path)) => path,
        Some(_) => return Err(ApiError::invalid_params(
            "save target must end with .fcf, .fcf.gz, or .fcfp; use export for binary font formats",
        )),
        None => {
            let current = api
                .session
                .with_doc(params.doc, |document| document.path.clone())?;
            match current {
                Some(path) if is_fcf_path(&path) => path.to_string_lossy().into_owned(),
                Some(_) => {
                    return Err(ApiError::invalid_params(
                        "imported fonts need an explicit .fcf or .fcfp path",
                    ))
                }
                None => {
                    return Err(ApiError::invalid_params(
                        "an unsaved document requires a path",
                    ))
                }
            }
        }
    };
    api.session
        .with_doc_mut(params.doc, |document| document.save(Path::new(&path)))??;
    // The document now *is* the file on disk, so it is no longer a recovery.
    api.recovered
        .lock()
        .map_err(|_| ApiError::internal("API recovery marker lock is poisoned"))?
        .remove(&params.doc);
    prefs::record_recent(api, path)?;
    summary_for(api, params.doc)
}

pub(crate) fn recent(api: &Api, _params: EmptyParams) -> Result<Vec<String>, ApiError> {
    prefs::recent(api)
}

pub(crate) fn recover_list(
    api: &Api,
    _params: EmptyParams,
) -> Result<Vec<RecoveryEntry>, ApiError> {
    Ok(autosave::list_recoverable(&api.autosave_dir()?)
        .into_iter()
        .map(|entry| RecoveryEntry {
            doc_path: entry
                .doc_path
                .map(|path| path.to_string_lossy().into_owned()),
            autosave_path: entry.autosave_path.to_string_lossy().into_owned(),
            saved_at_ms: epoch_millis(entry.saved_at),
        })
        .collect())
}

pub(crate) fn recover(api: &Api, params: RecoveryParams) -> Result<DocSummary, ApiError> {
    let autosave_path = PathBuf::from(&params.autosave_path);
    let entry = recovery_entry(api, &autosave_path)?;
    let font = fc_doc::fcf::read(&autosave_path)?;
    let recovered_from = entry
        .doc_path
        .as_ref()
        .map(|path| path.to_string_lossy().into_owned());
    let doc = api.session.open_font(font, entry.doc_path);
    // The snapshot is by definition newer than the document on disk.
    api.session
        .with_doc_mut(doc, |document| document.dirty = true)?;
    api.warnings
        .lock()
        .map_err(|_| ApiError::internal("API warning cache lock is poisoned"))?
        .insert(doc, Vec::new());
    if let Some(recovered_from) = recovered_from {
        api.recovered
            .lock()
            .map_err(|_| ApiError::internal("API recovery marker lock is poisoned"))?
            .insert(doc, recovered_from);
    }
    summary_for(api, doc)
}

pub(crate) fn discard_recovery(api: &Api, params: RecoveryParams) -> Result<(), ApiError> {
    let autosave_path = PathBuf::from(&params.autosave_path);
    recovery_entry(api, &autosave_path)?;
    fs::remove_file(&autosave_path).map_err(|error| {
        ApiError::io(format!(
            "failed to delete `{}`: {error}",
            autosave_path.display()
        ))
    })
}

/// Looks a snapshot up in the recovery directory, rejecting foreign paths.
fn recovery_entry(api: &Api, path: &Path) -> Result<autosave::RecoverableEntry, ApiError> {
    autosave::list_recoverable(&api.autosave_dir()?)
        .into_iter()
        .find(|entry| entry.autosave_path == path)
        .ok_or_else(|| {
            ApiError::not_found(format!(
                "`{}` is not a recoverable document",
                path.display()
            ))
        })
}

fn epoch_millis(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|since| u64::try_from(since.as_millis()).ok())
        .unwrap_or_default()
}

#[derive(Clone, Copy, Debug)]
enum SourceKind {
    Ufo,
    Designspace,
    Glyphs,
    Sfd,
    Type1,
    Svg,
    Eps,
    Afm,
    Pfm,
}

fn open_source(api: &Api, path: &Path, kind: SourceKind) -> Result<DocId, ApiError> {
    let font = match kind {
        SourceKind::Ufo => fc_formats::ufo::read_ufo(path),
        SourceKind::Designspace => fc_formats::ufo::read_designspace(path),
        SourceKind::Glyphs => fc_formats::glyphs::read_glyphs(path),
        SourceKind::Sfd => fc_formats::sfd::read_sfd(path),
        SourceKind::Type1 => fc_formats::type1::read_type1(path),
        SourceKind::Svg => fc_formats::vector::read_svg(path),
        SourceKind::Eps => fc_formats::vector::read_eps(path),
        SourceKind::Afm => fc_formats::metrics::read_afm_path(path),
        SourceKind::Pfm => fc_formats::metrics::pfm::read_pfm_path(path),
    }?;
    Ok(api.session.open_font(font, Some(path.to_path_buf())))
}

fn metrics_warning() -> Vec<DocWarning> {
    vec![DocWarning {
        code: "metrics-only-source".into(),
        message: "This file contains font metrics only; glyph outlines are empty.".into(),
    }]
}

fn open_binary_source(
    api: &Api,
    path: &Path,
    face_index: Option<u32>,
) -> Result<(DocId, Vec<DocWarning>), ApiError> {
    let options = fc_sfnt::ReadOptions {
        font_index: face_index,
        ..fc_sfnt::ReadOptions::default()
    };
    let format;
    let mut outcome = if has_extension(path, "ttx") {
        format = "ttx";
        fc_sfnt::ttx::read_path(path, &options)?
    } else if has_extension(path, "cff") {
        format = "cff";
        if face_index.is_some_and(|index| index != 0) {
            return Err(ApiError::invalid_params("standalone CFF contains one face"));
        }
        fc_sfnt::cff_interchange::read_path(path)?
    } else {
        format = "type42";
        let bytes = fc_formats::type42::read_path(path)?;
        fc_sfnt::read_bytes(&bytes, &options)?
    };
    let mut font = outcome
        .fonts
        .pop()
        .ok_or_else(|| ApiError::internal("source reader returned no font"))?;
    if let Some(source) = &mut font.extra.source {
        source.path = Some(path.to_string_lossy().into_owned());
        source.format = format.into();
    }
    let warnings = outcome
        .warnings
        .into_iter()
        .map(|warning| DocWarning {
            code: warning.code.into(),
            message: warning.message,
        })
        .collect();
    Ok((
        api.session.open_font(font, Some(path.to_path_buf())),
        warnings,
    ))
}

pub(crate) fn summary_for(api: &Api, doc: DocId) -> Result<DocSummary, ApiError> {
    let warnings = api
        .warnings
        .lock()
        .map_err(|_| ApiError::internal("API warning cache lock is poisoned"))?
        .get(&doc)
        .cloned()
        .unwrap_or_default();
    let recovered_from = api
        .recovered
        .lock()
        .map_err(|_| ApiError::internal("API recovery marker lock is poisoned"))?
        .get(&doc)
        .cloned();
    api.session
        .with_doc(doc, |document| {
            build_summary(doc, document, warnings, recovered_from)
        })
        .map_err(ApiError::from)
}

fn build_summary(
    doc: DocId,
    document: &Document,
    mut warnings: Vec<DocWarning>,
    recovered_from: Option<String>,
) -> DocSummary {
    warnings.extend(
        fc_formats::type1::read_warnings(&document.font)
            .into_iter()
            .map(|warning| DocWarning {
                code: warning.code.into(),
                message: warning.message,
            }),
    );
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
        recovered_from,
    }
}

fn document_format(document: &Document) -> String {
    if document.path.as_deref().is_some_and(is_fcf_path) {
        return if document
            .path
            .as_deref()
            .is_some_and(fc_doc::fcf::is_project_path)
        {
            "fcfp"
        } else {
            "fcf"
        }
        .to_owned();
    }
    if let Some(path) = document
        .path
        .as_deref()
        .filter(|p| fc_formats::bitmap::is_bitmap_path(p))
    {
        let inner = if path.extension().is_some_and(|e| e == "gz") {
            path.with_extension("")
        } else {
            path.to_path_buf()
        };
        return inner
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bitmap")
            .to_owned();
    }
    document
        .font
        .extra
        .source
        .as_ref()
        .map(|source| source.format.clone())
        .filter(|format| !format.is_empty())
        .unwrap_or_else(|| "fcf".to_owned())
}

fn face_names(path: &Path) -> Result<Vec<String>, ApiError> {
    let bytes = fs::read(path)
        .map_err(|error| ApiError::io(format!("failed to read `{}`: {error}", path.display())))?;
    fc_sfnt::face_names(&bytes).map_err(ApiError::from)
}

fn open_binary(
    api: &Api,
    path: &Path,
    face_index: Option<u32>,
) -> Result<(DocId, Vec<DocWarning>), ApiError> {
    let bytes = fs::read(path)
        .map_err(|error| ApiError::io(format!("failed to read `{}`: {error}", path.display())))?;
    let mut warnings = face_index
        .is_none()
        .then(|| fc_sfnt::collection_warning(&bytes))
        .flatten()
        .map(|warning| DocWarning {
            code: warning.code.to_owned(),
            message: warning.message,
        })
        .into_iter()
        .collect::<Vec<_>>();
    let options = fc_sfnt::ReadOptions {
        font_index: face_index,
        keep_original_bytes: true,
        decode_bitmaps: true,
    };
    let mut outcome = fc_sfnt::read_bytes(&bytes, &options)?;
    if !outcome.warnings.is_empty() {
        tracing::warn!(
            warning_count = outcome.warnings.len(),
            path = %path.display(),
            "warnings while opening binary font"
        );
    }
    warnings.extend(outcome.warnings.drain(..).map(|warning| DocWarning {
        code: warning.code.to_owned(),
        message: warning.message,
    }));
    let mut font = outcome
        .fonts
        .drain(..)
        .next()
        .ok_or_else(|| ApiError::internal("font reader returned no faces"))?;
    if let Some(source) = &mut font.extra.source {
        source.path = Some(path.to_string_lossy().into_owned());
    }
    let doc = api.session.open_font(font, Some(path.to_path_buf()));
    Ok((doc, warnings))
}

fn is_fcf_path(path: &Path) -> bool {
    fc_doc::fcf::is_native_path(path)
}

fn is_ufo_path(path: &Path) -> bool {
    has_extension(path, "ufo")
}

fn is_designspace_path(path: &Path) -> bool {
    has_extension(path, "designspace")
}

fn has_extension(path: &Path, expected: &str) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(expected))
}

fn is_binary_font_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .is_some_and(|extension| {
            matches!(
                extension.as_str(),
                "otb" | "ttf" | "otf" | "ttc" | "otc" | "woff" | "woff2"
            )
        })
}
