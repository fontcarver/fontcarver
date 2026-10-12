import type {
  AutomaticEntryOption,
  FeatureGenerationSettings,
  FeatureGenerator,
  Features,
} from '../../api'

import { removeSource, sourceItems, type SourceItem } from './featureSources'

/** Recipes whose content is an explicit mapping list, not a name convention. */
const MAPPING_DRIVEN: readonly FeatureGenerator[] = [
  'Localized',
  'DiscretionaryLigatures',
]

/** The recipe that can own this entry, if the kernel offers one for its name. */
export function automaticEntryOption(
  options: readonly AutomaticEntryOption[],
  item: Pick<SourceItem, 'kind' | 'label'>,
): AutomaticEntryOption | undefined {
  if (item.kind === 'prefixes') return undefined
  const kind = item.kind === 'classes' ? 'Class' : 'Feature'
  return options.find(
    (option) => option.kind === kind && option.name === item.label,
  )
}

/**
 * Whether switching this entry to automatic is meaningful on its own.
 *
 * Localized and discretionary ligatures generate from explicit mapping lists,
 * so enabling them needs the mappings in the generation panel first.
 */
export function automaticEntrySwitchable(
  option: AutomaticEntryOption,
  settings: FeatureGenerationSettings,
): boolean {
  if (!MAPPING_DRIVEN.includes(option.generator)) return true
  return option.generator === 'Localized'
    ? settings.localized.length > 0
    : settings.discretionary_ligatures.length > 0
}

/** Turn on the recipe that owns `option`, without touching other entries. */
export function enableAutomaticRecipe(
  settings: FeatureGenerationSettings,
  option: AutomaticEntryOption,
): FeatureGenerationSettings {
  const next: FeatureGenerationSettings = {
    ...settings,
    manual_entries: settings.manual_entries.filter(
      (entry) => entry !== option.name,
    ),
  }
  switch (option.generator) {
    case 'SmallCaps':
      next.small_caps = true
      break
    case 'CapsToSmallCaps':
      next.caps_to_small_caps = true
      break
    case 'StylisticSets':
      next.stylistic_sets = true
      break
    case 'CaseForms':
      next.case_forms = true
      break
    case 'VerticalForms':
      next.vertical_forms = true
      break
    case 'Ligatures':
      next.ligatures = true
      break
    case 'FigureStyles':
      next.figure_styles = true
      break
    case 'PositionalFigures':
      next.positional_figures = true
      break
    case 'Fractions':
      next.fractions = true
      break
    case 'Ordinals':
      next.ordinals = true
      break
    case 'Composition':
      next.composition = true
      break
    case 'AccessAllAlternates':
      next.access_all_alternates = true
      break
    case 'Classes':
      next.automatic_classes = [
        ...new Set([...settings.automatic_classes, option.name]),
      ]
      break
    case 'Localized':
    case 'DiscretionaryLigatures':
      // Their mappings are the switch; the panel owns them.
      break
  }
  return next
}

/** Names of the family's other entries the user still owns by hand. */
function ownedSiblings(
  source: Features,
  settings: FeatureGenerationSettings,
  item: SourceItem,
  option: AutomaticEntryOption,
  options: readonly AutomaticEntryOption[],
): string[] {
  const generated = new Set(
    settings.generated.map((record) => record.source_id),
  )
  const owned = new Set(
    sourceItems(source)
      .filter((entry) => entry.id !== item.id && !generated.has(entry.id))
      .map(
        (entry) =>
          `${entry.kind === 'classes' ? 'Class' : 'Feature'}:${entry.label}`,
      ),
  )
  return options
    .filter(
      (candidate) =>
        candidate.generator === option.generator &&
        candidate.name !== option.name &&
        owned.has(`${candidate.kind}:${candidate.name}`),
    )
    .map((candidate) => candidate.name)
}

/**
 * The draft after switching one entry to automatic.
 *
 * Enabling a recipe turns on its whole family, so every other entry of that
 * family the user still owns is held manual in the same change. Without that,
 * a sibling such as a hand-written `onum` would collide with the recipe and
 * block the entry the user actually asked for. The user's own text for the
 * switched entry is removed so the recipe can materialize it; callers confirm
 * first when that text is not empty.
 */
export function switchEntryToAutomatic(
  source: Features,
  settings: FeatureGenerationSettings,
  item: SourceItem,
  option: AutomaticEntryOption,
  options: readonly AutomaticEntryOption[] = [],
): { source: Features; settings: FeatureGenerationSettings } {
  const enabled = enableAutomaticRecipe(settings, option)
  const held = ownedSiblings(source, settings, item, option, options)
  return {
    source: removeSource(source, item.id),
    settings: {
      ...enabled,
      manual_entries: [...new Set([...enabled.manual_entries, ...held])],
    },
  }
}
