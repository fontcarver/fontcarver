import { describe, expect, it } from 'vitest'

import type { AutomaticEntryOption, Features } from '../../api'

import {
  automaticEntryOption,
  automaticEntrySwitchable,
  enableAutomaticRecipe,
  switchEntryToAutomatic,
} from './automaticEntries'
import { featureSettings } from './featureTestFixtures'
import { sourceItems } from './featureSources'

const options: AutomaticEntryOption[] = [
  { name: 'smcp', generator: 'SmallCaps', kind: 'Feature' },
  { name: 'ss03', generator: 'StylisticSets', kind: 'Feature' },
  { name: 'locl', generator: 'Localized', kind: 'Feature' },
  { name: 'Figures', generator: 'Classes', kind: 'Class' },
]

const source = (): Features => ({
  source_of_truth: 'Fea',
  prefixes: [
    {
      id: 'p1',
      tag: 'Setup',
      code: 'languagesystem DFLT dflt;',
      automatic: false,
      disabled: false,
      notes: '',
    },
  ],
  classes: [{ id: 'c1', name: 'Figures', code: 'zero one', automatic: false }],
  features: [
    {
      id: 'f1',
      tag: 'smcp',
      code: 'sub a by a.sc;',
      automatic: false,
      disabled: false,
      notes: '',
    },
  ],
})

describe('per-entry automatic switch', () => {
  it('matches an entry to the recipe that can own its name and kind', () => {
    const items = sourceItems(source())
    const feature = items.find((item) => item.label === 'smcp')!
    const glyphClass = items.find((item) => item.kind === 'classes')!
    const prefix = items.find((item) => item.kind === 'prefixes')!
    expect(automaticEntryOption(options, feature)?.generator).toBe('SmallCaps')
    expect(automaticEntryOption(options, glyphClass)?.generator).toBe('Classes')
    // A prefix never has a recipe, and a class name is not a feature tag.
    expect(automaticEntryOption(options, prefix)).toBeUndefined()
    expect(
      automaticEntryOption(options, { kind: 'features', label: 'Figures' }),
    ).toBeUndefined()
    expect(
      automaticEntryOption(options, { kind: 'features', label: 'ss03' })
        ?.generator,
    ).toBe('StylisticSets')
  })

  it('refuses to switch a mapping-driven recipe on without its mappings', () => {
    const settings = featureSettings()
    const localized = options[2]
    expect(automaticEntrySwitchable(localized, settings)).toBe(false)
    expect(
      automaticEntrySwitchable(localized, {
        ...settings,
        localized: [{ script: 'latn', language: 'TRK', substitutions: [] }],
      }),
    ).toBe(true)
    expect(automaticEntrySwitchable(options[0], settings)).toBe(true)
  })

  it('enables exactly one recipe and clears the entry from manual holds', () => {
    const settings = { ...featureSettings(), manual_entries: ['smcp', 'ss03'] }
    const next = enableAutomaticRecipe(settings, options[0])
    expect(next.small_caps).toBe(true)
    expect(next.stylistic_sets).toBe(false)
    expect(next.manual_entries).toEqual(['ss03'])
    const classes = enableAutomaticRecipe(settings, options[3])
    expect(classes.automatic_classes).toEqual(['Figures'])
    expect(classes.small_caps).toBe(false)
  })

  it('drops the user text of the entry it hands to the recipe', () => {
    const items = sourceItems(source())
    const feature = items.find((item) => item.label === 'smcp')!
    const next = switchEntryToAutomatic(
      source(),
      featureSettings(),
      feature,
      options[0],
      options,
    )
    expect(next.source.features).toEqual([])
    expect(next.source.prefixes).toHaveLength(1)
    expect(next.source.classes).toHaveLength(1)
    expect(next.settings.small_caps).toBe(true)
  })

  it('holds the family members the user still owns manual in the same change', () => {
    const figures: AutomaticEntryOption[] = [
      { name: 'lnum', generator: 'FigureStyles', kind: 'Feature' },
      { name: 'onum', generator: 'FigureStyles', kind: 'Feature' },
      { name: 'tnum', generator: 'FigureStyles', kind: 'Feature' },
      { name: 'smcp', generator: 'SmallCaps', kind: 'Feature' },
    ]
    const withFigures = source()
    withFigures.features = [
      {
        id: 'f-lnum',
        tag: 'lnum',
        code: 'sub one by one.lf;',
        automatic: false,
        disabled: false,
        notes: '',
      },
      {
        id: 'f-onum',
        tag: 'onum',
        code: 'sub one by one.osf;',
        automatic: false,
        disabled: false,
        notes: '',
      },
    ]
    const lnum = sourceItems(withFigures).find((item) => item.label === 'lnum')!
    const next = switchEntryToAutomatic(
      withFigures,
      featureSettings(),
      lnum,
      figures[0],
      figures,
    )
    expect(next.settings.figure_styles).toBe(true)
    // `onum` stays the user's; `tnum` has no entry, so the recipe may own it.
    expect(next.settings.manual_entries).toEqual(['onum'])
    expect(next.source.features.map((block) => block.tag)).toEqual(['onum'])
    // A generated sibling is not "owned" and must not be held manual.
    const generatedSettings = {
      ...featureSettings(),
      generated: [
        { generator: 'FigureStyles' as const, source_id: 'f-onum', glyphs: [] },
      ],
    }
    expect(
      switchEntryToAutomatic(
        withFigures,
        generatedSettings,
        lnum,
        figures[0],
        figures,
      ).settings.manual_entries,
    ).toEqual([])
  })
})
