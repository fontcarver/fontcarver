//! Validation and source selection for variable exports.
use crate::Error;
use fc_model::{FeaturesSource, Font, LayerKind, MasterId, Tag};
use std::collections::HashSet;

pub(crate) fn validate_export(font: &Font) -> Result<(), Error> {
    font.validate_designspace()
        .map_err(|e| Error::InvalidSource(e.to_string()))?;
    if font.features.source_of_truth == FeaturesSource::ImportedTables
        && [Tag(*b"GSUB"), Tag(*b"GPOS"), Tag(*b"GDEF"), Tag(*b"BASE")]
            .iter()
            .any(|tag| font.extra.raw_tables.contains_key(tag))
    {
        return Err(Error::InvalidSource("convert imported layout to editable feature source before compiling a new variable design space".into()));
    }
    Ok(())
}

pub(crate) fn master_subset(font: &Font, subset: &[MasterId]) -> Result<Font, Error> {
    let ids: HashSet<_> = subset.iter().collect();
    if ids.is_empty() || ids.len() != subset.len() || ids.iter().any(|id| font.master(id).is_none())
    {
        return Err(Error::InvalidSource(
            "master subset must contain unique existing master IDs".into(),
        ));
    }
    let mut selected = font.clone();
    selected.masters.retain(|m| ids.contains(&m.id));
    for glyph in selected.glyphs.values_mut() {
        glyph.layers.retain(|layer| match &layer.kind {
            LayerKind::Master(id)
            | LayerKind::Background(id)
            | LayerKind::Smart { master: id, .. }
            | LayerKind::Brace { master: id, .. }
            | LayerKind::Bracket { master: id, .. }
            | LayerKind::Backup { of: id, .. } => ids.contains(id),
            LayerKind::Custom(_) => true,
        });
    }
    selected
        .validate_designspace()
        .map_err(|e| Error::InvalidSource(e.to_string()))?;
    Ok(selected)
}
