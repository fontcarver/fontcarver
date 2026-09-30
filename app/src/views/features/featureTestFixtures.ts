import type {
  FeatureGenerationSettings,
  FeatureState,
  FeatureValidationResult,
} from '../../api'

/** Minimal source fixture shared by Features component acceptance tests. */
export function featureSettings(): FeatureGenerationSettings {
  return {
    small_caps: false,
    stylistic_sets: false,
    case_forms: false,
    vertical_forms: false,
    ligatures: false,
    localized: [],
    discretionary_ligatures: [],
    caps_to_small_caps: false,
    figure_styles: false,
    positional_figures: false,
    fractions: false,
    ordinals: false,
    composition: false,
    access_all_alternates: false,
    automatic_classes: [],
    manual_entries: [],
    writer_policy: { kerning: true, anchors: true },
    generated: [],
    glyph_fingerprint: null,
  }
}

/** Two independently identified source blocks allow located navigation checks. */
export function featureState(): FeatureState {
  return {
    doc: 1,
    generation: 0,
    compile_revision: 0,
    draft_revision: 0,
    authority: 'Fea',
    active_source: {
      source_of_truth: 'Fea',
      prefixes: [
        {
          id: 'p1',
          tag: 'Setup',
          code: '# setup',
          automatic: false,
          disabled: false,
          notes: '',
        },
        {
          id: 'p2',
          tag: 'Unicode',
          code: '# 中文😀\nrule',
          automatic: false,
          disabled: false,
          notes: '',
        },
      ],
      classes: [],
      features: [],
    },
    active_settings: featureSettings(),
    draft: null,
    has_unapplied_changes: false,
    original_layout_available: false,
    generation_stale: false,
    automatic_entries: [
      { name: 'smcp', generator: 'SmallCaps', kind: 'Feature' },
      { name: 'Uppercase', generator: 'Classes', kind: 'Class' },
    ],
  }
}

/** Validation tracks the current saved candidate identity. */
export function featureValidation(
  state: FeatureState,
): FeatureValidationResult {
  return {
    doc: state.doc,
    generation: state.generation,
    compile_revision: state.compile_revision,
    draft_revision: state.draft_revision,
    valid: true,
    diagnostics: [],
    truncated: false,
    compiled_masters: ['m1'],
    generated_source: state.draft?.source ?? state.active_source,
    generated_settings: state.draft?.settings ?? state.active_settings,
  }
}
