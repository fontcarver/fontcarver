//! Outline reading, glyph creation, and metrics handlers for the editor.

use fw_doc::{Command, Document, ResolvedMetrics};
use fw_model::{Glyph, GlyphId, GlyphName, Layer, LayerId, MasterId};
use kurbo::{BezPath, PathEl, Point, Rect, Shape};

use crate::{
    handlers::{
        glyphs::{build_cell, layer_master, matching_candidates, rounded_svg, sort_candidates},
        recipe::{self, RecipeItem},
    },
    AddGlyphsParams, AddGlyphsResult, Api, ApiError, ComponentView, FlattenedComponents,
    GlyphIdsParams, LayerOutline, LayerOutlineParams, MetricsResolveParams,
};

pub(crate) fn outline(api: &Api, params: LayerOutlineParams) -> Result<LayerOutline, ApiError> {
    api.session
        .with_doc(params.doc, |document| {
            let selection = Selection::resolve(document, params.gid, params.layer.as_ref())?;
            let components = component_views(document, &selection)?;
            let bbox = layer_bounds(selection.layer, &components);
            let width = selection.layer.width;
            Ok(LayerOutline {
                layer: selection.layer.id.clone(),
                master: selection.master.clone(),
                contours: selection.layer.contours.clone(),
                components,
                anchors: selection.layer.anchors.clone(),
                guides: selection.layer.guides.clone(),
                width,
                lsb: bbox.map_or(0.0, |bbox| bbox[0]),
                rsb: bbox.map_or(0.0, |bbox| width - bbox[2]),
                bbox,
                resolved_metrics: fw_doc::metrics::resolve(
                    &document.font,
                    &selection.name,
                    &selection.master,
                )?,
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
            let mut matches = matching_candidates(document, &master, &params.filter, None)?;
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
            fw_doc::metrics::resolve(&document.font, &name, &master).map_err(ApiError::from)
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
            let added = plan
                .added
                .iter()
                .filter_map(|name| document.glyph_id(name))
                .map(|gid| build_cell(document, gid, &master, false))
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
    added: Vec<GlyphName>,
    skipped: Vec<String>,
}

fn plan_additions(document: &Document, items: &[RecipeItem]) -> Additions {
    let mut commands = Vec::new();
    let mut added: Vec<GlyphName> = Vec::new();
    let mut skipped = Vec::new();
    for item in items {
        match item {
            RecipeItem::Unknown(token) => skipped.push(token.clone()),
            RecipeItem::TooLarge(token) => skipped.push(format!("too-large: {token}")),
            RecipeItem::Glyph { name, unicode } => {
                if document.font.glyph(name).is_some() || added.contains(name) {
                    skipped.push(name.to_string());
                    continue;
                }
                let mut glyph = Glyph::new(name.clone());
                glyph.unicodes.extend(unicode);
                commands.push(Command::AddGlyph { glyph });
                added.push(name.clone());
            }
        }
    }
    Additions {
        commands,
        added,
        skipped,
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
    component: &fw_model::Component,
) -> Result<BezPath, ApiError> {
    let base = document
        .glyph_id(&component.base)
        .ok_or_else(|| ApiError::not_found(format!("glyph `{}` does not exist", component.base)))?;
    let flattened = document.flattened_path(base, master)?;
    Ok(component.transform * flattened.as_ref())
}

/// Returns the exact bounds of a layer, components included.
fn layer_bounds(layer: &Layer, components: &[ComponentView]) -> Option<[f64; 4]> {
    let mut path = BezPath::new();
    for contour in &layer.contours {
        path.extend(contour.to_bezpath().elements().iter().copied());
    }
    let mut bounds = (!path.elements().is_empty()).then(|| path.bounding_box());
    for component in components.iter().filter_map(|view| view.bbox) {
        let rect = Rect::new(component[0], component[1], component[2], component[3]);
        bounds = Some(bounds.map_or(rect, |bounds| bounds.union(rect)));
    }
    bounds.map(|bounds| [bounds.x0, bounds.y0, bounds.x1, bounds.y1])
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
