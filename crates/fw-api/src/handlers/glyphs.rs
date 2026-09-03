//! Glyph-grid filtering, sorting, and detail handlers.

use std::cmp::Ordering;

use fw_doc::Document;
use fw_glyphdata::Category;
use fw_model::{Glyph, GlyphId, Layer, LayerKind, MasterId};
use kurbo::{BezPath, PathEl, Point, Shape};

use crate::{
    Api, ApiError, GlyphCell, GlyphCellParams, GlyphDetail, GlyphDetailParams, GlyphFilter,
    GlyphListParams, GlyphListResult, GlyphSearchParams, GlyphSort, LayerPath,
};

const MAX_GLYPH_PAGE: u32 = 2_000;

struct Candidate {
    order: usize,
    gid: GlyphId,
    name: String,
    first_unicode: Option<u32>,
    width: f64,
}

pub(crate) fn list(api: &Api, params: GlyphListParams) -> Result<GlyphListResult, ApiError> {
    if params.limit > MAX_GLYPH_PAGE {
        return Err(ApiError::invalid_params(format!(
            "glyph list limit {} exceeds {MAX_GLYPH_PAGE}",
            params.limit
        )));
    }
    list_impl(api, params)
}

pub(crate) fn cell(api: &Api, params: GlyphCellParams) -> Result<GlyphCell, ApiError> {
    api.session
        .with_doc(params.doc, |document| {
            build_cell(document, params.gid, &params.master, true)
        })
        .map_err(ApiError::from)?
}

pub(crate) fn detail(api: &Api, params: GlyphDetailParams) -> Result<GlyphDetail, ApiError> {
    api.session
        .with_doc(params.doc, |document| build_detail(document, params.gid))
        .map_err(ApiError::from)?
}

pub(crate) fn search(api: &Api, params: GlyphSearchParams) -> Result<Vec<GlyphCell>, ApiError> {
    let result = list_impl(
        api,
        GlyphListParams {
            doc: params.doc,
            master: None,
            filter: GlyphFilter {
                query: Some(params.query),
                ..GlyphFilter::default()
            },
            sort: GlyphSort::Order,
            offset: 0,
            limit: params.limit,
            include_paths: false,
        },
    )?;
    Ok(result.items)
}

