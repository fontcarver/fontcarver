//! Complete imported-layout proof and native-document restoration planning.

pub mod lifting;
mod shaping;

use fc_model::{
    FeatureAnchorState, FeatureBlock, FeatureGenerationSettings, FeatureKerningState, Features,
    FeaturesSource, Font, LayoutConversion,
};
use sha2::{Digest, Sha256};

use crate::{equivalent_layout, Error, LayoutImport};

/// Exact accepted source and model data covered by an all-master proof.
///
/// This runtime value is deliberately not serializable. A document transaction
/// must recheck its snapshot revisions before installing any of its fields.
#[derive(Clone, Debug)]
pub struct ProvenConversion {
    /// Source replacing all four imported layout-table authorities.
    pub source: Features,
    /// Explicit live model writer ownership, with name recipes disabled.
    pub settings: FeatureGenerationSettings,
    /// Persisted originals needed to undo later conversion-owned edits.
    pub backup: LayoutConversion,
    /// Proven replacement horizontal spacing, when lifting succeeded.
    pub kerning: Option<FeatureKerningState>,
    /// Proven replacement per-layer anchors, when lifting succeeded.
    pub anchors: Vec<FeatureAnchorState>,
    /// Whether all horizontal kerning moved into the model.
    pub kerning_lifted: bool,
    /// Whether complete supported mark lookups moved into the model.
    pub anchors_lifted: bool,
    /// Nonfatal details of source proof and optional model lifting.
    pub messages: Vec<String>,
}

