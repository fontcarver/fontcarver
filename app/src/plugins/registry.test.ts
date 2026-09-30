import { expect, it } from 'vitest'

import {
  bundledPlugin,
  bundledPlugins,
  curatedIndex,
  curatedNotice,
  defaultSettings,
  normalizeSetting,
  pluginEntry,
  readCuratedIndex,
} from './registry'

it('ships a curated index whose every entry carries complete provenance', () => {
  expect(curatedIndex.length).toBeGreaterThanOrEqual(3)
  expect(curatedNotice).toMatch(/never downloads|never download/i)
  for (const entry of curatedIndex) {
    expect(entry.origin).toBe('bundled')
    expect(entry.author.length).toBeGreaterThan(0)
    expect(entry.license).toMatch(/MIT|Apache/)
    expect(entry.source).toMatch(/^app\/src\/plugins\//)
    expect(entry.nameKey.startsWith('plugins.')).toBe(true)
    expect(entry.descriptionKey.startsWith('plugins.')).toBe(true)
    // The shipped implementation must exist with the same kind and version.
    const plugin = bundledPlugin(entry.id)
    expect(plugin?.manifest.kind).toBe(entry.kind)
    expect(plugin?.manifest.version).toBe(entry.version)
  }
  expect(bundledPlugins).toHaveLength(curatedIndex.length)
  expect(curatedIndex.filter((entry) => entry.kind === 'reporter').length).toBe(
    2,
  )
  expect(curatedIndex.filter((entry) => entry.kind === 'filter').length).toBe(1)
})

it('drops index entries with a wrong schema, no implementation or bad settings', () => {
  expect(readCuratedIndex(null)).toEqual([])
  expect(readCuratedIndex({ schema: 2, plugins: [] })).toEqual([])
  expect(readCuratedIndex({ schema: 1, plugins: 'no' })).toEqual([])
  const valid = {
    id: 'fontcarver.bounds',
    version: '1.0.0',
    kind: 'reporter',
    nameKey: 'plugins.boundsName',
    descriptionKey: 'plugins.boundsDescription',
    author: 'FontCarver project',
    license: 'MIT OR Apache-2.0',
    origin: 'bundled',
    source: 'app/src/plugins/reporters.ts',
    settings: [],
  }
  expect(readCuratedIndex({ schema: 1, plugins: [valid] })).toHaveLength(1)
  for (const change of [
    { id: 'remote.plugin' },
    { version: '9.9.9' },
    { kind: 'filter' },
    { origin: 'download' },
    { author: 1 },
    { license: null },
    { source: 2 },
    { settings: 'none' },
    {
      settings: [
        {
          key: '',
          labelKey: 'x',
          kind: 'number',
          default: 0,
          min: 0,
          max: 1,
          step: 1,
        },
      ],
    },
    {
      settings: [
        {
          key: 'a',
          labelKey: 'x',
          kind: 'other',
          default: 0,
          min: 0,
          max: 1,
          step: 1,
        },
      ],
    },
    {
      settings: [
        {
          key: 'a',
          labelKey: 'x',
          kind: 'number',
          default: 5,
          min: 0,
          max: 1,
          step: 1,
        },
      ],
    },
    {
      settings: [
        {
          key: 'a',
          labelKey: 'x',
          kind: 'number',
          default: 0,
          min: 0,
          max: 1,
          step: 0,
        },
      ],
    },
  ])
    expect(
      readCuratedIndex({ schema: 1, plugins: [{ ...valid, ...change }] }),
    ).toEqual([])
  // A duplicate id is listed once.
  expect(readCuratedIndex({ schema: 1, plugins: [valid, valid] })).toHaveLength(
    1,
  )
})

it('normalizes a setting into its declared range, step and kind', () => {
  const number = {
    key: 'dx',
    labelKey: 'plugins.dx',
    kind: 'number' as const,
    default: 0,
    min: -10,
    max: 10,
    step: 2,
  }
  expect(normalizeSetting(number, 3)).toBe(4)
  expect(normalizeSetting(number, 99)).toBe(10)
  expect(normalizeSetting(number, -99)).toBe(-10)
  expect(normalizeSetting(number, Number.NaN)).toBe(0)
  expect(normalizeSetting(number, 'text')).toBe(0)
  const toggle = { ...number, kind: 'toggle' as const, min: 0, max: 1, step: 1 }
  expect(normalizeSetting(toggle, 0.4)).toBe(0)
  expect(normalizeSetting(toggle, 0.6)).toBe(1)
})

it('exposes each entry and its declared defaults by id', () => {
  expect(
    pluginEntry('fontcarver.translate')?.settings.map((s) => s.key),
  ).toEqual(['dx', 'dy'])
  expect(defaultSettings('fontcarver.translate')).toEqual({ dx: 20, dy: 0 })
  expect(pluginEntry('unknown')).toBeUndefined()
  expect(defaultSettings('unknown')).toEqual({})
  expect(bundledPlugin('unknown')).toBeUndefined()
})

it('keeps a stepped value inside the declared range', () => {
  // Rounding to the step can push a clamped value back out of range.
  const setting = {
    key: 'dx',
    labelKey: 'plugins.dx',
    kind: 'number' as const,
    default: 0,
    min: -7,
    max: 7,
    step: 5,
  }
  expect(normalizeSetting(setting, 7)).toBeLessThanOrEqual(setting.max)
  expect(normalizeSetting(setting, 7)).toBeGreaterThanOrEqual(setting.min)
  expect(normalizeSetting(setting, -7)).toBeGreaterThanOrEqual(setting.min)
  expect(normalizeSetting(setting, 999)).toBe(5)
  expect(normalizeSetting(setting, -999)).toBe(-5)
})
