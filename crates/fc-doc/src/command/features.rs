//! Draft mutations intentionally cannot switch effective layout authority.

use fc_model::{FeatureDraft, FeatureGenerationSettings, Features, FeaturesSource, Font};

use crate::Error;

pub(super) fn set_draft(
    font: &mut Font,
    expected: u64,
    source: &Features,
    settings: &FeatureGenerationSettings,
) -> Result<(), Error> {
    check_revision(font, expected)?;
    if source.source_of_truth != FeaturesSource::Fea {
        return Err(Error::InvalidCommand(
            "feature drafts must contain FEA source; active authority cannot be changed by a draft"
                .into(),
        ));
    }
    let mut desired = if source == &font.features && settings == &font.feature_settings {
        None
    } else {
        Some(FeatureDraft {
            source: source.clone(),
            settings: settings.clone(),
            revision: font.feature_revision,
        })
    };
    if font
        .feature_draft
        .as_ref()
        .map(|draft| (&draft.source, &draft.settings))
        == desired
            .as_ref()
            .map(|draft| (&draft.source, &draft.settings))
    {
        return Ok(());
    }
    bump_revision(font)?;
    if let Some(draft) = &mut desired {
        draft.revision = font.feature_revision;
    }
    font.feature_draft = desired;
    Ok(())
}

pub(super) fn discard_draft(font: &mut Font, expected: u64) -> Result<(), Error> {
    check_revision(font, expected)?;
    if font.feature_draft.is_some() {
        bump_revision(font)?;
        font.feature_draft = None;
    }
    Ok(())
}

fn check_revision(font: &Font, expected: u64) -> Result<(), Error> {
    if font.feature_revision == expected {
        Ok(())
    } else {
        Err(Error::InvalidCommand(format!(
            "stale feature draft revision: expected {expected}, current {}",
            font.feature_revision
        )))
    }
}

pub(crate) fn bump_revision(font: &mut Font) -> Result<(), Error> {
    font.feature_revision = font.feature_revision.checked_add(1).ok_or_else(|| {
        Error::InvalidCommand(
            "feature revision is exhausted; cannot safely reuse a revision".into(),
        )
    })?;
    if let Some(draft) = &mut font.feature_draft {
        draft.revision = font.feature_revision;
    }
    Ok(())
}

pub(crate) fn is_draft(command: &super::Command) -> bool {
    match command {
        super::Command::SetFeatureDraft { .. } | super::Command::DiscardFeatureDraft { .. } => true,
        super::Command::Batch(commands) => commands.iter().all(is_draft),
        _ => false,
    }
}
