//! Bounded read-only comparisons of evaluated drawings and family metadata.
use std::collections::{BTreeMap, BTreeSet};

use fc_doc::Document;
use fc_model::{
    Axis, DesignLocation, FeatureGenerationSettings, Features, Font, GlyphId, KernSide, Kerning,
    KerningGroups, Layer, MasterId, MasterMetrics, PointKind,
};
use kurbo::{BezPath, PathEl, Rect, Shape};
use sha2::{Digest, Sha256};

use crate::{
    Api, ApiError, FontCompareCompatibility, FontCompareCounts, FontCompareFamilyMember,
    FontCompareFamilyParams, FontCompareFamilyResult, FontCompareFieldDiff, FontCompareGlyph,
    FontCompareGlyphParams, FontCompareGlyphResult, FontCompareKerning, FontCompareLayout,
    FontCompareOutline, FontComparePairDiff, FontCompareParams, FontCompareResult,
    FontCompareStatus, FontReferenceMaster, FontReferenceParams, FontReferenceResult, OpenParams,
};

const MAX_GLYPHS: usize = 100_000;
const MAX_ELEMENTS: usize = 16_000_000;
const MAX_GLYPH_ELEMENTS: usize = 100_000;
const MAX_GLYPH_WORK: usize = 1_000_000;
const MAX_KERN_RECORDS: usize = 200_000;
const MAX_METADATA_FIELDS: usize = 128;
const MAX_LIST_ITEMS: usize = 64;
const MAX_PAIR_EXAMPLES: usize = 32;
const MAX_FAMILY_MEMBERS: usize = 64;

type WorkCache = BTreeMap<String, (usize, Option<String>)>;

// Empty composite DAGs can consume exponential visits without path elements.
// Memoized expanded costs reject them before entering the outline evaluator.
fn outline_work(
    font: &Font,
    name: &str,
    master: &MasterId,
    visiting: &mut Vec<String>,
    cache: &mut WorkCache,
) -> (usize, Option<String>) {
    if visiting.len() >= 64 || visiting.iter().any(|glyph| glyph == name) {
        return (
            1,
            Some("cyclic or excessively deep component outline".into()),
        );
    }
    if let Some(cost) = cache.get(name) {
        return cost.clone();
    }
    let Some(layer) = font
        .glyph(name)
        .and_then(|glyph| glyph.master_layer(master))
    else {
        return (1, Some(format!("missing outline source: {name}")));
    };
    let mut cost = layer.contours.iter().fold(1usize, |sum, contour| {
        sum.saturating_add(contour.points.len()).saturating_add(1)
    });
    let mut issue = None;
    visiting.push(name.to_owned());
    for base in layer
        .components
        .iter()
        .map(|component| &component.base)
        .chain(
            layer
                .constructions
                .iter()
                .map(|construction| &construction.source),
        )
    {
        let (child_cost, child_issue) = outline_work(font, base, master, visiting, cache);
        cost = cost.saturating_add(child_cost);
        if child_issue.is_some() {
            issue = child_issue;
            break;
        }
        if cost > MAX_GLYPH_WORK {
            break;
        }
    }
    visiting.pop();
    cache.insert(name.to_owned(), (cost, issue.clone()));
    (cost, issue)
}

fn path_signature(path: &BezPath) -> [u8; 32] {
    let mut hash = Sha256::new();
    for element in path.elements() {
        let zero = kurbo::Point::ZERO;
        let (tag, points, count): (u8, [kurbo::Point; 3], usize) = match element {
            PathEl::MoveTo(point) => (0, [*point, zero, zero], 1),
            PathEl::LineTo(point) => (1, [*point, zero, zero], 1),
            PathEl::QuadTo(a, b) => (2, [*a, *b, zero], 2),
            PathEl::CurveTo(a, b, c) => (3, [*a, *b, *c], 3),
            PathEl::ClosePath => (4, [zero; 3], 0),
        };
        hash.update([tag]);
        for point in points.into_iter().take(count) {
            for value in [point.x, point.y] {
                hash.update(if value == 0. { 0_f64 } else { value }.to_le_bytes());
            }
        }
    }
    hash.finalize().into()
}

