//! Glyph-grid filtering, sorting, and detail handlers.

use std::cmp::Ordering;

use fc_doc::Document;
use fc_glyphdata::Category;
use fc_model::{Glyph, GlyphId, Layer, LayerKind, MasterId};
use kurbo::{BezPath, PathEl, Point, Shape};

use crate::{
    Api, ApiError, GlyphCell, GlyphCellParams, GlyphDetail, GlyphDetailParams, GlyphFilter,
    GlyphIndexParams, GlyphListParams, GlyphListResult, GlyphSearchParams, GlyphSort, LayerPath,
};

const MAX_GLYPH_PAGE: u32 = 2_000;

pub(crate) struct Candidate {
    order: usize,
    pub(crate) gid: GlyphId,
    name: String,
    first_unicode: Option<u32>,
    width: f64,
    search_rank: u8,
    radical: (u16, i16, u8),
    strokes: u16,
    reading: String,
    cid: u32,
}

pub(crate) fn list(api: &Api, params: GlyphListParams) -> Result<GlyphListResult, ApiError> {
    if params.limit > MAX_GLYPH_PAGE {
        return Err(ApiError::invalid_params(format!(
            "glyph list limit {} exceeds {MAX_GLYPH_PAGE}",
            params.limit
        )));
    }
    list_impl(api, params, None)
}

pub(crate) fn cell(api: &Api, params: GlyphCellParams) -> Result<GlyphCell, ApiError> {
    api.session
        .with_doc(params.doc, |document| {
            build_cell(document, params.gid, &params.master, true)
        })
        .map_err(ApiError::from)?
}

pub(crate) fn index_of(api: &Api, params: GlyphIndexParams) -> Result<Option<u32>, ApiError> {
    api.session
        .with_doc(params.doc, |document| {
            let master = selected_master(document, params.master.as_ref())?;
            let mut matches = matching_candidates(document, &master, &params.filter, None)?;
            sort_candidates(&mut matches, params.sort);
            Ok(matches
                .iter()
                .position(|candidate| candidate.gid == params.gid)
                .map(|index| u32::try_from(index).unwrap_or(u32::MAX)))
        })
        .map_err(ApiError::from)?
}

pub(crate) fn detail(api: &Api, params: GlyphDetailParams) -> Result<GlyphDetail, ApiError> {
    api.session
        .with_doc(params.doc, |document| build_detail(document, params.gid))
        .map_err(ApiError::from)?
}

pub(crate) fn search(api: &Api, params: GlyphSearchParams) -> Result<Vec<GlyphCell>, ApiError> {
    let query = params.query;
    let result = list_impl(
        api,
        GlyphListParams {
            doc: params.doc,
            master: None,
            filter: GlyphFilter {
                query: Some(query.clone()),
                ..GlyphFilter::default()
            },
            sort: GlyphSort::Order,
            offset: 0,
            limit: params.limit.min(MAX_GLYPH_PAGE),
            include_paths: false,
        },
        Some(&query),
    )?;
    Ok(result.items)
}

fn list_impl(
    api: &Api,
    params: GlyphListParams,
    search_query: Option<&str>,
) -> Result<GlyphListResult, ApiError> {
    api.session
        .with_doc(params.doc, |document| {
            let master = selected_master(document, params.master.as_ref())?;
            let mut matches = matching_candidates(document, &master, &params.filter, search_query)?;
            if search_query.is_some() {
                matches.sort_by(|left, right| {
                    left.search_rank
                        .cmp(&right.search_rank)
                        .then_with(|| left.order.cmp(&right.order))
                });
            } else {
                sort_candidates(&mut matches, params.sort);
            }
            let total = u32::try_from(matches.len()).unwrap_or(u32::MAX);
            let offset = params.offset as usize;
            let limit = params.limit as usize;
            let items = matches
                .into_iter()
                .skip(offset)
                .take(limit)
                .map(|candidate| build_cell(document, candidate.gid, &master, params.include_paths))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(GlyphListResult { total, items })
        })
        .map_err(ApiError::from)?
}