/// Compile every master and prove the complete imported layout before activation.
///
/// Source conversion succeeds independently of optional model lifting. Failed
/// lift attempts retain the complete source and keep the corresponding writer
/// disabled. Neither this function nor its compiler calls mutate `font`.
pub fn prepare_conversion(font: &Font, prefer_model_lift: bool) -> Result<ProvenConversion, Error> {
    if font.features.source_of_truth != FeaturesSource::ImportedTables {
        return blocked("the font already uses editable source");
    }
    if font.feature_draft.is_some() {
        return blocked("discard the unapplied source draft before converting the original layout");
    }
    if font.masters.is_empty() || font.masters.len() > 32 {
        return blocked("conversion requires between 1 and 32 masters");
    }
    let imported = LayoutImport::from_font(font);
    if imported.tables.is_empty() {
        return blocked("the original layout tables are unavailable");
    }
    check_glyph_mapping(font, imported.glyph_order())?;
    check_original_digest(font)?;
    let source = Features {
        prefixes: vec![FeatureBlock {
            tag: "Imported layout".into(),
            code: imported.decompile()?,
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut candidate = font.clone();
    candidate.features = source;
    candidate.feature_draft = None;
    candidate.feature_settings = FeatureGenerationSettings::default();
    candidate.feature_settings.writer_policy.kerning = false;
    candidate.feature_settings.writer_policy.anchors = false;
    prove_candidate(font, &imported, &candidate)?;

    let mut backup = LayoutConversion {
        original_features: font.features.clone(),
        original_settings: font.feature_settings.clone(),
        original_glyph_order: imported.glyph_order().to_vec(),
        baseline_fingerprint: imported.fingerprint.clone(),
        ..Default::default()
    };
    let mut kerning = None;
    let mut anchors = Vec::new();
    let mut kerning_lifted = false;
    let mut anchors_lifted = false;
    let mut messages = vec![format!(
        "Complete layout compiled and compared for {} master(s)",
        font.masters.len()
    )];
    if prefer_model_lift {
        let lifted = lifting::lift_candidate(font, &candidate.features);
        messages.extend(lifted.report);
        if lifted.kerning_lifted || lifted.anchors_lifted {
            // The final combined candidate gets the same complete-table proof,
            // even when individual optional lifts already checked compilation.
            match prove_candidate(font, &imported, &lifted.candidate) {
                Ok(()) => {
                    candidate = lifted.candidate;
                    backup.lifted_kerning = lifted.original_kerning;
                    backup.lifted_anchors = lifted.original_anchors;
                    kerning = lifted.new_kerning;
                    anchors = lifted.new_anchors;
                    kerning_lifted = lifted.kerning_lifted;
                    anchors_lifted = lifted.anchors_lifted;
                }
                Err(error) => messages.push(format!(
                    "Model lifting was not accepted; complete editable source was retained: {error}"
                )),
            }
        }
    }
    Ok(ProvenConversion {
        source: candidate.features,
        settings: candidate.feature_settings,
        backup,
        kerning,
        anchors,
        kerning_lifted,
        anchors_lifted,
        messages,
    })
}

/// Check original-table provenance before constructing a restoration transaction.
///
/// This checks persisted raw bytes, original glyph order, current export flags,
/// and every anchor/master identity touched by conversion. Outline and metadata
/// edits are intentionally allowed and are never copied from the backup.
pub fn revert_eligibility(font: &Font) -> Result<&LayoutConversion, Error> {
    let backup = font.layout_conversion.as_ref().ok_or_else(|| {
        Error::LayoutConversion("no original-layout conversion backup is available".into())
    })?;
    if font.features.source_of_truth != FeaturesSource::Fea
        || backup.original_features.source_of_truth != FeaturesSource::ImportedTables
    {
        return blocked("the original-layout authority record is inconsistent");
    }
    check_glyph_mapping(font, &backup.original_glyph_order)?;
    check_original_digest(font)?;
    if backup.baseline_fingerprint.is_empty()
        || LayoutImport::fingerprint_for_font(font) != backup.baseline_fingerprint
    {
        return blocked("original layout tables or glyph-order provenance changed or are missing");
    }
    for state in &backup.lifted_anchors {
        if font
            .glyph(&state.glyph)
            .and_then(|glyph| glyph.layer(&state.layer))
            .is_none()
        {
            return blocked("a layer required to restore original anchors no longer exists");
        }
    }
    if backup.lifted_kerning.as_ref().is_some_and(|state| {
        state
            .masters
            .iter()
            .any(|record| font.master(&record.master).is_none())
            || state
                .memberships
                .iter()
                .any(|record| font.glyph(&record.glyph).is_none())
    }) {
        return blocked("a master or glyph required to restore original spacing no longer exists");
    }
    Ok(backup)
}

fn check_glyph_mapping(font: &Font, original: &[fc_model::GlyphName]) -> Result<(), Error> {
    if original.is_empty()
        || !font.glyph_order().eq(original.iter())
        || original
            .iter()
            .any(|name| font.glyph(name).is_none_or(|glyph| !glyph.export))
        || font
            .extra
            .source
            .as_ref()
            .is_none_or(|source| source.glyph_order != original)
    {
        return blocked("current exported glyph mapping differs from the original layout");
    }
    Ok(())
}

fn check_original_digest(font: &Font) -> Result<(), Error> {
    if let Some(source) = &font.extra.source {
        if let Some(bytes) = &source.original_bytes {
            if !source.sha256.is_empty()
                && format!("{:x}", Sha256::digest(bytes.as_slice())) != source.sha256
            {
                return blocked("original binary bytes do not match their provenance digest");
            }
        }
    }
    Ok(())
}

fn prove_candidate(
    original: &Font,
    imported: &LayoutImport,
    candidate: &Font,
) -> Result<(), Error> {
    if candidate.masters.is_empty() {
        return blocked("no master is available for conversion validation");
    }
    for master in &candidate.masters {
        let compiled =
            fc_compile::compile_static_with_metadata(candidate, &master.id, &Default::default())?;
        if compiled.glyph_order != imported.glyph_order() {
            return blocked("compilation changed the original glyph-ID mapping");
        }
        equivalent_layout(imported, &compiled.bytes, &compiled.glyph_order)?;
        shaping::compare(original, imported, &compiled, &master.id)?;
    }
    Ok(())
}

fn blocked<T>(reason: &str) -> Result<T, Error> {
    Err(Error::LayoutConversion(reason.into()))
}