/// Canonical digest of a layer's component references, ignoring stored order-free
/// metadata the model does not keep. Components carry no internal identifiers, so
/// this compares bases, transforms, anchors and smart values exactly.
fn component_signature(layer: &Layer) -> Result<[u8; 32], ApiError> {
    let mut hash = Sha256::new();
    for component in &layer.components {
        let encoded =
            serde_json::to_vec(component).map_err(|error| ApiError::internal(error.to_string()))?;
        hash.update((encoded.len() as u64).to_le_bytes());
        hash.update(&encoded);
    }
    for construction in &layer.constructions {
        let encoded = serde_json::to_vec(construction)
            .map_err(|error| ApiError::internal(error.to_string()))?;
        hash.update((encoded.len() as u64).to_le_bytes());
        hash.update(&encoded);
    }
    Ok(hash.finalize().into())
}

#[derive(PartialEq)]
struct Topology {
    contours: Vec<(bool, Vec<PointKind>)>,
    components: Vec<String>,
}

impl From<&Layer> for Topology {
    fn from(layer: &Layer) -> Self {
        Self {
            contours: layer
                .contours
                .iter()
                .map(|contour| {
                    (
                        contour.closed,
                        contour.points.iter().map(|point| point.kind).collect(),
                    )
                })
                .collect(),
            components: layer
                .components
                .iter()
                .map(|c| c.base.to_string())
                .collect(),
        }
    }
}

struct GlyphSnapshot {
    id: GlyphId,
    unicodes: Vec<u32>,
    variation_selectors: BTreeSet<(u32, u32)>,
    metrics: Option<(f64, Option<f64>, Option<f64>)>,
    topology: Option<Topology>,
    components: Option<[u8; 32]>,
    path: Option<[u8; 32]>,
    issue: Option<String>,
}

struct FontSnapshot {
    generation: u64,
    family: String,
    upem: u16,
    axes: Vec<Axis>,
    masters: Vec<(String, DesignLocation)>,
    master: MasterId,
    metrics: MasterMetrics,
    glyphs: BTreeMap<String, GlyphSnapshot>,
    master_name: String,
    info: serde_json::Value,
    groups: KerningGroups,
    kerning_h: Kerning,
    kerning_v: Kerning,
    features: Features,
    feature_settings: FeatureGenerationSettings,
    fea: String,
}

fn invalid(message: impl ToString) -> ApiError {
    ApiError::invalid_params(message.to_string())
}

fn selected_master(document: &Document, master: Option<&MasterId>) -> Result<MasterId, ApiError> {
    match master {
        Some(master) if document.font.master(master).is_none() => {
            Err(invalid("comparison master does not exist"))
        }
        Some(master) => Ok(master.clone()),
        None => Ok(document.font.default_master().id.clone()),
    }
}

fn validate_layer_size(layer: &Layer) -> Result<usize, ApiError> {
    let points = layer
        .contours
        .iter()
        .try_fold(layer.components.len(), |count, c| {
            count.checked_add(c.points.len())
        })
        .ok_or_else(|| invalid("comparison source point count overflow"))?;
    if points > MAX_GLYPH_ELEMENTS {
        return Err(invalid(
            "comparison glyph exceeds 100000 source points/components",
        ));
    }
    if [
        Some(layer.width),
        layer.vertical_width,
        layer.vertical_origin,
    ]
    .into_iter()
    .flatten()
    .any(|metric| !metric.is_finite())
    {
        return Err(invalid("comparison glyph metrics must be finite"));
    }
    Ok(points)
}

