import { describe, expect, it } from 'vitest'
import { parseGlyphNames, mapPasteTargets } from './crossDocumentModel'

describe('cross document selection', () => {
  it('preserves source order and rejects duplicate glyphs', () => {
    expect(parseGlyphNames('A, B\nC')).toEqual(['A', 'B', 'C'])
    expect(() => parseGlyphNames('A B A')).toThrow('duplicate')
  })
  it('maps an explicit destination selection one-to-one', () => {
    expect(mapPasteTargets([7, 2], [12, 9], 'selected')).toEqual([
      { source: 7, target: 12 },
      { source: 2, target: 9 },
    ])
    expect(() => mapPasteTargets([7, 2], [12], 'selected')).toThrow('count')
    expect(() => mapPasteTargets([7], [], 'components')).toThrow('count')
  })
  it('delegates same-name resolution to the kernel', () => {
    expect(mapPasteTargets([7, 2], [], 'same_name')).toEqual([
      { source: 7, target: null },
      { source: 2, target: null },
    ])
  })
})
