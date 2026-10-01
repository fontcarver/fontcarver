//! Revision-checked CID identity, mapping and numeric glyph-order authoring.
use crate::{Api, ApiError, Change, CidApplyParams, CidState, DocParams};
use std::collections::BTreeMap;

pub(crate) fn state(api: &Api, params: DocParams) -> Result<CidState, ApiError> {
    api.session
        .with_doc(params.doc, |doc| CidState {
            generation: doc.generation,
            cid: doc.font.cid.clone(),
            glyphs: doc.font.glyph_order().map(ToString::to_string).collect(),
            // `otf::compile` orders `.notdef` plus the exporting glyphs, and
            // the CFF writer needs a CID for each of them.
            exported: doc
                .font
                .glyph_order()
                .filter(|name| doc.font.glyph(name).is_some_and(|glyph| glyph.export))
                .map(ToString::to_string)
                .collect(),
        })
        .map_err(ApiError::from)
}

pub(crate) fn apply(api: &Api, params: CidApplyParams) -> Result<Change, ApiError> {
    api.session
        .with_doc_mut(params.doc, |doc| {
            if params.expected_generation != doc.generation {
                return Err(ApiError::invalid_params(
                    "CID metadata changed; reload before applying edits",
                ));
            }
            let mut commands = Vec::new();
            if params.reorder {
                let Some(cid) = &params.cid else {
                    return Err(ApiError::invalid_params(
                        "CID ordering requires CID metadata",
                    ));
                };
                let reverse: BTreeMap<_, _> =
                    cid.cid_map.iter().map(|(id, name)| (name, *id)).collect();
                let mut order: Vec<_> = doc.font.glyph_order().cloned().collect();
                order.sort_by_key(|name| {
                    if name.as_ref() == ".notdef" {
                        0
                    } else {
                        reverse.get(name).copied().unwrap_or(u32::MAX)
                    }
                });
                commands.push(fc_doc::Command::SetGlyphOrder { order });
            }
            commands.insert(0, fc_doc::Command::SetCidInfo { cid: params.cid });
            doc.apply(fc_doc::Command::Batch(commands), params.label)
                .map_err(ApiError::from)
        })
        .map_err(ApiError::from)?
}