fn snapshot(document: &Document, master: Option<&MasterId>) -> Result<FontSnapshot, ApiError> {
    if document.font.glyphs.len() > MAX_GLYPHS {
        return Err(invalid("comparison exceeds 100000 glyphs per document"));
    }
    let master = selected_master(document, master)?;
    let selected = document
        .font
        .master(&master)
        .ok_or_else(|| invalid("comparison master does not exist"))?;
    let metrics = selected.metrics.clone();
    let master_name = selected.name.clone();
    if selected.kerning_h.0.len() > MAX_KERN_RECORDS
        || selected.kerning_v.0.len() > MAX_KERN_RECORDS
    {
        return Err(invalid(
            "comparison exceeds 200000 kerning records per master",
        ));
    }
    let kerning_h = selected.kerning_h.clone();
    let kerning_v = selected.kerning_v.clone();
    let info = serde_json::to_value(&document.font.info)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let mut result = FontSnapshot {
        generation: document.generation,
        family: document.font.info.family_name.clone(),
        upem: document.font.info.units_per_em,
        axes: document.font.axes.clone(),
        masters: document
            .font
            .masters
            .iter()
            .map(|m| (m.name.clone(), m.location.clone()))
            .collect(),
        master,
        metrics,
        glyphs: BTreeMap::new(),
        master_name,
        info,
        groups: document.font.kerning_groups.clone(),
        kerning_h,
        kerning_v,
        fea: document.font.features.to_fea(),
        features: document.font.features.clone(),
        feature_settings: document.font.feature_settings.clone(),
    };
    let mut elements = 0usize;
    let mut source_elements = 0usize;
    let mut work = 0usize;
    let mut work_cache = WorkCache::new();
    let mut failures = 0usize;
    for (name, glyph) in &document.font.glyphs {
        let mut unicodes = glyph.unicodes.clone();
        unicodes.sort_unstable();
        unicodes.dedup();
        let mut item = GlyphSnapshot {
            id: document
                .glyph_id(name)
                .ok_or_else(|| invalid("comparison glyph handle is missing"))?,
            unicodes,
            variation_selectors: glyph.variation_selectors.iter().copied().collect(),
            metrics: None,
            topology: None,
            components: None,
            path: None,
            issue: None,
        };
        if let Some(layer) = glyph.master_layer(&result.master) {
            source_elements = source_elements.saturating_add(validate_layer_size(layer)?);
            if source_elements > MAX_ELEMENTS {
                return Err(invalid(
                    "comparison exceeds 16000000 source points/components per document",
                ));
            }
            item.metrics = Some((layer.width, layer.vertical_width, layer.vertical_origin));
            item.topology = Some(Topology::from(layer));
            item.components = Some(component_signature(layer)?);
            let (cost, issue) = outline_work(
                &document.font,
                name,
                &result.master,
                &mut Vec::new(),
                &mut work_cache,
            );
            work = work.saturating_add(cost);
            if cost > MAX_GLYPH_WORK || work > MAX_ELEMENTS {
                return Err(invalid(
                    "comparison component expansion exceeds its bounded work budget",
                ));
            }
            if issue.is_some() {
                item.issue = issue;
                result.glyphs.insert(name.to_string(), item);
                continue;
            }
            match fc_geom::evaluate_layer(&document.font, name, &layer.id) {
                Ok(outcome) => {
                    elements = elements.saturating_add(outcome.path.elements().len());
                    if elements > MAX_ELEMENTS {
                        return Err(invalid(
                            "comparison exceeds 16000000 resolved path elements per document",
                        ));
                    }
                    if outcome.path.elements().len() > MAX_GLYPH_ELEMENTS {
                        return Err(invalid(
                            "comparison glyph exceeds 100000 resolved path elements",
                        ));
                    }
                    if outcome.path.is_finite() {
                        item.path = Some(path_signature(&outcome.path));
                    } else {
                        item.issue = Some("outline contains non-finite coordinates".into());
                    }
                }
                Err(error) => {
                    failures += 1;
                    if failures > 32 {
                        return Err(invalid(
                            "comparison exceeds 32 geometry evaluation failures",
                        ));
                    }
                    item.issue = Some(error.to_string());
                }
            }
        } else {
            item.issue = Some("selected master layer is missing".into());
        }
        result.glyphs.insert(name.to_string(), item);
    }
    Ok(result)
}

fn same_axes(left: &[Axis], right: &[Axis]) -> bool {
    left.len() == right.len()
        && left.iter().all(|axis| {
            right.iter().any(|other| {
                axis.tag == other.tag
                    && axis.min == other.min
                    && axis.default == other.default
                    && axis.max == other.max
                    && axis.map == other.map
            })
        })
}

fn same_masters(left: &[(String, DesignLocation)], right: &[(String, DesignLocation)]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut unmatched: Vec<_> = right.iter().collect();
    for master in left {
        let Some(index) = unmatched.iter().position(|other| *other == master) else {
            return false;
        };
        unmatched.swap_remove(index);
    }
    true
}

