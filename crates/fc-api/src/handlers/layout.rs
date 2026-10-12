//! Read-only imported layout inspection, outside the document session lock.

use std::sync::Arc;

use fc_doc::Document;
use fc_layout::LayoutImport;
use fc_model::FeaturesSource;

use crate::{
    Api, ApiError, DocId, LayoutConversionBlocker, LayoutInspectParams, LayoutInspection,
    LayoutLookupInfo, LayoutRuleGlyph, LayoutRuleGlyphs, LayoutRuleGlyphsParams, LayoutRuleInfo,
    LayoutRules, LayoutRulesParams, LayoutTableInfo,
};

const MAX_PAGE: u32 = 200;
const CACHE_DOCUMENTS: usize = 8;

struct Inspection {
    import: Arc<LayoutImport>,
    generation: u64,
    compile_revision: u64,
    draft_revision: u64,
}

pub(crate) fn inspect(
    api: &Api,
    params: LayoutInspectParams,
) -> Result<LayoutInspection, ApiError> {
    let snapshot = prepare(
        api,
        params.doc,
        params.generation,
        params.offset,
        params.limit,
    )?;
    let import = &snapshot.import;
    let items = import
        .lookups
        .iter()
        .skip(params.offset as usize)
        .take(params.limit as usize)
        .map(|lookup| LayoutLookupInfo {
            table: lookup.table.clone(),
            index: lookup.index,
            kind: lookup.kind,
            flags: lookup.flags,
            referenced_lookups: lookup
                .referenced_lookups
                .iter()
                .take(MAX_PAGE as usize)
                .copied()
                .collect(),
            references_truncated: lookup.referenced_lookups.len() > MAX_PAGE as usize,
            rule_count: count(lookup.rules.len()),
        })
        .collect();
    api.session
        .with_doc(params.doc, |document| {
            recheck(document, &snapshot)?;
            Ok(LayoutInspection {
                doc: params.doc,
                generation: snapshot.generation,
                compile_revision: snapshot.compile_revision,
                draft_revision: snapshot.draft_revision,
                authority: document.font.features.source_of_truth,
                original_layout_available: document.font.layout_conversion.is_some(),
                fingerprint: import.fingerprint.clone(),
                tables: import
                    .tables
                    .iter()
                    .map(|table| LayoutTableInfo {
                        tag: table.tag.clone(),
                        byte_length: count(table.byte_length),
                        lookup_count: count(table.lookup_count),
                    })
                    .collect(),
                blockers: import
                    .blockers
                    .iter()
                    .take(MAX_PAGE as usize)
                    .map(|blocker| LayoutConversionBlocker {
                        table: blocker.table.clone(),
                        lookup: blocker.lookup,
                        reason: blocker.reason.chars().take(4096).collect(),
                    })
                    .collect(),
                blockers_truncated: import.blockers.len() > MAX_PAGE as usize,
                can_convert: document.font.features.source_of_truth
                    == FeaturesSource::ImportedTables
                    && import.can_convert(),
                total: count(import.lookups.len()),
                items,
                next_offset: next_offset(import.lookups.len(), params.offset, params.limit),
            })
        })
        .map_err(ApiError::from)?
}

pub(crate) fn rules(api: &Api, params: LayoutRulesParams) -> Result<LayoutRules, ApiError> {
    let snapshot = prepare(
        api,
        params.doc,
        params.generation,
        params.offset,
        params.limit,
    )?;
    let import = &snapshot.import;
    let lookup = import
        .lookups
        .iter()
        .find(|lookup| lookup.table == params.table && lookup.index == params.lookup)
        .ok_or_else(|| ApiError::not_found("imported layout lookup does not exist"))?;
    api.session
        .with_doc(params.doc, |document| {
            recheck(document, &snapshot)?;
            let items = lookup
                .rules
                .iter()
                .skip(params.offset as usize)
                .take(params.limit as usize)
                .map(|rule| LayoutRuleInfo {
                    text: rule.text.chars().take(4096).collect(),
                    glyphs: rule
                        .glyphs
                        .iter()
                        .take(64)
                        .map(|name| LayoutRuleGlyph {
                            name: name.to_string(),
                            gid: document.glyph_id(name.as_ref()),
                        })
                        .collect(),
                    truncated: rule.truncated
                        || rule.glyphs.len() > 64
                        || rule.text.chars().count() > 4096,
                })
                .collect();
            Ok(LayoutRules {
                doc: params.doc,
                generation: snapshot.generation,
                compile_revision: snapshot.compile_revision,
                draft_revision: snapshot.draft_revision,
                fingerprint: import.fingerprint.clone(),
                table: params.table,
                lookup: params.lookup,
                total: count(lookup.rules.len()),
                items,
                next_offset: next_offset(lookup.rules.len(), params.offset, params.limit),
            })
        })
        .map_err(ApiError::from)?
}

