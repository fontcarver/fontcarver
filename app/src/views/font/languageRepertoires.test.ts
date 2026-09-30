import { describe, expect, it } from 'vitest'

import {
  languageCoverage,
  languageRepertoires,
  scalarRecipe,
  scalarRanges,
} from './languageRepertoires'

describe('language letter repertoires', () => {
  it('expands base exemplar characters to both cases without duplicates', () => {
    const english = languageRepertoires.find((entry) => entry.id === 'en')!
    expect(english.scalars).toHaveLength(52)
    expect(english.scalars).toContain(0x41)
    expect(english.scalars).toContain(0x7a)
    const turkish = languageRepertoires.find((entry) => entry.id === 'tr')!
    expect(turkish.scalars).toEqual(
      expect.arrayContaining([0x49, 0x69, 0x130, 0x131]),
    )
  })

  it('counts assigned scalars once and returns only missing members', () => {
    expect(
      languageCoverage([0x41, 0x42, 0x43], [0x41, 0x41, 0x43, 0x44]),
    ).toEqual({ present: 2, total: 3, missing: [0x42] })
  })

  it('compresses only adjacent scalars into filter ranges', () => {
    expect(scalarRanges([0x61, 0x42, 0x41, 0x43, 0x42, 0x1f600])).toEqual([
      [0x41, 0x43],
      [0x61, 0x61],
      [0x1f600, 0x1f600],
    ])
    expect(scalarRanges([])).toEqual([])
  })

  it('creates explicit Unicode recipes, preserving supplementary scalars', () => {
    expect(scalarRecipe([0x41, 0x1f600])).toBe('U+0041 U+1F600')
  })
})