fn compare_glyph(
    name: String,
    target: Option<&GlyphSnapshot>,
    source: Option<&GlyphSnapshot>,
) -> FontCompareGlyph {
    let mut issues = Vec::new();
    for (side, glyph) in [("target", target), ("source", source)] {
        if let Some(issue) = glyph.and_then(|glyph| glyph.issue.as_ref()) {
            issues.push(format!("{side}: {issue}"));
        }
    }
    let (outline_changed, metrics_changed, unicode_changed, compatible, components_changed) =
        match (target, source) {
            (Some(target), Some(source)) => (
                target.path != source.path || !issues.is_empty(),
                target.metrics != source.metrics,
                target.unicodes != source.unicodes
                    || target.variation_selectors != source.variation_selectors,
                target.topology.is_some() && target.topology == source.topology,
                target.components != source.components,
            ),
            _ => (false, false, false, false, false),
        };
    let status = match (target, source) {
        (None, _) => FontCompareStatus::MissingTarget,
        (_, None) => FontCompareStatus::MissingSource,
        _ if outline_changed
            || metrics_changed
            || unicode_changed
            || components_changed
            || !compatible =>
        {
            FontCompareStatus::Different
        }
        _ => FontCompareStatus::Equal,
    };
    FontCompareGlyph {
        name,
        target: target.map(|glyph| glyph.id),
        source: source.map(|glyph| glyph.id),
        status,
        outline_changed,
        metrics_changed,
        unicode_changed,
        compatible,
        components_changed,
        target_width: target.and_then(|glyph| glyph.metrics.map(|metrics| metrics.0)),
        source_width: source.and_then(|glyph| glyph.metrics.map(|metrics| metrics.0)),
        target_unicodes: target
            .map(|glyph| glyph.unicodes.clone())
            .unwrap_or_default(),
        source_unicodes: source
            .map(|glyph| glyph.unicodes.clone())
            .unwrap_or_default(),
        issues,
    }
}

fn render(value: &serde_json::Value) -> String {
    let text = match value {
        serde_json::Value::String(text) => text.clone(),
        other => other.to_string(),
    };
    if text.chars().count() > 200 {
        text.chars().take(199).chain(['\u{2026}']).collect()
    } else {
        text
    }
}

/// Differing font-info fields, excluding the two that have their own flags.
fn metadata_diff(
    target: &serde_json::Value,
    source: &serde_json::Value,
) -> Vec<FontCompareFieldDiff> {
    let empty = serde_json::Map::new();
    let target = target.as_object().unwrap_or(&empty);
    let source = source.as_object().unwrap_or(&empty);
    let mut fields: BTreeSet<&String> = target.keys().chain(source.keys()).collect();
    fields.remove(&"family_name".to_owned());
    fields.remove(&"units_per_em".to_owned());
    let null = serde_json::Value::Null;
    fields
        .into_iter()
        .filter_map(|field| {
            let left = target.get(field).unwrap_or(&null);
            let right = source.get(field).unwrap_or(&null);
            (left != right).then(|| FontCompareFieldDiff {
                field: field.clone(),
                target: render(left),
                source: render(right),
            })
        })
        .take(MAX_METADATA_FIELDS)
        .collect()
}

fn side_label(side: &KernSide) -> String {
    match side {
        KernSide::Glyph(name) => name.to_string(),
        KernSide::Group(group) => format!("@{group}"),
    }
}

fn group_diff(
    label: &str,
    target: &BTreeMap<String, std::collections::BTreeSet<fc_model::GlyphName>>,
    source: &BTreeMap<String, std::collections::BTreeSet<fc_model::GlyphName>>,
    result: &mut FontCompareKerning,
) {
    for name in target.keys() {
        if !source.contains_key(name) {
            push_bounded(&mut result.groups_only_target, format!("{label}:{name}"));
        }
    }
    for (name, members) in source {
        match target.get(name) {
            None => push_bounded(&mut result.groups_only_source, format!("{label}:{name}")),
            Some(existing) if existing != members => {
                push_bounded(&mut result.groups_changed, format!("{label}:{name}"))
            }
            Some(_) => {}
        }
    }
}

