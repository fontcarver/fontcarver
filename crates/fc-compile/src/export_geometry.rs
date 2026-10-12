//! Static geometry transformations on disposable export copies.

use std::collections::{HashMap, HashSet};

use fc_model::{Contour, Font, GlyphName, Layer, MasterId, TtIndex};
use kurbo::Affine;

use crate::{Error, ExportFormat, ExportOptions};

const MAX_POINTS: usize = 8_000_000;

pub(crate) fn requested(options: &ExportOptions) -> bool {
    options.add_extrema || options.round_coordinates || options.decompose_components
}

pub(crate) fn validate_options(options: &ExportOptions) -> Result<(), Error> {
    if options.subroutinize && (options.variable || options.format != ExportFormat::Otf) {
        return Err(Error::InvalidSource(
            "PostScript subroutinization currently requires static CFF1 OTF output".into(),
        ));
    }
    if options.variable && requested(options) {
        return Err(Error::InvalidSource(
            "export geometry options require a static master or instance; prepare compatible source masters before variable export".into(),
        ));
    }
    Ok(())
}

/// Applies static export geometry options to a copy of the selected master.
///
/// Resolve live geometry and static instances before calling this helper. Components
/// are decomposed first, then missing curve extrema are inserted, then outline
/// points, component translation offsets, advances and vertical origins are rounded
/// to whole font units.
/// Referenced non-exporting base glyphs are included. Other master layers remain
/// unchanged. Point-bound hints are cleared on changed layers and their component
/// dependents; the editable source is never mutated.
///
/// Type 1 callers should run this before [`crate::prepare_postscript_export`] so
/// overlap removal and automatic hinting see the prepared outlines.
///
/// # Errors
///
/// Returns an error for a missing master, unsupported subroutinization or variable
/// geometry options, unresolved live geometry, malformed outlines or components,
/// or an exceeded outline-work budget.
pub fn prepare_geometry_export(
    font: &Font,
    master: &MasterId,
    options: &ExportOptions,
) -> Result<Font, Error> {
    validate_options(options)?;
    if font.master(master).is_none() {
        return Err(Error::MissingMaster(master.clone()));
    }
    if !requested(options) {
        return Ok(font.clone());
    }
    let names = selected_glyphs(font, master)?;
    let mut prepared = font.clone();
    let mut changed = HashSet::new();
    let mut dependents = HashMap::<GlyphName, Vec<GlyphName>>::new();
    let mut points = 0usize;
    for name in &names {
        let layer = font
            .glyph(name)
            .and_then(|glyph| glyph.master_layer(master))
            .ok_or_else(|| Error::InvalidSource(format!("{name}: missing export master layer")))?;
        if layer.has_live_geometry() {
            return Err(Error::InvalidSource(format!(
                "{name}: resolve live geometry before applying export geometry options"
            )));
        }
        validate_layer(layer, &mut points)?;
        for component in &layer.components {
            dependents
                .entry(component.base.clone())
                .or_default()
                .push(name.clone());
        }
        let target = prepared
            .glyph_mut(name)
            .and_then(|glyph| glyph.master_layer_mut(master))
            .ok_or_else(|| Error::InvalidSource(format!("{name}: missing export target layer")))?;
        if options.decompose_components && !layer.components.is_empty() {
            let path = fc_model::flatten_layer(font, name, master)
                .map_err(|error| Error::InvalidSource(format!("{name}: {error}")))?;
            target.contours = Contour::from_bezpath(&path);
            target.components.clear();
        }
        if options.add_extrema {
            target.contours = fc_geom::add_extrema(&target.contours);
        }
        if options.round_coordinates {
            target.contours = fc_geom::round_to_grid(&target.contours, 1.0);
            target.width = target.width.round();
            target.vertical_width = target.vertical_width.map(f64::round);
            target.vertical_origin = target.vertical_origin.map(f64::round);
            for component in &mut target.components {
                let mut coefficients = component.transform.as_coeffs();
                coefficients[4] = coefficients[4].round();
                coefficients[5] = coefficients[5].round();
                component.transform = Affine::new(coefficients);
            }
        }
        validate_layer(target, &mut points)?;
        if layer.contours != target.contours
            || layer.components != target.components
            || layer.width != target.width
            || layer.vertical_width != target.vertical_width
            || layer.vertical_origin != target.vertical_origin
        {
            changed.insert(name.clone());
        }
    }
    // A composite program addresses its resolved base points, so a base edit also
    // invalidates the program even if the component's own transform is unchanged.
    let mut pending = changed.iter().cloned().collect::<Vec<_>>();
    while let Some(base) = pending.pop() {
        for name in dependents.get(&base).into_iter().flatten() {
            if changed.insert(name.clone()) {
                pending.push(name.clone());
            }
        }
    }
    for name in changed {
        if let Some(layer) = prepared
            .glyph_mut(&name)
            .and_then(|glyph| glyph.master_layer_mut(master))
        {
            layer.hints = Default::default();
            layer.tt_instructions = None;
            layer.tt_binding = None;
            for point in layer
                .contours
                .iter_mut()
                .flat_map(|contour| &mut contour.points)
            {
                point.tt_index = TtIndex::Unnumbered;
            }
        }
    }
    Ok(prepared)
}

fn selected_glyphs(font: &Font, master: &MasterId) -> Result<HashSet<GlyphName>, Error> {
    let mut pending = font
        .glyphs
        .values()
        .filter(|glyph| glyph.export)
        .map(|glyph| glyph.name.clone())
        .collect::<Vec<_>>();
    let mut names = HashSet::new();
    while let Some(name) = pending.pop() {
        if !names.insert(name.clone()) {
            continue;
        }
        let layer = font
            .glyph(&name)
            .and_then(|glyph| glyph.master_layer(master))
            .ok_or_else(|| Error::InvalidSource(format!("{name}: missing export master layer")))?;
        pending.extend(
            layer
                .components
                .iter()
                .map(|component| component.base.clone()),
        );
    }
    Ok(names)
}

fn validate_layer(layer: &Layer, points: &mut usize) -> Result<(), Error> {
    for contour in &layer.contours {
        *points = points.saturating_add(contour.points.len());
        if *points > MAX_POINTS {
            return Err(Error::Limit("export geometry point budget"));
        }
        let path = contour
            .try_to_bezpath()
            .map_err(|error| Error::InvalidSource(error.to_string()))?;
        if !path.is_finite() {
            return Err(Error::InvalidSource(
                "nonfinite export outline coordinate".into(),
            ));
        }
    }
    if layer
        .components
        .iter()
        .flat_map(|component| component.transform.as_coeffs())
        .any(|coordinate| !coordinate.is_finite())
    {
        return Err(Error::InvalidSource(
            "nonfinite export component transform".into(),
        ));
    }
    Ok(())
}
