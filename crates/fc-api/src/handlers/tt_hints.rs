//! Compile outside the session lock, then recheck the revision before committing.

use fc_compile::{compile_static_with_metadata, CompileOptions, StaticCompile};
use fc_doc::{command::HintCommand, Command};
use fc_model::{Font, Layer, TtInstructionBinding, TtMasterData};

use crate::{
    Api, ApiError, Change, TtHintApplyParams, TtHintDraft, TtHintPreview, TtHintPreviewParams,
    TtHintPreviewSize, TtHintState, TtHintStateParams, TtPointView, TtProgramSource,
};

fn invalid(error: impl std::fmt::Display) -> ApiError {
    ApiError::invalid_params(error.to_string())
}

fn snapshot(
    api: &Api,
    target: &TtHintStateParams,
    expected: Option<u64>,
) -> Result<(Font, u64), ApiError> {
    api.session
        .with_doc(target.doc, |doc| {
            if expected.is_some_and(|value| value != doc.generation) {
                return Err(invalid(
                    "TrueType sources changed; reload before applying or previewing",
                ));
            }
            source_layer(&doc.font, target)?;
            Ok((doc.font.clone(), doc.generation))
        })
        .map_err(ApiError::from)?
}

fn source_layer<'a>(font: &'a Font, target: &TtHintStateParams) -> Result<&'a Layer, ApiError> {
    if font.master(&target.master).is_none() {
        return Err(ApiError::not_found("TrueType master does not exist"));
    }
    let glyph = font
        .glyph(&target.glyph)
        .ok_or_else(|| ApiError::not_found("TrueType glyph does not exist"))?;
    if !glyph.export {
        return Err(invalid("TrueType authoring requires an exported glyph"));
    }
    glyph
        .master_layer(&target.master)
        .ok_or_else(|| ApiError::not_found("TrueType master layer does not exist"))
}

fn compiled(
    font: &Font,
    target: &TtHintStateParams,
) -> Result<(StaticCompile, u16, fc_sfnt::tt::GlyphBasis), ApiError> {
    let result = compile_static_with_metadata(font, &target.master, &CompileOptions::default())
        .map_err(invalid)?;
    let id = result
        .glyph_order
        .iter()
        .position(|name| name.as_ref() == target.glyph)
        .and_then(|index| u16::try_from(index).ok())
        .ok_or_else(|| invalid("glyph is missing from compiled TrueType output"))?;
    let basis = fc_sfnt::tt::point_basis(&result.bytes, id).map_err(invalid)?;
    Ok((result, id, basis))
}

fn program(bytes: &[u8]) -> TtProgramSource {
    let (assembly, error) = match fc_hint::tt::disassemble(bytes) {
        Ok(source) => (Some(source), None),
        Err(error) => (None, Some(error.to_string())),
    };
    TtProgramSource {
        bytes: bytes.to_vec(),
        assembly,
        error,
    }
}

pub(crate) fn state(api: &Api, target: TtHintStateParams) -> Result<TtHintState, ApiError> {
    let (font, generation) = snapshot(api, &target, None)?;
    let layer = source_layer(&font, &target)?;
    let (compiled, id, basis) = compiled(&font, &target)?;
    let master = font
        .master(&target.master)
        .ok_or_else(|| invalid("missing TrueType master"))?;
    Ok(TtHintState {
        generation,
        layer: layer.id.to_string(),
        binding_matches: layer
            .tt_binding
            .as_ref()
            .is_some_and(|binding| binding.fingerprint == basis.fingerprint),
        authored: layer
            .tt_binding
            .as_ref()
            .is_some_and(|binding| binding.authored),
        program: program(layer.tt_instructions.as_deref().unwrap_or_default()),
        globals: master.tt.clone(),
        fpgm: program(&master.tt.fpgm),
        prep: program(&master.tt.prep),
        path: fc_raster::compiled_outline(&compiled.bytes, id)
            .map_err(invalid)?
            .path
            .to_svg(),
        points: basis
            .points
            .into_iter()
            .map(|point| TtPointView {
                position: point.position,
                on_curve: point.on_curve,
                contour: point.contour,
            })
            .collect(),
        fingerprint: basis.fingerprint,
        phantom_start: basis.phantom_start,
        horizontal_phantoms: basis.horizontal_phantoms,
        vertical_phantoms: basis.vertical_phantoms,
        component_metrics: basis.component_metrics,
        units_per_em: font.info.units_per_em,
    })
}

