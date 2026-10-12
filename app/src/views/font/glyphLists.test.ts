import { describe, expect, it } from 'vitest'

import { emptyFilter } from '../../stores/documentStore'

import {
  glyphListFilter,
  isGlyphListActive,
  parseGlyphList,
} from './glyphLists'

describe('named glyph lists', () => {
  it('imports newline or whitespace-separated names and keeps first occurrence order', () => {
    expect(parseGlyphList(' A\n B\t A\r\nA.alt  uni4E00\n')).toEqual([
      'A',
      'B',
      'A.alt',
      'uni4E00',
    ])
    expect(parseGlyphList(' \n\t')).toEqual([])
  })

  it('filters by exact names and treats an empty list as matching no glyphs', () => {
    expect(glyphListFilter(['A', 'A.alt']).glyph_names).toEqual(['A', 'A.alt'])
    expect(glyphListFilter([]).glyph_names).toEqual([])
    expect(isGlyphListActive(emptyFilter(), [])).toBe(false)
    expect(isGlyphListActive(glyphListFilter([]), [])).toBe(true)
  })

  it('stops showing a list active after another sidebar condition is applied', () => {
    const filter = glyphListFilter(['A', 'B'])
    expect(isGlyphListActive(filter, ['B', 'A'])).toBe(true)
    expect(
      isGlyphListActive({ ...filter, categories: ['Letter'] }, ['A', 'B']),
    ).toBe(false)
    expect(isGlyphListActive({ ...filter, query: 'A' }, ['A', 'B'])).toBe(false)
    expect(isGlyphListActive(filter, ['A'])).toBe(false)
  })
})
