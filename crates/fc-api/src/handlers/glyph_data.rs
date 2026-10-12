//! Versioned overlay editing, metadata refresh and database-driven creation.
use std::collections::{BTreeMap, BTreeSet};

use fc_doc::{command::glyph_data_overlay, Command};
use fc_glyphdata::{parse_overlay, resolve_info};
use serde_json::Value;

use crate::{
    Api, ApiError, Change, GenerateGlyphsParams, GenerateGlyphsResult, GeneratedGlyph,
    GlyphDataState, GlyphDataStateParams, SetGlyphDataOverlayParams, UpdateGlyphInfoParams,
};

pub(crate) fn state(api: &Api, params: GlyphDataStateParams) -> Result<GlyphDataState, ApiError> {
    api.session
        .with_doc(params.doc, |doc| {
            let name = doc
                .glyph_name(params.gid)
                .ok_or_else(|| ApiError::not_found("glyph handle is no longer present"))?;
            let glyph = doc
                .font
                .glyph(name)
                .ok_or_else(|| ApiError::not_found("glyph is missing"))?;
            let overlay = glyph_data_overlay(&doc.font.info)?;
            let info = resolve_info(name.as_ref(), &glyph.unicodes, &overlay);
            let raw_name = name.as_ref();
            let base = raw_name
                .char_indices()
                .find(|(index, character)| *index > 0 && *character == '.')
                .map_or(raw_name, |(index, _)| &raw_name[..index]);
            let custom = overlay.glyphs.contains_key(name.as_ref())
                || overlay.glyphs.contains_key(base)
                || glyph.unicodes != info.unicodes
                || glyph
                    .category
                    .as_ref()
                    .is_some_and(|value| Some(value) != info.category.as_ref())
                || glyph
                    .sub_category
                    .as_ref()
                    .is_some_and(|value| Some(value) != info.sub_category.as_ref())
                || glyph
                    .script
                    .as_ref()
                    .is_some_and(|value| Some(value) != info.script.as_ref())
                || glyph
                    .production_name
                    .as_ref()
                    .is_some_and(|value| Some(value) != info.production_name.as_ref());
            Ok(GlyphDataState {
                generation: doc.generation,
                overlay_json: serde_json::to_string_pretty(&overlay)
                    .map_err(|error| ApiError::internal(error.to_string()))?,
                info: info.into(),
                custom,
            })
        })
        .map_err(ApiError::from)?
}

pub(crate) fn set_overlay(
    api: &Api,
    params: SetGlyphDataOverlayParams,
) -> Result<Change, ApiError> {
    let text = if params.overlay_json.trim().is_empty() {
        r#"{"version":1,"glyphs":{}}"#
    } else {
        &params.overlay_json
    };
    let overlay = parse_overlay(text).map_err(ApiError::invalid_params)?;
    let value =
        serde_json::to_value(&overlay).map_err(|error| ApiError::internal(error.to_string()))?;
    api.session
        .with_doc_mut(params.doc, |doc| {
            check_generation(doc.generation, params.expected_generation)?;
            check_label(&params.label)?;
            let mut info = doc.font.info.clone();
            if info.user_data.is_null() {
                info.user_data = Value::Object(Default::default());
            }
            let data = info.user_data.as_object_mut().ok_or_else(|| {
                ApiError::invalid_params(
                    "font user data must be an object to store a glyph database overlay",
                )
            })?;
            data.insert(fc_doc::command::GLYPH_DATA_OVERLAY_KEY.into(), value);
            doc.apply(
                Command::SetFontInfo {
                    info: Box::new(info),
                },
                params.label,
            )
            .map_err(ApiError::from)
        })
        .map_err(ApiError::from)?
}

pub(crate) fn update(api: &Api, params: UpdateGlyphInfoParams) -> Result<Change, ApiError> {
    if params.glyphs.is_empty() || params.glyphs.len() > 65_536 {
        return Err(ApiError::invalid_params(
            "select between one and 65536 glyphs",
        ));
    }
    api.session
        .with_doc_mut(params.doc, |doc| {
            check_generation(doc.generation, params.expected_generation)?;
            check_label(&params.label)?;
            let glyphs = params
                .glyphs
                .iter()
                .map(|gid| {
                    doc.glyph_name(*gid)
                        .cloned()
                        .ok_or_else(|| ApiError::not_found("selected glyph no longer exists"))
                })
                .collect::<Result<BTreeSet<_>, _>>()?
                .into_iter()
                .collect();
            doc.apply(Command::UpdateGlyphInfo { glyphs }, params.label)
                .map_err(ApiError::from)
        })
        .map_err(ApiError::from)?
}

fn check_generation(current: u64, expected: u64) -> Result<(), ApiError> {
    if current != expected {
        return Err(ApiError::invalid_params(
            "glyph information changed; reload before applying edits",
        ));
    }
    Ok(())
}

fn check_label(label: &str) -> Result<(), ApiError> {
    if label.len() > 512 {
        return Err(ApiError::invalid_params("undo label exceeds 512 bytes"));
    }
    Ok(())
}