pub(crate) fn matching_candidates(
    document: &Document,
    master: &MasterId,
    filter: &GlyphFilter,
    search_query: Option<&str>,
) -> Result<Vec<Candidate>, ApiError> {
    let query = super::glyph_query::GlyphQuery::new(filter)?;
    let smart = filter
        .smart_expression
        .as_deref()
        .filter(|expression| !expression.trim().is_empty())
        .map(super::smart_filter_parser::parse)
        .transpose()?
        .as_ref()
        .map(super::smart_filter_eval::CompiledSmartFilter::new)
        .transpose()?;
    let names = filter.glyph_names.as_ref().map(|names| {
        names
            .iter()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>()
    });
    if let Some(cjk) = &filter.cjk {
        super::cjk::validate(cjk)?;
    }
    let cid_map: std::collections::BTreeMap<_, _> = document
        .font
        .cid
        .iter()
        .flat_map(|c| c.cid_map.iter().map(|(id, name)| (name, *id)))
        .collect();
    let mut matches = Vec::new();
    for (order, name) in document.font.glyph_order().enumerate() {
        let glyph = document.font.glyph(name).ok_or_else(|| {
            ApiError::internal(format!("glyph order references missing glyph `{name}`"))
        })?;
        let layer = glyph.master_layer(master);
        let gid = document
            .glyph_id(name)
            .ok_or_else(|| ApiError::internal(format!("glyph `{name}` has no session handle")))?;
        let generation = document.glyph_generation(gid);
        if names
            .as_ref()
            .is_some_and(|names| !names.contains(name.as_ref()))
            || query
                .as_ref()
                .is_some_and(|query| query.rank(glyph).is_none())
            || smart
                .as_ref()
                .is_some_and(|smart| !smart.matches(&document.font, glyph, layer, generation))
        {
            continue;
        }
        if matches_filter(glyph, layer, generation, filter) {
            let unihan = glyph
                .unicodes
                .first()
                .and_then(|cp| fc_glyphdata::unihan(*cp));
            matches.push(Candidate {
                order,
                gid,
                name: name.to_string(),
                first_unicode: glyph.unicodes.first().copied(),
                radical: unihan
                    .and_then(|d| d.radicals().next())
                    .map(|r| (r.radical, r.residual, r.simplified))
                    .unwrap_or((u16::MAX, i16::MAX, u8::MAX)),
                strokes: unihan.and_then(|d| d.strokes().next()).unwrap_or(u16::MAX),
                reading: unihan.filter(|d| !d.mandarin.is_empty()).map_or_else(
                    || "\u{10ffff}".into(),
                    |d| fc_glyphdata::normalize_reading(d.mandarin),
                ),
                cid: cid_map.get(name).copied().unwrap_or(u32::MAX),
                width: layer.map_or(0.0, |layer| layer.width),
                search_rank: search_query
                    .and_then(|_| query.as_ref().and_then(|query| query.rank(glyph)))
                    .unwrap_or_default(),
            });
        }
    }
    Ok(matches)
}

pub(crate) fn selected_master(
    document: &Document,
    requested: Option<&MasterId>,
) -> Result<MasterId, ApiError> {
    let master = requested
        .cloned()
        .unwrap_or_else(|| document.font.default_master().id.clone());
    document
        .font
        .master(&master)
        .map(|_| master.clone())
        .ok_or_else(|| ApiError::not_found(format!("master `{master}` does not exist")))
}

