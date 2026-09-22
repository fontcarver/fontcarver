//! Export-only source preparation; the editable document is never changed.

use fc_model::{Contour, Font, MasterId};

use crate::Error;

/// Prepares a selected static master for PostScript output on a document copy.
///
/// Overlap removal runs before automatic hinting. It expands components and clears
/// point-bound hint programs; automatic hinting then infers fresh stems and private
/// parameters. Callers should disclose cleared hints when rehinting is disabled.
/// The source font, other master layers and document history remain unchanged.
pub fn prepare_postscript_export(
    font: &Font,
    master: &MasterId,
    remove_overlap: bool,
    autohint: bool,
) -> Result<Font, Error> {
    if font.master(master).is_none() {
        return Err(Error::MissingMaster(master.clone()));
    }
    let prepared = if remove_overlap {
        overlap(font, master)?
    } else {
        font.clone()
    };
    if autohint {
        ps_autohint(&prepared, master)
    } else {
        Ok(prepared)
    }
}

pub(crate) fn overlap(font: &Font, master: &MasterId) -> Result<Font, Error> {
    let mut prepared = font.clone();
    let mut points = 0usize;
    for glyph in font.glyphs.values().filter(|g| g.export) {
        let path = fc_model::flatten_layer(font, &glyph.name, master)
            .map_err(|e| Error::InvalidSource(e.to_string()))?;
        points = points.saturating_add(path.elements().len());
        if points > 8_000_000 {
            return Err(Error::Limit("overlap removal path budget"));
        }
        let contours = Contour::from_bezpath(&path);
        let contours = fc_geom::remove_overlap(&contours, fc_geom::CurveKind::Cubic, 0.01)
            .map_err(|e| Error::InvalidSource(format!("{}: {e}", glyph.name)))?;
        let target = prepared
            .glyph_mut(&glyph.name)
            .and_then(|g| g.master_layer_mut(master))
            .ok_or_else(|| Error::InvalidSource("missing overlap target".into()))?;
        target.contours = contours;
        target.components.clear();
        target.tt_instructions = None;
        target.tt_binding = None;
        target.hints = Default::default();
    }
    Ok(prepared)
}

pub(crate) fn ps_autohint(font: &Font, master: &MasterId) -> Result<Font, Error> {
    let names = font
        .glyphs
        .values()
        .filter(|g| g.export)
        .map(|g| g.name.clone())
        .collect::<Vec<_>>();
    let (hints, private) = fc_hint::ps::analyze(font, master, &names)
        .map_err(|e| Error::InvalidSource(e.to_string()))?;
    let mut prepared = font.clone();
    for (name, hints) in hints {
        if let Some(layer) = prepared
            .glyph_mut(&name)
            .and_then(|g| g.master_layer_mut(master))
        {
            layer.hints = hints;
        }
    }
    if let Some(master) = prepared.masters.iter_mut().find(|m| m.id == *master) {
        master.private_dict = private;
    }
    Ok(prepared)
}