fn list_impl(api: &Api, params: GlyphListParams) -> Result<GlyphListResult, ApiError> {
    api.session
        .with_doc(params.doc, |document| {
            let master = selected_master(document, params.master.as_ref())?;
            let mut matches = Vec::new();
            for (order, name) in document.font.glyph_order().enumerate() {
                let glyph = document.font.glyph(name).ok_or_else(|| {
                    ApiError::internal(format!("glyph order references missing glyph `{name}`"))
                })?;
                let layer = glyph.master_layer(&master).ok_or_else(|| {
                    ApiError::not_found(format!(
                        "master layer `{master}` does not exist in glyph `{name}`"
                    ))
                })?;
                let gid = document.glyph_id(name).ok_or_else(|| {
                    ApiError::internal(format!("glyph `{name}` has no session handle"))
                })?;
                if matches_filter(glyph, layer, document.glyph_generation(gid), &params.filter) {
                    matches.push(Candidate {
                        order,
                        gid,
                        name: name.to_string(),
                        first_unicode: glyph.unicodes.first().copied(),
                        width: layer.width,
                    });
                }
            }
            sort_candidates(&mut matches, params.sort);
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

fn selected_master(
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

fn matches_filter(glyph: &Glyph, layer: &Layer, generation: u64, filter: &GlyphFilter) -> bool {
    if filter
        .query
        .as_deref()
        .is_some_and(|query| !query_matches(glyph, query))
    {
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
            .any(|allowed| allowed == first_unicode.map(fw_glyphdata::script).unwrap_or("Zzzz"))
    {
        return false;
    }
    if !filter.blocks.is_empty()
        && !filter.blocks.iter().any(|allowed| {
            allowed
                == first_unicode
                    .map(fw_glyphdata::block_name)
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
        .is_some_and(|expected| expected == layer.components.is_empty())
    {
        return false;
    }
    !filter.changed_only || generation > 0
}

fn category_matches(first_unicode: Option<u32>, allowed: &[String]) -> bool {
    let Some(unicode) = first_unicode else {
        return allowed.iter().any(|value| value == "Unencoded");
    };
    let category = category_name(fw_glyphdata::category(unicode));
    let sub_category = fw_glyphdata::sub_category(unicode);
    allowed
        .iter()
        .any(|value| value == category || value == sub_category)
}

fn query_matches(glyph: &Glyph, query: &str) -> bool {
    let query = query.trim();
    if query.is_empty() {
        return true;
    }
    let lower = query.to_lowercase();
    if let Some(digits) = lower.strip_prefix("u+") {
        return parse_scalar(digits).is_some_and(|unicode| glyph.unicodes.contains(&unicode));
    }
    if lower.starts_with("uni") {
        return fw_glyphdata::unicode_for_name(&lower)
            .is_some_and(|unicode| glyph.unicodes.contains(&unicode));
    }
    let mut characters = query.chars();
    if let (Some(character), None) = (characters.next(), characters.next()) {
        return glyph.unicodes.contains(&u32::from(character));
    }
    glyph.name.to_lowercase().contains(&lower)
}

fn parse_scalar(digits: &str) -> Option<u32> {
    let unicode = u32::from_str_radix(digits, 16).ok()?;
    char::from_u32(unicode).map(u32::from)
}

fn sort_candidates(candidates: &mut [Candidate], sort: GlyphSort) {
    candidates.sort_by(|left, right| match sort {
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

fn build_cell(
    document: &Document,
    gid: GlyphId,
    master: &MasterId,
    include_path: bool,
) -> Result<GlyphCell, ApiError> {
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
    let layer = glyph.master_layer(master).ok_or_else(|| {
        ApiError::not_found(format!(
            "master layer `{master}` does not exist in glyph `{name}`"
        ))
    })?;
    let flattened = document.flattened_path(gid, master)?;
    let bbox = (!flattened.elements().is_empty()).then(|| {
        let bounds = flattened.bounding_box();
        [bounds.x0, bounds.y0, bounds.x1, bounds.y1]
    });
    Ok(GlyphCell {
        gid,
        name: name.to_string(),
        unicodes: glyph.unicodes.clone(),
        width: layer.width,
        bbox,
        path: include_path.then(|| rounded_svg(&flattened)),
        color_label: glyph.color_label,
        export: glyph.export,
        has_components: !layer.components.is_empty(),
        is_empty: layer.is_empty(),
        generation: document.glyph_generation(gid),
    })
}

fn build_detail(document: &Document, gid: GlyphId) -> Result<GlyphDetail, ApiError> {
    let name = document
        .glyph_name(gid)
        .ok_or_else(|| ApiError::not_found(format!("glyph handle {} does not exist", gid.0)))?;
    let glyph = document
        .font
        .glyph(name)
        .ok_or_else(|| ApiError::not_found(format!("glyph `{name}` does not exist")))?
        .clone();
    let paths = glyph
        .layers
        .iter()
        .map(|layer| {
            flatten_detail_layer(document, layer).map(|path| LayerPath {
                layer: layer.id.clone(),
                flattened: rounded_svg(&path),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(GlyphDetail { glyph, paths })
}

fn flatten_detail_layer(document: &Document, layer: &Layer) -> Result<BezPath, ApiError> {
    let master = layer_master(layer).unwrap_or_else(|| document.font.default_master().id.clone());
    let mut path = BezPath::new();
    for contour in &layer.contours {
        path.extend(contour.to_bezpath().elements().iter().copied());
    }
    for component in &layer.components {
        let base = document.glyph_id(&component.base).ok_or_else(|| {
            ApiError::not_found(format!("glyph `{}` does not exist", component.base))
        })?;
        let flattened = document.flattened_path(base, &master)?;
        path.extend(
            (component.transform * flattened.as_ref())
                .elements()
                .iter()
                .copied(),
        );
    }
    Ok(path)
}

fn layer_master(layer: &Layer) -> Option<MasterId> {
    match &layer.kind {
        LayerKind::Master(master)
        | LayerKind::Background(master)
        | LayerKind::Brace { master, .. }
        | LayerKind::Bracket { master, .. } => Some(master.clone()),
        LayerKind::Backup { of, .. } => Some(of.clone()),
        LayerKind::Custom(_) => None,
    }
}

fn rounded_svg(path: &BezPath) -> String {
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