fn push_bounded(list: &mut Vec<String>, value: String) {
    if list.len() < MAX_LIST_ITEMS {
        list.push(value);
    }
}

fn kerning_records(
    target: &Kerning,
    source: &Kerning,
    vertical: bool,
    result: &mut FontCompareKerning,
) {
    let keys: BTreeSet<_> = target.0.keys().chain(source.0.keys()).collect();
    for key in keys {
        let left = target.0.get(key).copied();
        let right = source.0.get(key).copied();
        match (left, right) {
            (Some(_), None) => result.pairs_only_target += 1,
            (None, Some(_)) => result.pairs_only_source += 1,
            (Some(a), Some(b)) if a == b => {
                result.pairs_equal += 1;
                continue;
            }
            _ => result.pairs_changed += 1,
        }
        if result.examples.len() < MAX_PAIR_EXAMPLES {
            result.examples.push(FontComparePairDiff {
                left: side_label(&key.0),
                right: side_label(&key.1),
                vertical,
                target: left,
                source: right,
            });
        }
    }
}

fn kerning_diff(target: &FontSnapshot, source: &FontSnapshot) -> FontCompareKerning {
    let mut result = FontCompareKerning::default();
    group_diff(
        "side1",
        &target.groups.side1,
        &source.groups.side1,
        &mut result,
    );
    group_diff(
        "side2",
        &target.groups.side2,
        &source.groups.side2,
        &mut result,
    );
    kerning_records(&target.kerning_h, &source.kerning_h, false, &mut result);
    kerning_records(&target.kerning_v, &source.kerning_v, true, &mut result);
    result
}

/// Compares editable layout source by stable name, ignoring generated item IDs.
fn layout_diff(target: &FontSnapshot, source: &FontSnapshot) -> FontCompareLayout {
    let mut result = FontCompareLayout {
        source_of_truth_changed: target.features.source_of_truth != source.features.source_of_truth
            || target.feature_settings != source.feature_settings,
        feature_source_equal: target.fea == source.fea,
        ..Default::default()
    };
    let blocks = |snapshot: &FontSnapshot| {
        snapshot
            .features
            .prefixes
            .iter()
            .map(|block| (format!("prefix:{}", block.tag), block_key(block)))
            .chain(
                snapshot
                    .features
                    .features
                    .iter()
                    .map(|block| (format!("feature:{}", block.tag), block_key(block))),
            )
            .collect::<BTreeMap<_, _>>()
    };
    let (left, right) = (blocks(target), blocks(source));
    for (name, key) in &left {
        match right.get(name) {
            None => push_bounded(&mut result.only_target, name.clone()),
            Some(other) if other != key => push_bounded(&mut result.changed, name.clone()),
            Some(_) => {}
        }
    }
    for name in right.keys() {
        if !left.contains_key(name) {
            push_bounded(&mut result.only_source, name.clone());
        }
    }
    let classes = |snapshot: &FontSnapshot| {
        snapshot
            .features
            .classes
            .iter()
            .map(|class| (class.name.clone(), (class.code.clone(), class.automatic)))
            .collect::<BTreeMap<_, _>>()
    };
    let (left, right) = (classes(target), classes(source));
    for (name, key) in &left {
        match right.get(name) {
            None => push_bounded(&mut result.classes_only_target, name.clone()),
            Some(other) if other != key => push_bounded(&mut result.classes_changed, name.clone()),
            Some(_) => {}
        }
    }
    for name in right.keys() {
        if !left.contains_key(name) {
            push_bounded(&mut result.classes_only_source, name.clone());
        }
    }
    result
}

fn block_key(block: &fc_model::FeatureBlock) -> (String, bool, bool, String) {
    (
        block.code.clone(),
        block.automatic,
        block.disabled,
        block.notes.clone(),
    )
}

fn layout_equal(layout: &FontCompareLayout) -> bool {
    layout.only_target.is_empty()
        && layout.only_source.is_empty()
        && layout.changed.is_empty()
        && layout.classes_only_target.is_empty()
        && layout.classes_only_source.is_empty()
        && layout.classes_changed.is_empty()
        && !layout.source_of_truth_changed
        && layout.feature_source_equal
}

