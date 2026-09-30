import { beforeEach, describe, expect, it, vi } from 'vitest'

import type { ExportOptions } from '../api'
import {
  MAX_EXPORT_PROFILES,
  useExportProfileStore,
} from './exportProfileStore'

const STORAGE_KEY = 'fontcarver.exportProfiles'

function options(): ExportOptions {
  const value = {
    format: 'Ttf' as const,
    variable: false,
    master: 'm1',
    master_subset: ['m1', 'm2'],
    bitmap_strike: [16, 1] as [number, number],
    remove_overlap: true,
    autohint: false,
    keep_tt_instructions: true,
    preserve_untouched_tables: true,
    production_names: false,
    table_subset: ['DSIG'],
    add_extrema: false,
    round_coordinates: false,
    decompose_components: false,
    subroutinize: false,
  }
  return value
}

beforeEach(() => {
  vi.restoreAllMocks()
  localStorage.clear()
  useExportProfileStore.getState().reload()
})

describe('export profiles', () => {
  it('persists versioned options-only snapshots and reloads independently', () => {
    const source = options()
    const saved = useExportProfileStore.getState().save(' Web ', source)
    source.master_subset?.push('m3')
    source.table_subset?.push('TEST')
    saved.options.bitmap_strike![0] = 48
    expect(useExportProfileStore.getState().profiles[0]?.options).toEqual(
      options(),
    )

    const record = JSON.parse(localStorage.getItem(STORAGE_KEY)!)
    expect(record.version).toBe(1)
    expect(Object.keys(record.profiles[0])).toEqual(['id', 'name', 'options'])
    expect(record.profiles[0].name).toBe('Web')
    useExportProfileStore.setState({ profiles: [] })
    useExportProfileStore.getState().reload()
    expect(useExportProfileStore.getState().profiles[0]?.options).toEqual(
      options(),
    )
    useExportProfileStore.getState().remove(saved.id)
    useExportProfileStore.getState().reload()
    expect(useExportProfileStore.getState().profiles).toEqual([])
  })

  it('rejects blank and duplicate names and limits the number of profiles', () => {
    const { save } = useExportProfileStore.getState()
    expect(() => save('  ', options())).toThrow('blankName')
    save('Web', options())
    expect(() => save(' web ', options())).toThrow('duplicateName')
    for (let index = 1; index < MAX_EXPORT_PROFILES; index += 1)
      save(`Profile ${index}`, options())
    expect(() => save('Overflow', options())).toThrow('limitReached')
    expect(useExportProfileStore.getState().profiles).toHaveLength(
      MAX_EXPORT_PROFILES,
    )
  })

  it.each([
    '{bad json',
    JSON.stringify({ version: 2, profiles: [] }),
    JSON.stringify({
      version: 1,
      profiles: [
        { id: 'x', name: 'Bad', options: { ...options(), format: 'Unknown' } },
      ],
    }),
    JSON.stringify({
      version: 1,
      profiles: [
        { id: 'x', name: 'Bad', options: { ...options(), variable: 'false' } },
      ],
    }),
    JSON.stringify({
      version: 1,
      profiles: [
        {
          id: 'x',
          name: 'Bad',
          options: { ...options(), bitmap_strike: [0, 1] },
        },
      ],
    }),
    JSON.stringify({
      version: 1,
      profiles: [
        { id: 'x', name: 'Bad', options: { ...options(), instance: -1 } },
      ],
    }),
    JSON.stringify({
      version: 1,
      profiles: [
        {
          id: 'x',
          name: 'Bad',
          options: { ...options(), table_subset: ['NOT A TAG'] },
        },
      ],
    }),
  ])(
    'reports invalid storage instead of exposing an unchecked profile: %s',
    (raw) => {
      localStorage.setItem(STORAGE_KEY, raw)
      useExportProfileStore.getState().reload()
      expect(useExportProfileStore.getState().profiles).toEqual([])
      expect(useExportProfileStore.getState().error).toBe('invalidStorage')
      expect(localStorage.getItem(STORAGE_KEY)).toBe(raw)
    },
  )

  it('defaults newly added processing options in older profiles', () => {
    const legacy = { ...options() } as Record<string, unknown>
    for (const key of [
      'add_extrema',
      'round_coordinates',
      'decompose_components',
      'subroutinize',
    ])
      delete legacy[key]
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        version: 1,
        profiles: [{ id: 'legacy', name: 'Older', options: legacy }],
      }),
    )
    useExportProfileStore.getState().reload()
    expect(useExportProfileStore.getState().error).toBeNull()
    expect(useExportProfileStore.getState().profiles[0]?.options).toEqual(
      options(),
    )
  })

  it('preserves a glyph selector and strips unrelated document and destination fields', () => {
    const source = {
      ...options(),
      glyph: 'A',
      doc: 99,
      path: '/fonts/secret.ttf',
    }
    useExportProfileStore.getState().save('EPS glyph', source)
    useExportProfileStore.getState().reload()
    expect(useExportProfileStore.getState().profiles[0]?.options).toEqual({
      ...options(),
      glyph: 'A',
    })
  })

  it('rejects duplicate stored names and ids instead of making ambiguous picker entries', () => {
    for (const other of [
      { id: 'a', name: 'Other' },
      { id: 'b', name: ' web ' },
    ]) {
      localStorage.setItem(
        STORAGE_KEY,
        JSON.stringify({
          version: 1,
          profiles: [
            { id: 'a', name: 'Web', options: options() },
            { ...other, options: options() },
          ],
        }),
      )
      useExportProfileStore.getState().reload()
      expect(useExportProfileStore.getState().profiles).toEqual([])
      expect(useExportProfileStore.getState().error).toBe('invalidStorage')
    }
  })

  it('rejects malformed processing flags without replacing saved settings', () => {
    useExportProfileStore.getState().save('Valid', options())
    const previous = localStorage.getItem(STORAGE_KEY)
    const malformed = {
      ...options(),
      add_extrema: null,
    } as unknown as ExportOptions
    expect(() =>
      useExportProfileStore.getState().save('Broken', malformed),
    ).toThrow('invalidOptions')
    expect(localStorage.getItem(STORAGE_KEY)).toBe(previous)
  })

  it('does not publish a save or deletion when writing to storage fails', () => {
    const saved = useExportProfileStore.getState().save('Keep', options())
    const record = localStorage.getItem(STORAGE_KEY)
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new DOMException('Full', 'QuotaExceededError')
    })
    expect(() =>
      useExportProfileStore.getState().save('Lost', options()),
    ).toThrow('storageUnavailable')
    expect(() => useExportProfileStore.getState().remove(saved.id)).toThrow(
      'storageUnavailable',
    )
    expect(
      useExportProfileStore.getState().profiles.map((profile) => profile.name),
    ).toEqual(['Keep'])
    expect(localStorage.getItem(STORAGE_KEY)).toBe(record)
  })

  it('reports denied storage reads', () => {
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new DOMException('Denied', 'SecurityError')
    })
    useExportProfileStore.getState().reload()
    expect(useExportProfileStore.getState().error).toBe('storageUnavailable')
  })
})