pub(crate) fn rule_glyphs(
    api: &Api,
    params: LayoutRuleGlyphsParams,
) -> Result<LayoutRuleGlyphs, ApiError> {
    let snapshot = prepare(
        api,
        params.doc,
        params.generation,
        params.offset,
        params.limit,
    )?;
    let import = &snapshot.import;
    let names = import
        .rule_glyphs(&params.table, params.lookup, params.rule as usize)
        .ok_or_else(|| ApiError::not_found("imported layout rule does not exist"))?;
    api.session
        .with_doc(params.doc, |document| {
            recheck(document, &snapshot)?;
            let items = names
                .iter()
                .skip(params.offset as usize)
                .take(params.limit as usize)
                .map(|name| LayoutRuleGlyph {
                    name: name.to_string(),
                    gid: document.glyph_id(name.as_ref()),
                })
                .collect();
            Ok(LayoutRuleGlyphs {
                doc: params.doc,
                generation: snapshot.generation,
                compile_revision: snapshot.compile_revision,
                draft_revision: snapshot.draft_revision,
                fingerprint: import.fingerprint.clone(),
                table: params.table,
                lookup: params.lookup,
                rule: params.rule,
                total: count(names.len()),
                items,
                next_offset: next_offset(names.len(), params.offset, params.limit),
            })
        })
        .map_err(ApiError::from)?
}

fn prepare(
    api: &Api,
    doc: DocId,
    expected: Option<u64>,
    offset: u32,
    limit: u32,
) -> Result<Inspection, ApiError> {
    if !(1..=MAX_PAGE).contains(&limit) {
        return Err(ApiError::invalid_params(
            "layout page limit must be between 1 and 200",
        ));
    }
    if offset != 0 && expected.is_none() {
        return Err(ApiError::invalid_params(
            "layout continuation pages require a generation",
        ));
    }
    let (generation, compile_revision, draft_revision, cached, font) = api
        .session
        .with_doc(doc, |document| {
            if expected.is_some_and(|generation| generation != document.generation) {
                return Err(stale());
            }
            let cached = api
                .layout_cache
                .lock()
                .map_err(|_| ApiError::internal("layout cache lock is poisoned"))?
                .get(&doc)
                .filter(|(revision, _)| *revision == document.compile_revision())
                .map(|(_, import)| Arc::clone(import));
            let font = cached.is_none().then(|| document.font.clone());
            Ok((
                document.generation,
                document.compile_revision(),
                document.font.feature_revision,
                cached,
                font,
            ))
        })
        .map_err(ApiError::from)??;
    let import = match (cached, font) {
        (Some(import), _) => import,
        (_, Some(font)) => Arc::new(LayoutImport::from_font(&font)),
        _ => return Err(ApiError::internal("layout snapshot is missing")),
    };
    let snapshot = Inspection {
        import,
        generation,
        compile_revision,
        draft_revision,
    };
    api.session
        .with_doc(doc, |document| {
            recheck(document, &snapshot)?;
            let mut cache = api
                .layout_cache
                .lock()
                .map_err(|_| ApiError::internal("layout cache lock is poisoned"))?;
            if cache.len() >= CACHE_DOCUMENTS && !cache.contains_key(&doc) {
                cache.clear();
            }
            cache.insert(doc, (compile_revision, Arc::clone(&snapshot.import)));
            Ok::<_, ApiError>(())
        })
        .map_err(ApiError::from)??;
    Ok(snapshot)
}

fn recheck(document: &Document, snapshot: &Inspection) -> Result<(), ApiError> {
    if document.generation != snapshot.generation
        || document.compile_revision() != snapshot.compile_revision
        || document.font.feature_revision != snapshot.draft_revision
    {
        return Err(stale());
    }
    Ok(())
}

fn stale() -> ApiError {
    ApiError {
        code: "stale_layout".into(),
        message: "layout inputs changed; restart inspection".into(),
    }
}

fn count(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}
fn next_offset(total: usize, offset: u32, limit: u32) -> Option<u32> {
    let next = offset.saturating_add(limit);
    ((next as usize) < total).then_some(next)
}
