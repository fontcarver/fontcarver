//! Document query and mutation handlers.

use std::collections::{BTreeMap, HashMap};

use fw_doc::EditToken;
use fw_model::{Font, SourceProvenance, Tag};
use read_fonts::FileRef;

use crate::{
    handlers::session::summary_for, Api, ApiError, ApplyParams, DocParams, DocSummary,
    EditApplyParams, EditBeginParams, EditEndParams, EditTokenId, FontInfoView, SidebarCounts,
    TableInfo,
};

pub(crate) fn summary(api: &Api, params: DocParams) -> Result<DocSummary, ApiError> {
    summary_for(api, params.doc)
}

pub(crate) fn font_info(api: &Api, params: DocParams) -> Result<FontInfoView, ApiError> {
    api.session
        .with_doc(params.doc, |document| FontInfoView {
            info: document.font.info.clone(),
            masters: document.font.masters.clone(),
            tables: table_inventory(&document.font),
            instances: document.font.instances.clone(),
        })
        .map_err(ApiError::from)
}

fn table_inventory(font: &Font) -> Vec<TableInfo> {
    let source_lengths = font
        .extra
        .source
        .as_ref()
        .map(source_table_lengths)
        .unwrap_or_default();
    let mut tables = font
        .extra
        .raw_tables
        .iter()
        .map(|(tag, bytes)| {
            (
                *tag,
                TableInfo {
                    tag: tag.to_string(),
                    len: u32::try_from(bytes.len()).unwrap_or(u32::MAX),
                    preserved: true,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    if let Some(source) = &font.extra.source {
        for tag in &source.table_tags {
            tables.entry(*tag).or_insert_with(|| TableInfo {
                tag: tag.to_string(),
                len: source_lengths.get(tag).copied().unwrap_or_default(),
                preserved: false,
            });
        }
    }
    tables.into_values().collect()
}

fn source_table_lengths(source: &SourceProvenance) -> BTreeMap<Tag, u32> {
    if source.table_tags.is_empty() {
        return BTreeMap::new();
    }
    let Some(bytes) = source.original_bytes.as_deref() else {
        return BTreeMap::new();
    };
    let Ok(file) = FileRef::new(bytes.as_slice()) else {
        return BTreeMap::new();
    };
    file.fonts()
        .filter_map(Result::ok)
        .find(|face| {
            face.table_directory()
                .table_records()
                .iter()
                .map(|record| Tag(record.tag().to_be_bytes()))
                .eq(source.table_tags.iter().copied())
        })
        .map(|face| {
            face.table_directory()
                .table_records()
                .iter()
                .map(|record| (Tag(record.tag().to_be_bytes()), record.length()))
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn sidebar(api: &Api, params: DocParams) -> Result<SidebarCounts, ApiError> {
    api.session
        .with_doc(params.doc, |document| {
            let mut cache = api
                .sidebar_cache
                .lock()
                .map_err(|_| ApiError::internal("API sidebar cache lock is poisoned"))?;
            if let Some((generation, counts)) = cache.get(&params.doc) {
                if *generation == document.generation {
                    return Ok(counts.clone());
                }
            }
            let counts = compute_sidebar(document);
            cache.insert(params.doc, (document.generation, counts.clone()));
            Ok(counts)
        })
        .map_err(ApiError::from)?
}

pub(crate) fn apply(api: &Api, params: ApplyParams) -> Result<fw_doc::Change, ApiError> {
    api.session
        .with_doc_mut(params.doc, |document| {
            document.apply(params.command, params.label)
        })
        .map_err(ApiError::from)?
        .map_err(ApiError::from)
}

pub(crate) fn edit_begin(api: &Api, params: EditBeginParams) -> Result<EditTokenId, ApiError> {
    api.session
        .with_doc_mut(params.doc, |document| {
            let token = document.begin_edit(params.label);
            EditTokenId(token.0)
        })
        .map_err(ApiError::from)
}

pub(crate) fn edit_apply(api: &Api, params: EditApplyParams) -> Result<fw_doc::Change, ApiError> {
    api.session
        .with_doc_mut(params.doc, |document| {
            document.apply_in_edit(&EditToken(params.token.0), params.command)
        })
        .map_err(ApiError::from)?
        .map_err(ApiError::from)
}

pub(crate) fn edit_end(api: &Api, params: EditEndParams) -> Result<(), ApiError> {
    api.session
        .with_doc_mut(params.doc, |document| {
            document.end_edit(EditToken(params.token.0));
        })
        .map_err(ApiError::from)
}

pub(crate) fn undo(api: &Api, params: DocParams) -> Result<Option<fw_doc::Change>, ApiError> {
    api.session
        .with_doc_mut(params.doc, |document| document.undo())
        .map_err(ApiError::from)?
        .map_err(ApiError::from)
}

pub(crate) fn redo(api: &Api, params: DocParams) -> Result<Option<fw_doc::Change>, ApiError> {
    api.session
        .with_doc_mut(params.doc, |document| document.redo())
        .map_err(ApiError::from)?
        .map_err(ApiError::from)
}

fn compute_sidebar(document: &fw_doc::Document) -> SidebarCounts {
    let mut categories = HashMap::<String, u32>::new();
    let mut scripts = HashMap::<String, u32>::new();
    let mut blocks = HashMap::<String, u32>::new();
    let mut color_labels = HashMap::<u8, u32>::new();
    let mut unencoded = 0_u32;

    for glyph in document.font.glyphs.values() {
        let (category, script, block) = match glyph.unicodes.first().copied() {
            Some(unicode) => (
                super::glyphs::category_name(fw_glyphdata::category(unicode)).to_owned(),
                fw_glyphdata::script(unicode).to_owned(),
                fw_glyphdata::block_name(unicode).to_owned(),
            ),
            None => {
                unencoded = unencoded.saturating_add(1);
                (
                    "Unencoded".to_owned(),
                    "Zzzz".to_owned(),
                    "Unencoded".to_owned(),
                )
            }
        };
        increment(&mut categories, category);
        increment(&mut scripts, script);
        increment(&mut blocks, block);
        if let Some(label) = glyph.color_label {
            *color_labels.entry(label).or_default() += 1;
        }
    }

    SidebarCounts {
        categories: ordered_categories(&categories),
        scripts: ordered_named_counts(fw_glyphdata::all_scripts(), &scripts, &["Zzzz"]),
        blocks: ordered_named_counts(fw_glyphdata::all_block_names(), &blocks, &["Unencoded"]),
        unencoded,
        total: u32::try_from(document.font.glyphs.len()).unwrap_or(u32::MAX),
        color_labels: sorted_counts(color_labels),
    }
}

fn increment(counts: &mut HashMap<String, u32>, key: String) {
    *counts.entry(key).or_default() += 1;
}

fn ordered_categories(counts: &HashMap<String, u32>) -> Vec<(String, u32)> {
    const ORDER: &[&str] = &[
        "Letter",
        "Number",
        "Punctuation",
        "Symbol",
        "Separator",
        "Mark",
        "Format",
        "Private",
        "Other",
        "Unencoded",
    ];
    ordered_named_counts(ORDER, counts, &[])
}

fn ordered_named_counts(
    order: &[&str],
    counts: &HashMap<String, u32>,
    extras: &[&str],
) -> Vec<(String, u32)> {
    let mut result = Vec::new();
    for name in order.iter().chain(extras) {
        if result.iter().any(|(existing, _)| existing == name) {
            continue;
        }
        if let Some(count) = counts.get(*name) {
            result.push(((*name).to_owned(), *count));
        }
    }
    let mut unknown: Vec<_> = counts
        .iter()
        .filter(|(name, _)| !result.iter().any(|(existing, _)| existing == *name))
        .map(|(name, count)| (name.clone(), *count))
        .collect();
    unknown.sort_by(|left, right| left.0.cmp(&right.0));
    result.extend(unknown);
    result
}

fn sorted_counts<K: Ord>(counts: HashMap<K, u32>) -> Vec<(K, u32)> {
    let mut counts: Vec<_> = counts.into_iter().collect();
    counts.sort_by(|left, right| left.0.cmp(&right.0));
    counts
}
