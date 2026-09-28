//! Export ownership and compiled-point checks for explicitly authored programs.

use fc_model::{Font, GlyphName, MasterId};
use fc_sfnt::tt::GlyphProgram;

use crate::{Error, ExportFormat, ExportOptions, ExportWarning};

pub(crate) fn authored(font: &Font, master: &MasterId) -> bool {
    font.master(master)
        .is_some_and(|m| m.tt.authored_units_per_em.is_some())
        || font.glyphs.values().any(|glyph| {
            glyph.master_layer(master).is_some_and(|layer| {
                layer
                    .tt_binding
                    .as_ref()
                    .is_some_and(|binding| binding.authored)
            })
        })
}

pub(crate) fn selected_authored(font: &Font, options: &ExportOptions) -> bool {
    if options.variable || options.instance.is_some() {
        font.masters.iter().any(|master| authored(font, &master.id))
    } else {
        authored(
            font,
            options.master.as_ref().unwrap_or(&font.default_master().id),
        )
    }
}

pub(crate) fn preflight(font: &Font, options: &ExportOptions) -> Result<(), Error> {
    if options.format == ExportFormat::Otf || !selected_authored(font, options) {
        return Ok(());
    }
    if font
        .glyphs
        .values()
        .flat_map(|g| &g.layers)
        .any(fc_model::Layer::has_live_geometry)
    {
        return Err(Error::InvalidSource("authored TrueType programs require reviewing the compiled points after baking live layer effects".into()));
    }
    if options.variable || options.instance.is_some() || options.remove_overlap || options.autohint
    {
        return Err(Error::InvalidSource("authored TrueType programs currently require static master export without overlap removal or automatic hinting; retain the source programs and choose compatible export options".into()));
    }
    if font.glyphs.values().any(|glyph| {
        !glyph.smart_axes.is_empty()
            || glyph.layers.iter().any(|layer| {
                matches!(layer.kind, fc_model::LayerKind::Bracket { .. })
                    || layer
                        .components
                        .iter()
                        .any(|c| c.smart.as_ref().is_some_and(|s| !s.is_empty()))
            })
    }) {
        return Err(Error::InvalidSource("authored TrueType programs cannot currently export through smart or bracket source resolution".into()));
    }
    Ok(())
}

pub(crate) fn apply(
    bytes: &[u8],
    font: &Font,
    master: &MasterId,
    order: &[GlyphName],
    keep_imported: bool,
) -> Result<(Vec<u8>, Vec<ExportWarning>), Error> {
    let data = &font
        .master(master)
        .ok_or_else(|| Error::MissingMaster(master.clone()))?
        .tt;
    if data
        .authored_units_per_em
        .is_some_and(|upem| upem != font.info.units_per_em)
    {
        return Err(Error::InvalidSource("TrueType global program scale is stale; review the CVT and global programs after changing units per em".into()));
    }
    fc_hint::tt::validate_globals(data).map_err(|e| Error::InvalidSource(e.to_string()))?;
    let mut candidates = Vec::new();
    let mut omitted = 0;
    for (index, name) in order.iter().enumerate() {
        let Some(layer) = font.glyph(name).and_then(|g| g.master_layer(master)) else {
            continue;
        };
        let program = layer.tt_instructions.as_deref().unwrap_or_default();
        if program.is_empty() {
            continue;
        }
        match &layer.tt_binding {
            Some(binding) if binding.authored || keep_imported => {
                let id = u16::try_from(index)
                    .map_err(|_| Error::InvalidSource("too many TrueType glyphs".into()))?;
                candidates.push((id, name, binding, program));
            }
            _ if keep_imported => omitted += 1,
            _ => {}
        }
    }
    let ids: Vec<_> = candidates.iter().map(|c| c.0).collect();
    let bindings =
        fc_sfnt::tt::fingerprints(bytes, &ids).map_err(|e| Error::InvalidSource(e.to_string()))?;
    let mut programs = Vec::new();
    for ((id, name, binding, program), current) in candidates.into_iter().zip(bindings) {
        if current != binding.fingerprint {
            if binding.authored {
                return Err(Error::InvalidSource(format!("stale TrueType point binding for {name}; review its program against the current compiled points")));
            }
            omitted += 1;
            continue;
        }
        programs.push(GlyphProgram {
            glyph_id: id,
            fingerprint: current,
            bytes: program.to_vec(),
        });
    }
    // Recompiled outlines can reorder points used by retained raw layout tables.
    crate::otf_preservation::check_geometry_independent_layout(bytes)
        .and_then(|()| crate::otf_preservation_aat::check(bytes))
        .map_err(|error| {
            Error::InvalidSource(format!(
                "authored TrueType output cannot preserve this point-dependent layout: {error}"
            ))
        })?;
    let bytes = fc_sfnt::tt::with_instructions(bytes, &programs, data)
        .map_err(|e| Error::InvalidSource(e.to_string()))?;
    let warnings = if omitted == 0 {
        Vec::new()
    } else {
        vec![ExportWarning {
        code: "tt-imported-programs-omitted",
        message: format!("{omitted} imported TrueType programs could not be retained because their point bindings do not match the compiled outlines; authored programs were retained"),
    }]
    };
    Ok((bytes, warnings))
}