fn kerning_equal(kerning: &FontCompareKerning) -> bool {
    kerning.groups_only_target.is_empty()
        && kerning.groups_only_source.is_empty()
        && kerning.groups_changed.is_empty()
        && kerning.pairs_only_target == 0
        && kerning.pairs_only_source == 0
        && kerning.pairs_changed == 0
}

/// Compatibility flags and the sectioned differences of two snapshots.
fn compatibility(
    target: &FontSnapshot,
    source: &FontSnapshot,
) -> (
    FontCompareCompatibility,
    Vec<FontCompareFieldDiff>,
    FontCompareKerning,
    FontCompareLayout,
) {
    let metadata = metadata_diff(&target.info, &source.info);
    let kerning = kerning_diff(target, source);
    let layout = layout_diff(target, source);
    let flags = FontCompareCompatibility {
        family_name: target.family == source.family,
        units_per_em: target.upem == source.upem,
        axes: same_axes(&target.axes, &source.axes),
        masters: same_masters(&target.masters, &source.masters),
        glyph_set: target.glyphs.keys().eq(source.glyphs.keys()),
        master_metrics: target.metrics == source.metrics,
        metadata: metadata.is_empty(),
        kerning: kerning_equal(&kerning),
        layout: layout_equal(&layout),
    };
    (flags, metadata, kerning, layout)
}

pub(crate) fn compare(api: &Api, params: FontCompareParams) -> Result<FontCompareResult, ApiError> {
    if !(1..=512).contains(&params.limit) {
        return Err(invalid("comparison page limit must be between 1 and 512"));
    }
    let target = api
        .session
        .with_doc(params.doc, |doc| snapshot(doc, params.master.as_ref()))
        .map_err(ApiError::from)??;
    let source = api
        .session
        .with_doc(params.source_doc, |doc| {
            snapshot(doc, params.source_master.as_ref())
        })
        .map_err(ApiError::from)??;
    let (compatibility, metadata, kerning, layout) = compatibility(&target, &source);
    let names: BTreeSet<_> = target
        .glyphs
        .keys()
        .chain(source.glyphs.keys())
        .cloned()
        .collect();
    let mut counts = FontCompareCounts::default();
    let mut glyphs = Vec::new();
    let mut total = 0;
    for name in names {
        let row = compare_glyph(
            name.clone(),
            target.glyphs.get(&name),
            source.glyphs.get(&name),
        );
        match row.status {
            FontCompareStatus::Equal => counts.equal += 1,
            FontCompareStatus::MissingTarget => counts.missing_target += 1,
            FontCompareStatus::MissingSource => counts.missing_source += 1,
            FontCompareStatus::Different => {
                counts.outlines += u32::from(row.outline_changed);
                counts.metrics += u32::from(row.metrics_changed);
                counts.unicodes += u32::from(row.unicode_changed);
                counts.incompatible += u32::from(!row.compatible);
                counts.components += u32::from(row.components_changed);
            }
        }
        if !params.differing_only || row.status != FontCompareStatus::Equal {
            if total >= params.offset && glyphs.len() < params.limit as usize {
                glyphs.push(row);
            }
            total += 1;
        }
    }
    Ok(FontCompareResult {
        generation: target.generation,
        source_generation: source.generation,
        total,
        offset: params.offset,
        limit: params.limit,
        doc_family: target.family,
        source_family: source.family,
        doc_upem: target.upem,
        source_upem: source.upem,
        master: target.master,
        source_master: source.master,
        compatibility,
        counts,
        glyphs,
        metadata,
        kerning,
        layout,
    })
}