fn matches_filter(
    glyph: &Glyph,
    layer: Option<&Layer>,
    generation: u64,
    filter: &GlyphFilter,
) -> bool {
    if filter.cjk.as_ref().is_some_and(|filter| {
        !glyph
            .unicodes
            .iter()
            .any(|cp| super::cjk::matches(*cp, filter))
    }) {
        return false;
    }
    let first_unicode = glyph.unicodes.first().copied();
    if filter.unencoded_only && first_unicode.is_some() {
        return false;
    }
    if !filter.categories.is_empty()
        && !category_matches(first_unicode, filter.categories.as_slice())
    {
        return false;
    }
    if !filter.scripts.is_empty()
        && !filter
            .scripts
            .iter()
            .any(|allowed| allowed == first_unicode.map(fc_glyphdata::script).unwrap_or("Zzzz"))
    {
        return false;
    }
    if !filter.blocks.is_empty()
        && !filter.blocks.iter().any(|allowed| {
            allowed
                == first_unicode
                    .map(fc_glyphdata::block_name)
                    .unwrap_or("Unencoded")
        })
    {
        return false;
    }
    if !filter.color_labels.is_empty()
        && !glyph
            .color_label
            .is_some_and(|label| filter.color_labels.contains(&label))
    {
        return false;
    }
    if !filter.tags.is_empty()
        && !filter
            .tags
            .iter()
            .any(|allowed| glyph.tags.contains(allowed))
    {
        return false;
    }
    if !filter.unicode_ranges.is_empty()
        && !glyph.unicodes.iter().any(|unicode| {
            filter
                .unicode_ranges
                .iter()
                .any(|range| (range[0]..=range[1]).contains(unicode))
        })
    {
        return false;
    }
    if filter
        .has_components
        .is_some_and(|expected| expected == layer.is_none_or(|layer| layer.components.is_empty()))
    {
        return false;
    }
    !filter.changed_only || generation > 0
}

fn category_matches(first_unicode: Option<u32>, allowed: &[String]) -> bool {
    let Some(unicode) = first_unicode else {
        return allowed.iter().any(|value| value == "Unencoded");
    };
    let category = category_name(fc_glyphdata::category(unicode));
    let sub_category = fc_glyphdata::sub_category(unicode);
    allowed.iter().any(|value| {
        value == category || value == sub_category || value == &format!("{category}/{sub_category}")
    })
}

pub(crate) fn sort_candidates(candidates: &mut [Candidate], sort: GlyphSort) {
    candidates.sort_by(|left, right| match sort {
        GlyphSort::Radical => left
            .radical
            .cmp(&right.radical)
            .then_with(|| unicode_order(left, right)),
        GlyphSort::Strokes => left
            .strokes
            .cmp(&right.strokes)
            .then_with(|| unicode_order(left, right)),
        GlyphSort::Reading => left
            .reading
            .cmp(&right.reading)
            .then_with(|| unicode_order(left, right)),
        GlyphSort::Cid => left
            .cid
            .cmp(&right.cid)
            .then_with(|| unicode_order(left, right)),
        GlyphSort::Order => left.order.cmp(&right.order),
        GlyphSort::Unicode => unicode_order(left, right),
        GlyphSort::Name => left
            .name
            .cmp(&right.name)
            .then_with(|| left.order.cmp(&right.order)),
        GlyphSort::Width => left
            .width
            .total_cmp(&right.width)
            .then_with(|| left.order.cmp(&right.order)),
    });
}

fn unicode_order(left: &Candidate, right: &Candidate) -> Ordering {
    match (left.first_unicode, right.first_unicode) {
        (Some(left_unicode), Some(right_unicode)) => left_unicode
            .cmp(&right_unicode)
            .then_with(|| left.order.cmp(&right.order)),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => left.order.cmp(&right.order),
    }
}

