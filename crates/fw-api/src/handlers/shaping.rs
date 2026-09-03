//! Text-shaping handler backed by `fw-layout`.

use fw_layout::{FontKey, ShapeRequest, ShapedRun};

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
    // Shaping reads the model in place: copying a large font on every keystroke
    // would cost far more than the shaping itself.
    api.session
        .with_doc(params.doc, |document| {
            let master = match &params.master {
                Some(master) => master.clone(),
                None => document.font.default_master().id.clone(),
            };
            api.shaper
                .shape(
                    &document.font,
                    &FontKey {
                        doc: params.doc.0,
                        master,
                    },
                    document.generation,
                    &request,
                )
                .map_err(ApiError::from)
        })
        .map_err(ApiError::from)?
}
