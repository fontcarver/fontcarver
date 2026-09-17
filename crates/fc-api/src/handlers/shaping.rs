//! Text-shaping handler backed by `fc-layout`.

use fc_layout::{widths_for_text, FontKey, ModelWidths, ShapePlan, ShapeRequest, ShapedRun};
use fc_model::Font;

use crate::{Api, ApiError, ShapeParams};

pub(crate) fn shape(api: &Api, params: ShapeParams) -> Result<ShapedRun, ApiError> {
    let request = ShapeRequest {
        text: params.text,
        features: params.features,
        script: params.script,
        language: params.language,
        direction: params.direction,
        size: params.size,
    };
    let (plan, font, widths) = prepare(api, &params.doc, params.master.as_ref(), &request.text)?;
    // Compiling and shaping happen outside the session lock: a first compile
    // takes seconds, and holding the lock would stall every edit of every open
    // document for that long.
    api.shaper
        .shape_planned(plan, font, &widths, &request)
        .map_err(ApiError::from)
}

/// Reads everything a shaping request needs while the document is held.
///
/// Only the plan and the model widths of the requested text are always
/// collected; the font is copied only when a compile is actually going to run,
/// so a burst of keystrokes copies it at most once per scheduled rebuild.
fn prepare(
    api: &Api,
    doc: &fc_doc::DocId,
    master: Option<&fc_model::MasterId>,
    text: &str,
) -> Result<(ShapePlan, Option<Font>, ModelWidths), ApiError> {
    api.session
        .with_doc(*doc, |document| {
            let master = match master {
                Some(master) => master.clone(),
                None => document.font.default_master().id.clone(),
            };
            if document.font.master(&master).is_none() {
                return Err(ApiError::from(fc_layout::Error::MissingMaster(master)));
            }
            let key = FontKey { doc: doc.0, master };
            let plan = api.shaper.plan(&key, document.compile_revision());
            let font = plan.needs_font().then(|| document.font.clone());
            let widths = widths_for_text(&document.font, &key.master, text);
            Ok((plan, font, widths))
        })
        .map_err(ApiError::from)?
}