pub(crate) fn build_cell(
    document: &Document,
    gid: GlyphId,
    master: &MasterId,
    include_path: bool,
) -> Result<GlyphCell, ApiError> {
    use super::glyph_status;

    if document.font.master(master).is_none() {
        return Err(ApiError::not_found(format!(
            "master `{master}` does not exist"
        )));
    }
    let name = document
        .glyph_name(gid)
        .ok_or_else(|| ApiError::not_found(format!("glyph handle {} does not exist", gid.0)))?;
    let glyph = document
        .font
        .glyph(name)
        .ok_or_else(|| ApiError::not_found(format!("glyph `{name}` does not exist")))?;
    let (category, sub_category, script) = glyph_classification(glyph);
    let Some(layer) = glyph.master_layer(master) else {
        return Ok(GlyphCell {
            gid,
            name: name.to_string(),
            unicodes: glyph.unicodes.clone(),
            category,
            sub_category,
            script,
            width: 0.0,
            vertical_advance: None,
            vertical_origin: None,
            bbox: None,
            path: include_path.then(String::new),
            color_label: glyph.color_label,
            export: glyph.export,
            has_components: false,
            is_empty: true,
            metrics_out_of_sync: glyph_status::metrics_out_of_sync(
                &document.font,
                glyph,
                master,
                None,
            ),
            has_annotations: glyph_status::has_annotations(glyph),
            has_custom_glyph_info: glyph_status::has_custom_glyph_info(glyph),
            incompatible_masters: glyph_status::incompatible_masters(&document.font, glyph),
            locked: glyph.locked,
            generation: document.glyph_generation(gid),
        });
    };
    let mut flattened = document.flattened_path(gid, master)?;
    let outline_bbox = (!flattened.elements().is_empty()).then(|| {
        let bounds = flattened.bounding_box();
        [bounds.x0, bounds.y0, bounds.x1, bounds.y1]
    });
    let bitmap_preview = flattened.elements().is_empty()
        && document
            .font
            .bitmap_strikes
            .iter()
            .any(|s| s.glyphs.contains_key(name));
    if bitmap_preview {
        if let Some(strike) = document
            .font
            .bitmap_strikes
            .iter()
            .find(|s| s.glyphs.contains_key(name))
        {
            let image = strike
                .flattened(name)
                .map_err(|e| ApiError::invalid_params(e.to_string()))?;
            let b = &image.bitmap;
            if usize::from(b.width) * usize::from(b.height) <= 262_144 {
                let scale = f64::from(document.font.info.units_per_em) / f64::from(strike.ppem);
                let mut path = BezPath::new();
                for row in 0..b.height {
                    for col in 0..b.width {
                        if b.coverage(col, row) == 0 {
                            continue;
                        }
                        let x = (f64::from(b.origin_x) + f64::from(col)) * scale;
                        let y = (f64::from(b.origin_y) + f64::from(b.height - row)) * scale;
                        path.move_to((x, y));
                        path.line_to((x + scale, y));
                        path.line_to((x + scale, y - scale));
                        path.line_to((x, y - scale));
                        path.close_path();
                    }
                }
                flattened = std::sync::Arc::new(path);
            }
        }
    }
    let bbox = if bitmap_preview {
        (!flattened.elements().is_empty()).then(|| {
            let bounds = flattened.bounding_box();
            [bounds.x0, bounds.y0, bounds.x1, bounds.y1]
        })
    } else {
        outline_bbox
    };
    Ok(GlyphCell {
        gid,
        name: name.to_string(),
        unicodes: glyph.unicodes.clone(),
        category,
        sub_category,
        script,
        width: layer.width,
        vertical_advance: Some(
            layer
                .vertical_width
                .or_else(|| {
                    document
                        .font
                        .master(master)
                        .and_then(|m| m.metrics.default_vertical_advance)
                })
                .unwrap_or(f64::from(document.font.info.units_per_em)),
        ),
        vertical_origin: Some(
            layer
                .vertical_origin
                .or_else(|| {
                    document.font.master(master).and_then(|m| {
                        m.metrics
                            .default_vertical_origin
                            .or(Some(m.metrics.ascender))
                    })
                })
                .unwrap_or_default(),
        ),
        bbox,
        path: include_path.then(|| rounded_svg(&flattened)),
        color_label: glyph.color_label,
        export: glyph.export,
        has_components: !layer.components.is_empty(),
        is_empty: layer.is_empty() && !bitmap_preview,
        metrics_out_of_sync: glyph_status::metrics_out_of_sync(
            &document.font,
            glyph,
            master,
            outline_bbox,
        ),
        has_annotations: glyph_status::has_annotations(glyph),
        has_custom_glyph_info: glyph_status::has_custom_glyph_info(glyph),
        incompatible_masters: glyph_status::incompatible_masters(&document.font, glyph),
        locked: glyph.locked,
        generation: if bitmap_preview {
            document.generation
        } else {
            document.glyph_generation(gid)
        },
    })
}