/// Compares every requested (document, master) member against one reference.
pub(crate) fn compare_family(
    api: &Api,
    params: FontCompareFamilyParams,
) -> Result<FontCompareFamilyResult, ApiError> {
    let reference = api
        .session
        .with_doc(params.doc, |doc| snapshot(doc, params.master.as_ref()))
        .map_err(ApiError::from)??;
    let mut requested = Vec::new();
    for doc in std::iter::once(params.doc).chain(params.docs.iter().copied()) {
        let masters = api
            .session
            .with_doc(doc, |document| {
                if params.all_masters {
                    document
                        .font
                        .masters
                        .iter()
                        .map(|master| master.id.clone())
                        .collect::<Vec<_>>()
                } else {
                    vec![document.font.default_master().id.clone()]
                }
            })
            .map_err(ApiError::from)?;
        for master in masters {
            // A member repeated in `docs`, and the reference member itself, are
            // compared once.
            if (doc == params.doc && master == reference.master)
                || requested.contains(&(doc, master.clone()))
            {
                continue;
            }
            requested.push((doc, master));
        }
    }
    if requested.len() + 1 > MAX_FAMILY_MEMBERS {
        return Err(invalid(
            "family comparison is limited to 64 documents and masters",
        ));
    }
    let mut members = vec![FontCompareFamilyMember {
        doc: params.doc,
        family: reference.family.clone(),
        master: reference.master.clone(),
        master_name: reference.master_name.clone(),
        reference: true,
        compatibility: FontCompareCompatibility {
            family_name: true,
            units_per_em: true,
            axes: true,
            masters: true,
            glyph_set: true,
            master_metrics: true,
            metadata: true,
            kerning: true,
            layout: true,
        },
        counts: FontCompareCounts::default(),
        examples: Vec::new(),
        issue: None,
    }];
    let mut compatible = true;
    for (doc, master) in requested {
        let member = api
            .session
            .with_doc(doc, |document| snapshot(document, Some(&master)));
        let candidate = match member.map_err(ApiError::from)? {
            Ok(candidate) => candidate,
            // One unreadable member must not abandon the whole family report.
            Err(error) => {
                compatible = false;
                members.push(FontCompareFamilyMember {
                    doc,
                    family: String::new(),
                    master,
                    master_name: String::new(),
                    reference: false,
                    compatibility: FontCompareCompatibility {
                        family_name: false,
                        units_per_em: false,
                        axes: false,
                        masters: false,
                        glyph_set: false,
                        master_metrics: false,
                        metadata: false,
                        kerning: false,
                        layout: false,
                    },
                    counts: FontCompareCounts::default(),
                    examples: Vec::new(),
                    issue: Some(error.message),
                });
                continue;
            }
        };
        let (flags, _, _, _) = compatibility(&candidate, &reference);
        let mut counts = FontCompareCounts::default();
        let mut examples = Vec::new();
        let names: BTreeSet<_> = candidate
            .glyphs
            .keys()
            .chain(reference.glyphs.keys())
            .cloned()
            .collect();
        for name in names {
            let row = compare_glyph(
                name.clone(),
                candidate.glyphs.get(&name),
                reference.glyphs.get(&name),
            );
            match row.status {
                FontCompareStatus::Equal => counts.equal += 1,
                FontCompareStatus::MissingTarget => counts.missing_target += 1,
                FontCompareStatus::MissingSource => counts.missing_source += 1,
                FontCompareStatus::Different => {
                    counts.outlines += u32::from(row.outline_changed);
                    counts.metrics += u32::from(row.metrics_changed);
                    counts.unicodes += u32::from(row.unicode_changed);
                    counts.incompatible += u32::from(!row.compatible);
                    counts.components += u32::from(row.components_changed);
                }
            }
            if row.status != FontCompareStatus::Equal && examples.len() < MAX_LIST_ITEMS {
                examples.push(name);
            }
        }
        let matches = examples.is_empty()
            && [
                flags.family_name,
                flags.units_per_em,
                flags.axes,
                flags.masters,
                flags.glyph_set,
                flags.master_metrics,
                flags.metadata,
                flags.kerning,
                flags.layout,
            ]
            .into_iter()
            .all(|flag| flag);
        compatible &= matches;
        members.push(FontCompareFamilyMember {
            doc,
            family: candidate.family.clone(),
            master: candidate.master.clone(),
            master_name: candidate.master_name.clone(),
            reference: false,
            compatibility: flags,
            counts,
            examples,
            issue: None,
        });
    }
    Ok(FontCompareFamilyResult {
        members,
        compatible,
    })
}

