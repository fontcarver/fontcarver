import { describe, expect, it } from 'vitest'

import { fuzzyMatch, fuzzyRank } from './fuzzy'

describe('fuzzyMatch', () => {
  it('matches a subsequence and rejects a non-subsequence', () => {
    expect(fuzzyMatch('of', 'Open Font…')).not.toBeNull()
    expect(fuzzyMatch('zzz', 'Open Font…')).toBeNull()
  })

  it('matches everything for an empty query', () => {
    expect(fuzzyMatch('', 'anything')).toEqual({ score: 0, indexes: [] })
  })

  it('scores word-start and adjacent matches higher', () => {
    const wordStarts = fuzzyMatch('of', 'Open Font…')?.score ?? 0
    const scattered = fuzzyMatch('of', 'Toggle Inspector Frame')?.score ?? 0

    expect(wordStarts).toBeGreaterThan(scattered)
  })
})

describe('fuzzyRank', () => {
  it('ranks better matches first and drops non-matches', () => {
    const items = ['Toggle Sidebar', 'Save As…', 'Open Font…', 'Save']

    expect(fuzzyRank('sav', items, (item) => item)).toEqual([
      'Save',
      'Save As…',
    ])
  })
})
