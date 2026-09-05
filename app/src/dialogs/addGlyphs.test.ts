import { describe, expect, it } from 'vitest'

import { classifySkip, groupSkips } from './addGlyphs'

describe('recipe skip reasons', () => {
  it('reads the reason the kernel gives each skipped item', () => {
    // A range past the expansion cap is the one reason the kernel prefixes.
    expect(classifySkip('too-large: U+0000-U+10FFFF')).toBe('tooLarge')
    // Every planned glyph has a valid glyph name, so a bare one is a duplicate.
    expect(classifySkip('A')).toBe('present')
    expect(classifySkip('uni4E00')).toBe('present')
    expect(classifySkip('a.sc')).toBe('present')
    expect(classifySkip('f_i')).toBe('present')
    // Anything the grammar could not turn into a glyph is unknown.
    expect(classifySkip('12three')).toBe('unknown')
    expect(classifySkip('@charset:Klingon')).toBe('unknown')
    expect(classifySkip('汉字')).toBe('unknown')
  })

  it('groups a mixed skip list by reason, in reporting order', () => {
    expect(
      groupSkips(['A', '12three', 'too-large: A-\u{10FFFF}', 'B']),
    ).toEqual([
      { reason: 'present', items: ['A', 'B'] },
      { reason: 'unknown', items: ['12three'] },
      { reason: 'tooLarge', items: ['too-large: A-\u{10FFFF}'] },
    ])
  })

  it('reports nothing for an empty skip list', () => {
    expect(groupSkips([])).toEqual([])
  })
})