fn prepare(
    font: &Font,
    target: &TtHintStateParams,
    draft: &TtHintDraft,
) -> Result<(Vec<u8>, TtMasterData, u16), ApiError> {
    let program = fc_hint::tt::assemble(&draft.program).map_err(invalid)?;
    let mut globals = draft.globals.clone();
    globals.fpgm = fc_hint::tt::assemble(&draft.fpgm).map_err(invalid)?;
    globals.prep = fc_hint::tt::assemble(&draft.prep).map_err(invalid)?;
    globals.authored_units_per_em = Some(font.info.units_per_em);
    globals.max_size_of_instructions = None;
    fc_hint::tt::validate_globals(&globals).map_err(invalid)?;
    let (_, id, basis) = compiled(font, target)?;
    if basis.fingerprint != draft.fingerprint {
        return Err(invalid(
            "TrueType point identity changed; review the current compiled points",
        ));
    }
    Ok((program, globals, id))
}

pub(crate) fn apply(api: &Api, params: TtHintApplyParams) -> Result<Change, ApiError> {
    let (font, generation) = snapshot(api, &params.target, Some(params.expected_generation))?;
    let (program, data, _) = prepare(&font, &params.target, &params.draft)?;
    let layer = source_layer(&font, &params.target)?.id.clone();
    let command = Command::Batch(vec![
        Command::Hints(HintCommand::Tt {
            glyph: params.target.glyph.into(),
            layer,
            program,
            fingerprint: params.draft.fingerprint,
        }),
        Command::Hints(HintCommand::TtGlobals {
            master: params.target.master,
            data,
        }),
    ]);
    api.session
        .with_doc_mut(params.target.doc, |doc| {
            if doc.generation != generation {
                return Err(invalid(
                    "TrueType sources changed while compiling; reload before applying",
                ));
            }
            doc.apply(command, params.label).map_err(ApiError::from)
        })
        .map_err(ApiError::from)?
}

pub(crate) fn preview(api: &Api, params: TtHintPreviewParams) -> Result<TtHintPreview, ApiError> {
    if params.ppems.is_empty()
        || params.ppems.len() > 8
        || params.ppems.iter().any(|size| !(1..=128).contains(size))
    {
        return Err(invalid(
            "TrueType preview requires one to eight pixel sizes in 1..=128",
        ));
    }
    let (mut font, generation) = snapshot(api, &params.target, Some(params.expected_generation))?;
    let (program, globals, id) = prepare(&font, &params.target, &params.draft)?;
    let layer = font
        .glyph_mut(&params.target.glyph)
        .and_then(|g| g.master_layer_mut(&params.target.master))
        .ok_or_else(|| invalid("missing TrueType layer"))?;
    layer.tt_instructions = Some(program);
    layer.tt_binding = Some(TtInstructionBinding {
        fingerprint: params.draft.fingerprint,
        authored: true,
    });
    font.masters
        .iter_mut()
        .find(|master| master.id == params.target.master)
        .ok_or_else(|| invalid("missing TrueType master"))?
        .tt = globals;
    let output = fc_compile::export(
        &font,
        &|_| 1,
        &fc_compile::ExportOptions {
            master: Some(params.target.master.clone()),
            preserve_untouched_tables: false,
            ..Default::default()
        },
    )
    .map_err(invalid)?;
    // Resolve the actual output GID by the compiled model order; names in APIs
    // remain model identities and are never interpreted as positional indices.
    let mut sizes = Vec::new();
    for ppem in params.ppems {
        let hinted = fc_raster::preview_glyph(&output.bytes, id, ppem, true, params.mono)
            .map_err(invalid)?;
        let unhinted = fc_raster::preview_glyph(&output.bytes, id, ppem, false, params.mono)
            .map_err(invalid)?;
        sizes.push(TtHintPreviewSize {
            ppem,
            hinted: hinted.path.to_svg(),
            unhinted: unhinted.path.to_svg(),
            hinted_advance: hinted.advance,
            unhinted_advance: unhinted.advance,
        });
    }
    Ok(TtHintPreview {
        generation,
        sizes,
        warnings: output
            .warnings
            .into_iter()
            .map(|warning| crate::DocWarning {
                code: warning.code.to_owned(),
                message: warning.message,
            })
            .collect(),
    })
}
