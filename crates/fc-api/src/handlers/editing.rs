//! Outline reading, glyph creation, and metrics handlers for the editor.

use fc_doc::{Command, Document, ResolvedMetrics};
use fc_model::{
    query::{declared_curve_format, CurveFormat},
    Glyph, GlyphId, GlyphName, Layer, LayerId, MasterId,
};
use kurbo::{BezPath, PathEl, Point, Shape};

use crate::{
    handlers::{
        glyphs::{
            build_cell, glyph_classification, layer_master, matching_candidates, rounded_svg,
            sort_candidates,
        },
        recipe::{self, RecipeItem},
    },
    AddGlyphsParams, AddGlyphsResult, Api, ApiError, ComponentView, FlattenedComponents, GlyphCell,
    GlyphIdsParams, LayerCurveFormat, LayerOutline, LayerOutlineParams, MetricsResolveParams,
    PREVIEW_GLYPH_ID,
};

pub(crate) fn outline(api: &Api, params: LayerOutlineParams) -> Result<LayerOutline, ApiError> {
    api.session
        .with_doc(params.doc, |document| {
            let selection = Selection::resolve(document, params.gid, params.layer.as_ref())?;
            let components = component_views(document, &selection)?;
            let path =
                fc_geom::flatten_layer_by_id(&document.font, &selection.name, &selection.layer.id)
                    .map_err(|e| ApiError::geometry(e.to_string()))?;
            let bbox = (!path.is_empty()).then(|| {
                let b = path.bounding_box();
                [b.x0, b.y0, b.x1, b.y1]
            });
            let width = selection.layer.width;
            Ok(LayerOutline {
                layer: selection.layer.id.clone(),
                master: selection.master.clone(),
                curve_format: declared_curve_format(&document.font, Some(&selection.master)).map(
                    |format| match format {
                        CurveFormat::Quadratic => LayerCurveFormat::Quadratic,
                        CurveFormat::Cubic => LayerCurveFormat::Cubic,
                    },
                ),
                contours: selection.layer.contours.clone(),
                components,
                anchors: selection.layer.anchors.clone(),
                guides: selection.layer.guides.clone(),
                width,
                lsb: bbox.map_or(0.0, |bbox| bbox[0]),
                rsb: bbox.map_or(width, |bbox| width - bbox[2]),
                bbox,
                resolved_metrics: fc_doc::metrics::resolve_layer(
                    &document.font,
                    &selection.name,
                    &selection.master,
                    &selection.layer.id,
                )
                .unwrap_or(ResolvedMetrics {
                    lsb: bbox.map_or(0.0, |bbox| bbox[0]),
                    rsb: bbox.map_or(width, |bbox| width - bbox[2]),
                    width,
                    source: fc_doc::MetricsSource::Explicit,
                }),
            })
        })
        .map_err(ApiError::from)?
}

pub(crate) fn flattened_components(
    api: &Api,
    params: LayerOutlineParams,
) -> Result<FlattenedComponents, ApiError> {
    api.session
        .with_doc(params.doc, |document| {
            let selection = Selection::resolve(document, params.gid, params.layer.as_ref())?;
            let mut paths = Vec::new();
            let mut points = Vec::new();
            for component in &selection.layer.components {
                let path = component_path(document, &selection.master, component)?;
                collect_points(&path, &mut points);
                paths.push(rounded_svg(&path));
            }
            Ok(FlattenedComponents { paths, points })
        })
        .map_err(ApiError::from)?
}

pub(crate) fn ids(api: &Api, params: GlyphIdsParams) -> Result<Vec<GlyphId>, ApiError> {
    api.session
        .with_doc(params.doc, |document| {
            let master =
                crate::handlers::glyphs::selected_master(document, params.master.as_ref())?;
            let overlay = crate::handlers::glyphs::document_overlay(document)?;
            let mut matches =
                matching_candidates(document, &overlay, &master, &params.filter, None)?;
            sort_candidates(&mut matches, params.sort);
            Ok(matches.iter().map(|candidate| candidate.gid).collect())
        })
        .map_err(ApiError::from)?
}

