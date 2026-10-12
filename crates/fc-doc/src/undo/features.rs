use fc_model::{FeatureDraft, FeatureGenerationSettings, Features, Font, LayoutConversion};

/// Effective source, editor draft and conversion metadata captured together.
#[derive(Clone, Debug, PartialEq)]
pub struct FeatureSnapshot {
    source: Features,
    settings: FeatureGenerationSettings,
    draft: Option<FeatureDraft>,
    revision: u64,
    conversion: Option<LayoutConversion>,
}

impl FeatureSnapshot {
    pub(super) fn capture(font: &Font) -> Self {
        Self {
            source: font.features.clone(),
            settings: font.feature_settings.clone(),
            draft: font.feature_draft.clone(),
            revision: font.feature_revision,
            conversion: font.layout_conversion.clone(),
        }
    }

    pub(super) fn restore(&self, font: &mut Font) {
        font.features = self.source.clone();
        font.feature_settings = self.settings.clone();
        font.feature_draft = self.draft.clone();
        font.feature_revision = self.revision;
        font.layout_conversion = self.conversion.clone();
    }

    pub(super) fn estimated_bytes(&self) -> usize {
        // Include draft text and conversion backups in the undo budget rather
        // than counting just their small vector headers.
        serde_json::to_vec(&(&self.source, &self.settings, &self.draft, &self.conversion))
            .map_or(std::mem::size_of::<Self>(), |bytes| bytes.len())
    }
}
