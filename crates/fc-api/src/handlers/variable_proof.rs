//! Exact variable proofs with one bounded, current design-space cache.

use std::{collections::BTreeMap, sync::Mutex};

use fc_layout::{FontKey, ShapeRequest, Shaper};
use fc_model::{DesignLocation, Font};

use crate::{
    Api, ApiError, ProofFont, ProofFontParams, VariableProof, VariableProofGlyph,
    VariableProofParams,
};

/// One location and one most recent response; slider sweeps cannot retain fonts.
#[derive(Default)]
pub(crate) struct VariableProofCache(Mutex<Option<Entry>>);

struct Entry {
    doc: fc_doc::DocId,
    generation: u64,
    location: DesignLocation,
    user_location: BTreeMap<String, f64>,
    font: Font,
    shaper: Shaper,
    response: Option<(ShapeRequest, VariableProof)>,
}

pub(crate) fn preview(api: &Api, params: VariableProofParams) -> Result<VariableProof, ApiError> {
    if params.text.len() > 4096 || params.features.len() > 128 {
        return Err(ApiError::invalid_params(
            "variable proof exceeds text or feature limits",
        ));
    }
    // This lock only serializes proof requests, never edits or ordinary shaping.
    let mut cached = api
        .variable_proof
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let (generation, location, user_location, source) = api
        .session
        .with_doc(params.doc, |doc| {
            let location = resolve_location(&doc.font, &params)?;
            let user_location = doc
                .font
                .axes
                .iter()
                .map(|axis| {
                    (
                        axis.tag.to_string(),
                        axis.design_to_user(location[&axis.tag]),
                    )
                })
                .collect();
            let reusable = cached.as_ref().is_some_and(|entry| {
                entry.doc == params.doc
                    && entry.generation == doc.generation
                    && entry.location == location
            });
            Ok::<_, ApiError>((
                doc.generation,
                location,
                user_location,
                (!reusable).then(|| doc.font.clone()),
            ))
        })
        .map_err(ApiError::from)??;
    if let Some(source) = source {
        // Discard old data before compiling so even failures cannot grow the cache.
        *cached = None;
        let font = fc_compile::instantiate(&source, &location, None)
            .map_err(|error| ApiError::invalid_params(error.to_string()))?;
        *cached = Some(Entry {
            doc: params.doc,
            generation,
            location,
            user_location,
            font,
            shaper: Shaper::new(),
            response: None,
        });
    }
    let entry = cached
        .as_mut()
        .ok_or_else(|| ApiError::invalid_params("missing variable proof snapshot"))?;
    let request = ShapeRequest {
        text: params.text,
        features: params.features,
        script: params.script,
        language: params.language,
        direction: params.direction,
        size: None,
    };
    if let Some((previous, result)) = &entry.response {
        if previous == &request {
            return Ok(result.clone());
        }
    }
    let master = &entry.font.default_master().id;
    let run = entry
        .shaper
        .shape(
            &entry.font,
            &FontKey {
                doc: params.doc.0,
                master: master.clone(),
            },
            generation,
            &request,
        )
        .map_err(ApiError::from)?;
    let mut x = 0.;
    let mut y = 0.;
    let mut outlines: BTreeMap<String, String> = BTreeMap::new();
    let mut glyphs = Vec::with_capacity(run.glyphs.len());
    for (index, shaped) in run.glyphs.iter().enumerate() {
        let name = run.names.get(index).cloned().flatten();
        let path = if let Some(name) = &name {
            if let Some(path) = outlines.get(name) {
                path.clone()
            } else if let Some(glyph) = entry.font.glyph(name) {
                let layer = glyph
                    .master_layer(master)
                    .ok_or_else(|| ApiError::not_found("interpolated master layer is missing"))?;
                let outline = fc_geom::flatten_layer_by_id(&entry.font, &glyph.name, &layer.id)
                    .map_err(|error| ApiError::invalid_params(error.to_string()))?;
                let path = super::glyphs::rounded_svg(&outline);
                outlines.insert(name.clone(), path.clone());
                path
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        glyphs.push(VariableProofGlyph {
            name,
            path,
            x: x + shaped.x_offset,
            y: y + shaped.y_offset,
            x_advance: shaped.x_advance,
            y_advance: shaped.y_advance,
        });
        x += shaped.x_advance;
        y += shaped.y_advance;
    }
    let result = VariableProof {
        generation,
        location: entry.user_location.clone(),
        glyphs,
        units_per_em: run.units_per_em,
        warnings: run.warnings,
    };
    entry.response = Some((request, result.clone()));
    Ok(result)
}

fn resolve_location(font: &Font, params: &VariableProofParams) -> Result<DesignLocation, ApiError> {
    if let Some(index) = params.instance {
        let instance = font
            .instances
            .get(index as usize)
            .ok_or_else(|| ApiError::invalid_params("named instance no longer exists"))?;
        return font
            .complete_location(&instance.location)
            .map_err(|error| ApiError::invalid_params(error.to_string()));
    }
    if params.location.len() > 16
        || params
            .location
            .keys()
            .any(|tag| !font.axes.iter().any(|axis| axis.tag.to_string() == *tag))
    {
        return Err(ApiError::invalid_params("proof contains an unknown axis"));
    }
    let mut location = DesignLocation::new();
    for axis in &font.axes {
        let user = params
            .location
            .get(&axis.tag.to_string())
            .copied()
            .unwrap_or(axis.default);
        if !user.is_finite() || user < axis.min || user > axis.max {
            return Err(ApiError::invalid_params(
                "proof coordinate is outside the axis range",
            ));
        }
        location.insert(axis.tag, axis.user_to_design(user));
    }
    Ok(location)
}

/// Compiles one face of the live document as WOFF 2 for an embedded proof sheet.
///
/// The sheet needs the bytes in the browser, not a file on disk, so this returns
/// them base64 encoded. A named instance is interpolated; otherwise the selected
/// master — or the default one — is compiled statically, so the page shows what
/// an export of that face would actually do.
pub(crate) fn proof_font(api: &Api, params: ProofFontParams) -> Result<ProofFont, ApiError> {
    let (font, generations, generation) = api
        .session
        .with_doc(params.doc, |doc| {
            let generations: Vec<(fc_model::GlyphName, u64)> = doc
                .font
                .glyph_order()
                .map(|name| {
                    let generation = doc
                        .glyph_id(name)
                        .map_or(0, |gid| doc.glyph_generation(gid));
                    (name.clone(), generation)
                })
                .collect();
            (doc.font.clone(), generations, doc.generation)
        })
        .map_err(ApiError::from)?;
    let instance = match params.instance {
        Some(index) => {
            let position = usize::try_from(index)
                .map_err(|_| ApiError::invalid_params("named instance no longer exists"))?;
            if font.instances.get(position).is_none() {
                return Err(ApiError::invalid_params("named instance no longer exists"));
            }
            Some(position)
        }
        None => None,
    };
    let master = match &params.master {
        Some(id) => {
            if !font.masters.iter().any(|master| &master.id == id) {
                return Err(ApiError::not_found("selected master no longer exists"));
            }
            id.clone()
        }
        None => font.default_master().id.clone(),
    };
    let style_name = instance
        .and_then(|position| font.instances.get(position))
        .map(|named| named.name.clone())
        .or_else(|| {
            font.masters
                .iter()
                .find(|candidate| candidate.id == master)
                .map(|candidate| candidate.name.clone())
        })
        .unwrap_or_default();
    let options = fc_compile::ExportOptions {
        instance,
        format: fc_compile::ExportFormat::Woff2,
        master: Some(master),
        ..fc_compile::ExportOptions::default()
    };
    let generations: std::collections::HashMap<_, _> = generations.into_iter().collect();
    let report = fc_compile::export(
        &font,
        &|name: &fc_model::GlyphName| generations.get(name).copied().unwrap_or_default(),
        &options,
    )?;
    Ok(ProofFont {
        generation,
        woff2_base64: base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &report.bytes,
        ),
        family_name: font.info.family_name.clone(),
        style_name,
        units_per_em: font.info.units_per_em,
        warnings: report
            .warnings
            .into_iter()
            .map(|warning| warning.message)
            .collect(),
    })
}
