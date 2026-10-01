import { describe, expect, it } from 'vitest'

import type { CjkRadicalLibrary } from '../../../api'

import {
  componentEntries,
  entryChar,
  entryCodepoint,
  filterEntries,
  idsToken,
  pageGlyphNames,
  radicalEntries,
} from './cjkRadicalEntries'

/** Radicals 9 人 (with 亻) and 75 木, as the kernel derives them. */
const library: CjkRadicalLibrary = {
  unihan_version: '17.0.0',
  radicals: [
    {
      number: 9,
      kangxi: 0x2f08,
      kangxi_glyph: null,
      forms: [
        { unicode: 0x4eba, strokes: 2, glyph: 'uni4EBA' },
        { unicode: 0x4ebb, strokes: 2, glyph: null },
      ],
      members: 412,
    },
    {
      number: 75,
      kangxi: 0x2f4a,
      kangxi_glyph: 'kangxiTree',
      forms: [{ unicode: 0x6728, strokes: 4, glyph: 'wood' }],
      members: 1,
    },
  ],
  components: [
    { name: 'wood', uses: 2, unicode: 0x6728 },
    { name: 'stroke.left', uses: 1, unicode: null },
  ],
  components_truncated: false,
}

describe('radical library entries', () => {
  it('lists the block character first, then every written form', () => {
    const entries = radicalEntries(library)
    expect(entries.map((entry) => entry.key)).toEqual([
      'r9:k',
      'r9:20154',
      'r9:20155',
      'r75:k',
      'r75:26408',
    ])
    expect(entries[0]).toMatchObject({
      unicode: 0x2f08,
      glyph: null,
      radical: 9,
      kangxi: true,
      count: 412,
    })
    // The block character borrows the canonical form's stroke count.
    expect(entries[0]?.strokes).toBe(2)
    expect(entries[1]).toMatchObject({ unicode: 0x4eba, glyph: 'uni4EBA' })
    expect(entries[2]).toMatchObject({ unicode: 0x4ebb, glyph: null })
    expect(entryChar(entries[4]!)).toBe('木')
    expect(entryCodepoint(entries[4]!)).toBe('U+6728')
  })

  it('lists the fonts own component bases with their use counts', () => {
    const entries = componentEntries(library)
    expect(entries).toHaveLength(2)
    expect(entries[0]).toMatchObject({
      glyph: 'wood',
      unicode: 0x6728,
      count: 2,
      radical: null,
    })
    // A component with no encoding is still pickable by name.
    expect(entries[1]).toMatchObject({ glyph: 'stroke.left', unicode: null })
    expect(entryChar(entries[1]!)).toBeNull()
    expect(entryCodepoint(entries[1]!)).toBe('')
  })

  it('hides forms the font lacks when asked, and keeps them otherwise', () => {
    const entries = radicalEntries(library)
    expect(
      filterEntries(entries, '', true).map((entry) => entry.glyph),
    ).toEqual(['uni4EBA', 'kangxiTree', 'wood'])
    expect(filterEntries(entries, '', false)).toHaveLength(5)
  })

  it('matches a radical number, a stroke count, a character and a name', () => {
    const entries = radicalEntries(library)
    expect(
      filterEntries(entries, '75', false).map((entry) => entry.key),
    ).toEqual(['r75:k', 'r75:26408'])
    expect(
      filterEntries(entries, '4', false).map((entry) => entry.key),
    ).toEqual(['r75:k', 'r75:26408'])
    expect(
      filterEntries(entries, '亻', false).map((entry) => entry.key),
    ).toEqual(['r9:20155'])
    expect(
      filterEntries(entries, 'WOOD', false).map((entry) => entry.key),
    ).toEqual(['r75:26408'])
    expect(
      filterEntries(entries, 'u+2f08', false).map((entry) => entry.key),
    ).toEqual(['r9:k'])
    expect(filterEntries(entries, 'nothing', false)).toHaveLength(0)
  })

  it('references a present form by name and a missing one by character', () => {
    const entries = radicalEntries(library)
    expect(idsToken(entries[1]!)).toBe('[uni4EBA]')
    expect(idsToken(entries[2]!)).toBe('亻')
    expect(idsToken(entries[0]!)).toBe('⼈')
  })

  it('asks for each page glyph once', () => {
    const page = [...radicalEntries(library), ...componentEntries(library)]
    expect(pageGlyphNames(page)).toEqual([
      'uni4EBA',
      'kangxiTree',
      'wood',
      'stroke.left',
    ])
    expect(pageGlyphNames([])).toEqual([])
  })
})
