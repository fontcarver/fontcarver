//! Bounded spacing-matrix and beam reads for the Text tab's Spacing panel.
//!
//! The panel needs, for every glyph in the shaped run at once, what the layer
//! stores, what its formulas resolve to and where its ink starts and ends at a
//! measurement height. Asking per glyph would be one request per cell on every
//! keystroke, so this answers the whole run in one bounded call at one document
//! generation.

use fc_doc::Document;
use fc_model::{GlyphName, MetricsKeys};
use kurbo::Shape;

use crate::{Api, ApiError, SpacingMatrix, SpacingMatrixParams, SpacingRow, SPACING_MATRIX_LIMIT};

pub(crate) fn spacing(api: &Api, params: SpacingMatrixParams) -> Result<SpacingMatrix, ApiError> {
    if params.glyphs.len() > SPACING_MATRIX_LIMIT {
        return Err(ApiError::invalid_params(format!(
            "a spacing request carries at most {SPACING_MATRIX_LIMIT} glyph names"
        )));
    }
    if params.beam.is_some_and(|height| !height.is_finite()) {
        return Err(ApiError::invalid_params(
            "the beam height must be a finite number of font units",
        ));
    }
    api.session
        .with_doc(params.doc, |document| {
            let master = super::glyphs::selected_master(document, params.master.as_ref())?;
            let rows = params
                .glyphs
                .iter()
                .map(|name| row(document, name, &master, params.beam))
                .collect();
            Ok(SpacingMatrix {
                generation: document.generation,
                master,
                rows,
            })
        })
        .map_err(ApiError::from)?
}

/// One row, reporting per-glyph failure in the row instead of failing the call.
///
/// A run legitimately contains glyphs that are missing from the document, have
/// no layer for the selected master or carry a cyclic formula. None of those is
/// a reason to refuse the whole matrix, so each is a field of its own row.
fn row(
    document: &Document,
    name: &str,
    master: &fc_model::MasterId,
    beam: Option<f64>,
) -> SpacingRow {
    let mut row = SpacingRow {
        glyph: name.to_owned(),
        layer: None,
        width: 0.0,
        lsb: None,
        rsb: None,
        glyph_keys: MetricsKeys::default(),
        layer_keys: None,
        resolved: None,
        key_error: None,
        out_of_sync: false,
        beam_left: None,
        beam_right: None,
        missing: true,
        locked: false,
    };
    let glyph_name = GlyphName::from(name);
    let font = &document.font;
    let Some(glyph) = font.glyph(&glyph_name) else {
        return row;
    };
    row.missing = false;
    row.locked = glyph.locked;
    let Some(layer) = glyph.master_layer(master) else {
        return row;
    };
    row.layer = Some(layer.id.clone());
    row.width = layer.width;
    if let Ok(path) = fc_geom::flatten_layer_by_id(font, &glyph_name, &layer.id) {
        if !path.is_empty() {
            let bounds = path.bounding_box();
            row.lsb = Some(bounds.x0);
            row.rsb = Some(layer.width - bounds.x1);
        }
    }
    row.glyph_keys = glyph.metrics_keys.clone();
    row.layer_keys = layer.metrics_keys.clone();
    match fc_doc::metrics::resolve(font, &glyph_name, master) {
        Ok(resolved) => {
            row.out_of_sync = resolved.source == fc_doc::MetricsSource::Key
                && [
                    (resolved.lsb, row.lsb.unwrap_or(0.0)),
                    (resolved.rsb, row.rsb.unwrap_or(layer.width)),
                    (resolved.width, layer.width),
                ]
                .into_iter()
                .any(|(resolved, stored)| {
                    !resolved.is_finite() || (resolved - stored).abs() > 1e-6
                });
            row.resolved = Some(resolved);
        }
        Err(error) => {
            row.key_error = Some(error.to_string());
            row.out_of_sync = true;
        }
    }
    if let Some(height) = beam {
        if let Ok(Some((left, right))) =
            fc_doc::metrics::measure_beam(font, &glyph_name, master, None, height)
        {
            row.beam_left = Some(left);
            row.beam_right = Some(right);
        }
    }
    row
}