/// Opens a compiled or source font as a reference the kernel refuses to modify.
pub(crate) fn open_reference(
    api: &Api,
    params: FontReferenceParams,
) -> Result<FontReferenceResult, ApiError> {
    let summary = super::session::open(
        api,
        OpenParams {
            path: params.path.clone(),
            face_index: params.face_index,
        },
    )?;
    // Mark before returning: an edit attempted between opening and marking would
    // still be rejected because no handle has been published yet.
    api.session
        .with_doc_mut(summary.doc, |document| document.set_read_only(true))
        .map_err(ApiError::from)?;
    let (family, upem, masters) = api
        .session
        .with_doc(summary.doc, |document| {
            (
                document.font.info.family_name.clone(),
                document.font.info.units_per_em,
                document
                    .font
                    .masters
                    .iter()
                    .map(|master| FontReferenceMaster {
                        id: master.id.clone(),
                        name: master.name.clone(),
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .map_err(ApiError::from)?;
    Ok(FontReferenceResult {
        doc: summary.doc,
        family,
        path: params.path,
        units_per_em: upem,
        masters,
        read_only: true,
        warnings: summary
            .warnings
            .into_iter()
            .map(|warning| warning.message)
            .collect(),
    })
}

fn overlay(
    document: &Document,
    master: Option<&MasterId>,
    name: &str,
    expected_generation: Option<u64>,
) -> Result<(Option<FontCompareOutline>, Option<Rect>, bool), ApiError> {
    if expected_generation.is_some_and(|expected| expected != document.generation) {
        return Err(invalid("comparison is stale; compare the documents again"));
    }
    let master = selected_master(document, master)?;
    let Some(glyph) = document.font.glyph(name) else {
        return Ok((None, None, false));
    };
    let Some(layer) = glyph.master_layer(&master) else {
        return Ok((None, None, true));
    };
    validate_layer_size(layer)?;
    let (work, issue) = outline_work(
        &document.font,
        name,
        &master,
        &mut Vec::new(),
        &mut WorkCache::new(),
    );
    if work > MAX_GLYPH_WORK {
        return Err(invalid(
            "comparison component expansion exceeds its bounded work budget",
        ));
    }
    if let Some(issue) = issue {
        return Err(invalid(issue));
    }
    let outcome = fc_geom::evaluate_layer(&document.font, name, &layer.id).map_err(invalid)?;
    if !outcome.path.is_finite() || outcome.path.elements().len() > MAX_GLYPH_ELEMENTS {
        return Err(invalid(
            "comparison overlay exceeds 100000 finite path elements",
        ));
    }
    let bounds = (!outcome.path.is_empty()).then(|| outcome.path.bounding_box());
    let outline = FontCompareOutline {
        path: outcome.path.to_svg(),
        bounds: bounds.map(|b| [b.x0, b.y0, b.x1, b.y1]).unwrap_or([0.; 4]),
        width: layer.width,
        vertical_width: layer.vertical_width,
        vertical_origin: layer.vertical_origin,
        upem: document.font.info.units_per_em,
        approximated: !outcome.approximated_layers.is_empty(),
    };
    Ok((Some(outline), bounds, true))
}

pub(crate) fn glyph(
    api: &Api,
    params: FontCompareGlyphParams,
) -> Result<FontCompareGlyphResult, ApiError> {
    if params.name.is_empty() || params.name.len() > 1024 {
        return Err(invalid("comparison glyph name must contain 1–1024 bytes"));
    }
    let (target, target_bounds, target_exists) = api
        .session
        .with_doc(params.doc, |doc| {
            overlay(
                doc,
                params.master.as_ref(),
                &params.name,
                params.expected_generation,
            )
        })
        .map_err(ApiError::from)??;
    let (source, source_bounds, source_exists) = api
        .session
        .with_doc(params.source_doc, |doc| {
            overlay(
                doc,
                params.source_master.as_ref(),
                &params.name,
                params.expected_source_generation,
            )
        })
        .map_err(ApiError::from)??;
    if !target_exists && !source_exists {
        return Err(invalid("comparison glyph is absent from both documents"));
    }
    let bounds = match (target_bounds, source_bounds) {
        (Some(target), Some(source)) => target.union(source),
        (Some(bounds), None) | (None, Some(bounds)) => bounds,
        (None, None) => Rect::ZERO,
    };
    Ok(FontCompareGlyphResult {
        name: params.name,
        target,
        source,
        bounds: [bounds.x0, bounds.y0, bounds.x1, bounds.y1],
    })
}
