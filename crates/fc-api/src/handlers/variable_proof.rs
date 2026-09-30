//! Exact variable proofs with one bounded, current design-space cache.

use std::{collections::BTreeMap, sync::Mutex};

use fc_layout::{FontKey, ShapeRequest, Shaper};
use fc_model::{DesignLocation, Font};

use crate::{Api, ApiError, VariableProof, VariableProofGlyph, VariableProofParams};

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