/// Creates the glyphs a recipe asks for, building composites from the database.
///
/// This is the "generate missing glyphs" path the sidebar and the Add Glyphs
/// dialog use. It shares `glyphs.add`'s recipe grammar and duplicate handling;
/// the difference is that a glyph whose database record carries a composition is
/// created as a composite with its marks placed from real anchor pairs, and that
/// everything is applied as one undo step. A member the document lacks leaves the
/// glyph blank and is reported, so nothing is silently dropped.
pub(crate) fn generate(
    api: &Api,
    params: GenerateGlyphsParams,
) -> Result<GenerateGlyphsResult, ApiError> {
    let master = api
        .session
        .with_doc(params.doc, |document| {
            crate::handlers::glyphs::selected_master(document, params.master.as_ref())
        })
        .map_err(ApiError::from)??;
    let plan = api
        .session
        .with_doc(params.doc, |document| -> Result<Plan, ApiError> {
            let overlay = glyph_data_overlay(&document.font.info)?;
            let mut plan = Plan::default();
            let mut staged: BTreeMap<fc_model::GlyphName, fc_model::Glyph> = BTreeMap::new();
            for item in super::recipe::parse(&params.recipe) {
                match item {
                    super::recipe::RecipeItem::Unknown(token) => plan.skipped.push(token),
                    super::recipe::RecipeItem::TooLarge(token) => {
                        plan.skipped.push(format!("too-large: {token}"));
                    }
                    super::recipe::RecipeItem::Glyph { name, unicode } => {
                        if document.font.glyph(&name).is_some() || staged.contains_key(&name) {
                            plan.skipped.push(name.to_string());
                            continue;
                        }
                        if staged.len() >= MAX_GENERATED {
                            plan.skipped.push(format!("too-many: {name}"));
                            continue;
                        }
                        if params.count_only {
                            // Nothing is shown per glyph, so nothing is built:
                            // only the name has to be reserved against the rest
                            // of the recipe.
                            plan.order.push(name.clone());
                            staged.insert(name, fc_model::Glyph::default());
                            continue;
                        }
                        let composed = fc_doc::command::composed_glyph(
                            &document.font,
                            &staged,
                            &name,
                            unicode.into_iter().collect(),
                            &overlay,
                        );
                        plan.composed.push(GeneratedGlyph {
                            name: name.to_string(),
                            components: composed
                                .components
                                .iter()
                                .map(ToString::to_string)
                                .collect(),
                            unavailable: composed
                                .unavailable
                                .iter()
                                .map(ToString::to_string)
                                .collect(),
                        });
                        plan.order.push(name.clone());
                        staged.insert(name, composed.glyph);
                    }
                }
            }
            plan.glyphs = plan
                .order
                .iter()
                .filter_map(|name| staged.get(name).cloned())
                .collect();
            Ok(plan)
        })
        .map_err(ApiError::from)??;
    if params.count_only {
        return Ok(GenerateGlyphsResult {
            added_count: u32::try_from(plan.order.len()).unwrap_or(u32::MAX),
            added: Vec::new(),
            skipped: plan.skipped,
            composed: Vec::new(),
        });
    }
    if params.dry_run {
        return Ok(GenerateGlyphsResult {
            added_count: u32::try_from(plan.glyphs.len()).unwrap_or(u32::MAX),
            added: plan
                .glyphs
                .iter()
                .map(|glyph| preview_cell(glyph, &master))
                .collect(),
            skipped: plan.skipped,
            composed: plan.composed,
        });
    }
    if !plan.glyphs.is_empty() {
        let commands = plan
            .glyphs
            .iter()
            .cloned()
            .map(|glyph| Command::AddGlyph { glyph })
            .collect();
        api.session
            .with_doc_mut(params.doc, |document| {
                document.apply(Command::Batch(commands), "Generate glyphs")
            })
            .map_err(ApiError::from)?
            .map_err(ApiError::from)?;
    }
    api.session
        .with_doc(params.doc, |document| {
            let overlay = super::glyphs::document_overlay(document)?;
            let added = plan
                .order
                .iter()
                .filter_map(|name| document.glyph_id(name))
                .map(|gid| super::glyphs::build_cell(document, &overlay, gid, &master, false))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(GenerateGlyphsResult {
                added_count: u32::try_from(added.len()).unwrap_or(u32::MAX),
                added,
                skipped: plan.skipped,
                composed: plan.composed,
            })
        })
        .map_err(ApiError::from)?
}

/// The most glyphs one generation request may create.
const MAX_GENERATED: usize = 8_192;

#[derive(Default)]
struct Plan {
    order: Vec<fc_model::GlyphName>,
    glyphs: Vec<fc_model::Glyph>,
    composed: Vec<GeneratedGlyph>,
    skipped: Vec<String>,
}

/// Builds the cell of a glyph a dry run would create; it has no session handle.
fn preview_cell(glyph: &fc_model::Glyph, master: &fc_model::MasterId) -> crate::GlyphCell {
    let (category, sub_category, script) = super::glyphs::glyph_classification(glyph);
    let layer = glyph.master_layer(master);
    crate::GlyphCell {
        gid: crate::PREVIEW_GLYPH_ID,
        name: glyph.name.to_string(),
        unicodes: glyph.unicodes.clone(),
        category,
        sub_category,
        script,
        width: layer.map_or(0., |layer| layer.width),
        vertical_advance: None,
        vertical_origin: None,
        bbox: None,
        path: None,
        color_label: None,
        layer_color_label: None,
        export: glyph.export,
        has_components: layer.is_some_and(|layer| !layer.components.is_empty()),
        is_empty: layer.is_none_or(fc_model::Layer::is_empty),
        metrics_out_of_sync: false,
        has_annotations: false,
        has_custom_glyph_info: false,
        incompatible_masters: false,
        locked: false,
        placeholder: false,
        generation: 0,
    }
}