fn build_detail(document: &Document, gid: GlyphId) -> Result<GlyphDetail, ApiError> {
    let name = document
        .glyph_name(gid)
        .ok_or_else(|| ApiError::not_found(format!("glyph handle {} does not exist", gid.0)))?;
    let mut glyph = document
        .font
        .glyph(name)
        .ok_or_else(|| ApiError::not_found(format!("glyph `{name}` does not exist")))?
        .clone();
    let (category, sub_category, script) = glyph_classification(&glyph);
    glyph.category = Some(category);
    glyph.sub_category = Some(sub_category);
    glyph.script = Some(script);
    let paths = glyph
        .layers
        .iter()
        .map(|layer| {
            fc_geom::flatten_layer_by_id(&document.font, name, &layer.id)
                .map_err(|e| ApiError::geometry(e.to_string()))
                .map(|path| LayerPath {
                    layer: layer.id.clone(),
                    flattened: rounded_svg(&path),
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(GlyphDetail { glyph, paths })
}

pub(crate) fn glyph_classification(glyph: &Glyph) -> (String, String, String) {
    let first_unicode = glyph.unicodes.first().copied();
    let category = glyph.category.clone().unwrap_or_else(|| {
        first_unicode
            .map(fc_glyphdata::category)
            .map(category_name)
            .unwrap_or("Unencoded")
            .to_owned()
    });
    let sub_category = glyph.sub_category.clone().unwrap_or_else(|| {
        first_unicode
            .map(fc_glyphdata::sub_category)
            .unwrap_or("Other")
            .to_owned()
    });
    let script = glyph.script.clone().unwrap_or_else(|| {
        first_unicode
            .map(fc_glyphdata::script)
            .unwrap_or("Zzzz")
            .to_owned()
    });
    (category, sub_category, script)
}

pub(crate) fn layer_master(layer: &Layer) -> Option<MasterId> {
    match &layer.kind {
        LayerKind::Master(master)
        | LayerKind::Background(master)
        | LayerKind::Brace { master, .. }
        | LayerKind::Smart { master, .. }
        | LayerKind::Bracket { master, .. } => Some(master.clone()),
        LayerKind::Backup { of, .. } => Some(of.clone()),
        LayerKind::Custom(_) => None,
    }
}

pub(crate) fn rounded_svg(path: &BezPath) -> String {
    let mut rounded = BezPath::new();
    for element in path.elements() {
        match *element {
            PathEl::MoveTo(point) => rounded.move_to(round_point(point)),
            PathEl::LineTo(point) => rounded.line_to(round_point(point)),
            PathEl::QuadTo(control, point) => {
                rounded.quad_to(round_point(control), round_point(point));
            }
            PathEl::CurveTo(control1, control2, point) => {
                rounded.curve_to(
                    round_point(control1),
                    round_point(control2),
                    round_point(point),
                );
            }
            PathEl::ClosePath => rounded.close_path(),
        }
    }
    rounded.to_svg()
}

fn round_point(point: Point) -> Point {
    Point::new(round_coordinate(point.x), round_coordinate(point.y))
}

fn round_coordinate(value: f64) -> f64 {
    let rounded = (value * 100.0).round() / 100.0;
    if rounded == 0.0 {
        0.0
    } else {
        rounded
    }
}

pub(crate) const fn category_name(category: Category) -> &'static str {
    match category {
        Category::Letter => "Letter",
        Category::Number => "Number",
        Category::Punctuation => "Punctuation",
        Category::Symbol => "Symbol",
        Category::Separator => "Separator",
        Category::Mark => "Mark",
        Category::Format => "Format",
        Category::Private => "Private",
        Category::Other => "Other",
    }
}