pub(crate) fn metrics_resolve(
    api: &Api,
    params: MetricsResolveParams,
) -> Result<ResolvedMetrics, ApiError> {
    api.session
        .with_doc(params.doc, |document| {
            let master =
                crate::handlers::glyphs::selected_master(document, params.master.as_ref())?;
            let name = glyph_name(document, params.gid)?.clone();
            if let Some(layer) = &params.layer {
                fc_doc::metrics::resolve_layer(&document.font, &name, &master, layer)
            } else {
                fc_doc::metrics::resolve(&document.font, &name, &master)
            }
            .map_err(ApiError::from)
        })
        .map_err(ApiError::from)?
}

pub(crate) fn add(api: &Api, params: AddGlyphsParams) -> Result<AddGlyphsResult, ApiError> {
    let master = api
        .session
        .with_doc(params.doc, |document| {
            crate::handlers::glyphs::selected_master(document, params.master.as_ref())
        })
        .map_err(ApiError::from)??;
    let plan = api
        .session
        .with_doc(params.doc, |document| {
            plan_additions(document, &recipe::parse(&params.recipe))
        })
        .map_err(ApiError::from)?;
    // A dry run answers "what would this recipe do?" from the same grammar and
    // the same duplicate check as a real call, without applying anything.
    if params.dry_run {
        return Ok(AddGlyphsResult {
            added: plan.added.iter().map(preview_cell).collect(),
            skipped: plan.skipped,
        });
    }
    if !plan.commands.is_empty() {
        api.session
            .with_doc_mut(params.doc, |document| {
                document.apply(Command::Batch(plan.commands), "Add glyphs")
            })
            .map_err(ApiError::from)?
            .map_err(ApiError::from)?;
    }
    api.session
        .with_doc(params.doc, |document| {
            let overlay = crate::handlers::glyphs::document_overlay(document)?;
            let added = plan
                .added
                .iter()
                .filter_map(|(name, _)| document.glyph_id(name))
                .map(|gid| build_cell(document, &overlay, gid, &master, false))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(AddGlyphsResult {
                added,
                skipped: plan.skipped,
            })
        })
        .map_err(ApiError::from)?
}

struct Additions {
    commands: Vec<Command>,
    added: Vec<(GlyphName, Option<u32>)>,
    skipped: Vec<String>,
}

fn plan_additions(document: &Document, items: &[RecipeItem]) -> Additions {
    let mut commands = Vec::new();
    let mut added: Vec<(GlyphName, Option<u32>)> = Vec::new();
    let mut skipped = Vec::new();
    for item in items {
        match item {
            RecipeItem::Unknown(token) => skipped.push(token.clone()),
            RecipeItem::TooLarge(token) => skipped.push(format!("too-large: {token}")),
            RecipeItem::Glyph { name, unicode } => {
                if document.font.glyph(name).is_some()
                    || added.iter().any(|(planned, _)| planned == name)
                {
                    skipped.push(name.to_string());
                    continue;
                }
                let mut glyph = Glyph::new(name.clone());
                glyph.unicodes.extend(unicode);
                commands.push(Command::AddGlyph { glyph });
                added.push((name.clone(), *unicode));
            }
        }
    }
    Additions {
        commands,
        added,
        skipped,
    }
}

/// Builds the cell of a glyph a dry run would create.
///
/// The glyph does not exist yet, so it has no session handle, no outline and no
/// advance; only the identity a recipe determines is filled in. Callers key a
/// preview by its name, never by its [`PREVIEW_GLYPH_ID`] handle.
fn preview_cell((name, unicode): &(GlyphName, Option<u32>)) -> GlyphCell {
    let mut glyph = Glyph::new(name.clone());
    glyph.unicodes.extend(*unicode);
    let (category, sub_category, script) = glyph_classification(&glyph);
    GlyphCell {
        gid: PREVIEW_GLYPH_ID,
        name: name.to_string(),
        unicodes: glyph.unicodes,
        category,
        sub_category,
        script,
        width: 0.0,
        vertical_advance: None,
        vertical_origin: None,
        bbox: None,
        path: None,
        color_label: None,
        layer_color_label: None,
        export: glyph.export,
        has_components: false,
        is_empty: true,
        metrics_out_of_sync: false,
        has_annotations: false,
        has_custom_glyph_info: false,
        incompatible_masters: false,
        locked: false,
        placeholder: false,
        generation: 0,
    }
}

