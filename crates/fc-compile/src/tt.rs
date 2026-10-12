//! Export ownership and compiled-point checks for explicitly authored programs.

use fc_model::{Font, GlyphName, MasterId};
use fc_sfnt::tt::GlyphProgram;

use crate::{Error, ExportFormat, ExportOptions, ExportWarning};

pub(crate) fn authored(font: &Font, master: &MasterId) -> bool {
    authored_globals(font, master) || authored_glyphs(font, master)
}

/// Whether the master carries authored global programs, CVT and maxima.
///
/// Global data is geometry independent: it addresses control values and the
/// interpreter's state, not outline points, so geometry preprocessing and
/// variable compilation cannot invalidate it.
pub(crate) fn authored_globals(font: &Font, master: &MasterId) -> bool {
    font.master(master)
        .is_some_and(|m| m.tt.authored_units_per_em.is_some())
}

/// Whether any glyph in the master carries an explicitly authored program.
pub(crate) fn authored_glyphs(font: &Font, master: &MasterId) -> bool {
    font.glyphs.values().any(|glyph| {
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
    // Authored global programs, CVT and interpreter maxima do not address
    // outline points, so they survive every geometry option and variable
    // compilation; only authored glyph programs carry a point binding.
    let glyphs = if options.variable || options.instance.is_some() {
        font.masters
            .iter()
            .any(|master| authored_glyphs(font, &master.id))
    } else {
        authored_glyphs(
            font,
            options.master.as_ref().unwrap_or(&font.default_master().id),
        )
    };
    if !glyphs {
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
    if options.variable
        || options.instance.is_some()
        || options.remove_overlap
        || options.autohint
        || options.add_extrema
        || options.round_coordinates
        || options.decompose_components
    {
        return Err(Error::InvalidSource("authored TrueType programs currently require static master export without geometry preprocessing or automatic hinting; retain the source programs and choose compatible export options".into()));
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
    variable: bool,
) -> Result<(Vec<u8>, Vec<ExportWarning>), Error> {
    let data = &font
        .master(master)
        .ok_or_else(|| Error::MissingMaster(master.clone()))?
        .tt;
    let mut variation = Vec::new();
    let mut extra = Vec::new();
    if variable {
        variation = cvt_variation(font, master, data, &mut extra)?;
    }
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
    let bytes = fc_sfnt::tt::with_instructions(bytes, &programs, data, &variation)
        .map_err(|e| Error::InvalidSource(e.to_string()))?;
    let mut warnings = extra;
    if omitted != 0 {
        warnings.push(ExportWarning {
        code: "tt-imported-programs-omitted",
        message: format!("{omitted} imported TrueType programs could not be retained because their point bindings do not match the compiled outlines; authored programs were retained"),
    });
    }
    Ok((bytes, warnings))
}

/// Builds `cvar` regions from the masters' control value tables.
///
/// Every master's CVT must declare the same number of control values, because a
/// `cvar` delta addresses one control value by index. A source that does not is
/// reported and exported with the selected master's control values alone.
fn cvt_variation(
    font: &Font,
    master: &MasterId,
    data: &fc_model::TtMasterData,
    warnings: &mut Vec<ExportWarning>,
) -> Result<Vec<fc_sfnt::tt::CvtRegion>, Error> {
    use fontdrasil::{
        types::Tag,
        variations::{RoundingBehaviour, VariationModel},
    };
    if data.cvt.is_empty() || font.axes.is_empty() || font.masters.len() < 2 {
        return Ok(Vec::new());
    }
    let axis_order: Vec<_> = font.axes.iter().map(|axis| Tag::new(&axis.tag.0)).collect();
    let mut sources = std::collections::HashMap::new();
    for source in &font.masters {
        if source.tt.cvt.len() != data.cvt.len() {
            warnings.push(ExportWarning {
                code: "cvar-not-authored",
                message: format!("Master `{}` declares {} control values and the exported master declares {}; the control value table was written without variation", source.name, source.tt.cvt.len(), data.cvt.len()),
            });
            return Ok(Vec::new());
        }
        let location = font
            .complete_location(&source.location)
            .map_err(|error| Error::InvalidSource(error.to_string()))?;
        let values: Vec<f64> = source
            .tt
            .cvt
            .iter()
            .map(|value| f64::from(*value))
            .collect();
        sources.insert(
            fc_geom::variation::normalized_location(&font.axes, &location),
            values,
        );
    }
    if sources.len() < 2 {
        return Ok(Vec::new());
    }
    // The exported master must be the design default, or its control values are
    // not the ones the default instance would use.
    let default = font
        .design_default_master()
        .map_err(|error| Error::InvalidSource(error.to_string()))?;
    if &default.id != master {
        warnings.push(ExportWarning {
            code: "cvar-not-authored",
            message: "Control value variation is written from the design default master; the selected master's values were written without variation".to_owned(),
        });
        return Ok(Vec::new());
    }
    let model = VariationModel::new(sources.keys().cloned().collect(), axis_order.clone());
    let deltas = model
        .deltas_with_rounding::<f64, f64>(&sources, RoundingBehaviour::RoundTiesEven)
        .map_err(|error| Error::InvalidSource(error.to_string()))?;
    let mut regions = Vec::new();
    for (region, values) in deltas {
        if region.is_default() || values.iter().all(|value| *value == 0.0) {
            continue;
        }
        let deltas = values
            .iter()
            .map(|value| {
                let rounded = value.round();
                i16::try_from(rounded as i32).map_err(|_| {
                    Error::InvalidSource(
                        "a control value delta exceeds the signed 16-bit range".into(),
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        regions.push(fc_sfnt::tt::CvtRegion {
            region: axis_order
                .iter()
                .map(|tag| {
                    region.get(tag).map_or((0.0, 0.0, 0.0), |tent| {
                        (tent.min.to_f64(), tent.peak.to_f64(), tent.max.to_f64())
                    })
                })
                .collect(),
            deltas,
        });
    }
    Ok(regions)
}