struct Selection<'a> {
    name: GlyphName,
    layer: &'a Layer,
    master: MasterId,
}

impl<'a> Selection<'a> {
    fn resolve(
        document: &'a Document,
        gid: GlyphId,
        layer: Option<&LayerId>,
    ) -> Result<Self, ApiError> {
        let name = glyph_name(document, gid)?.clone();
        let glyph = document
            .font
            .glyph(&name)
            .ok_or_else(|| ApiError::not_found(format!("glyph `{name}` does not exist")))?;
        let default_master = document.font.default_master().id.clone();
        let layer = match layer {
            Some(id) => glyph
                .layer(id)
                .ok_or_else(|| ApiError::not_found(format!("layer `{id}` does not exist")))?,
            None => glyph.master_layer(&default_master).ok_or_else(|| {
                ApiError::not_found(format!("glyph `{name}` has no default master layer"))
            })?,
        };
        let master = layer_master(layer).unwrap_or(default_master);
        Ok(Self {
            name,
            layer,
            master,
        })
    }
}

fn glyph_name(document: &Document, gid: GlyphId) -> Result<&GlyphName, ApiError> {
    document
        .glyph_name(gid)
        .ok_or_else(|| ApiError::not_found(format!("glyph handle {} does not exist", gid.0)))
}

fn component_views(
    document: &Document,
    selection: &Selection<'_>,
) -> Result<Vec<ComponentView>, ApiError> {
    selection
        .layer
        .components
        .iter()
        .map(|component| {
            let path = component_path(document, &selection.master, component)?;
            let bbox = (!path.elements().is_empty()).then(|| {
                let bounds = path.bounding_box();
                [bounds.x0, bounds.y0, bounds.x1, bounds.y1]
            });
            Ok(ComponentView {
                smart_axes: document
                    .font
                    .glyph(&component.base)
                    .filter(|g| !g.smart_axes.is_empty())
                    .map(|g| g.smart_axes.clone()),
                component: component.clone(),
                path: rounded_svg(&path),
                bbox,
            })
        })
        .collect()
}

fn component_path(
    document: &Document,
    master: &MasterId,
    component: &fc_model::Component,
) -> Result<BezPath, ApiError> {
    if component.smart.as_ref().is_some_and(|p| !p.is_empty())
        || document
            .font
            .glyph(&component.base)
            .is_some_and(|g| !g.smart_axes.is_empty())
    {
        return fc_geom::flatten_smart_component(
            &document.font,
            &component.base,
            master,
            component.smart.as_ref().unwrap_or(&Default::default()),
        )
        .and_then(|path| {
            let transformed = component.transform * path;
            if !transformed.is_finite() {
                return Err(fc_model::Error::InvalidDesignSpace(
                    "component transform overflows outline coordinates".into(),
                ));
            }
            Ok(transformed)
        })
        .map_err(|e| ApiError::invalid_params(e.to_string()));
    }
    let base = document
        .glyph_id(&component.base)
        .ok_or_else(|| ApiError::not_found(format!("glyph `{}` does not exist", component.base)))?;
    let flattened = document.flattened_path(base, master)?;
    let transformed = component.transform * flattened.as_ref();
    if !transformed.is_finite() {
        return Err(ApiError::invalid_params(
            "component transform overflows outline coordinates",
        ));
    }
    Ok(transformed)
}

fn collect_points(path: &BezPath, points: &mut Vec<[f64; 2]>) {
    let mut push = |point: Point| points.push([point.x, point.y]);
    for element in path.elements() {
        match *element {
            PathEl::MoveTo(point) | PathEl::LineTo(point) => push(point),
            PathEl::QuadTo(control, point) => {
                push(control);
                push(point);
            }
            PathEl::CurveTo(first, second, point) => {
                push(first);
                push(second);
                push(point);
            }
            PathEl::ClosePath => {}
        }
    }
}
