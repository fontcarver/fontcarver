import { describe, expect, it } from 'vitest'

import {
  InvalidSearchError,
  planBatchRename,
  replaceGlyphNames,
} from './batchRename'

describe('batch glyph rename', () => {
  it('replaces every literal occurrence and omits unchanged names', () => {
    expect(
      replaceGlyphNames(['A.alt.alt', 'B.alt', 'C'], '.alt', '.ss01'),
    ).toEqual([
      { from: 'A.alt.alt', to: 'A.ss01.ss01' },
      { from: 'B.alt', to: 'B.ss01' },
    ])
    expect(replaceGlyphNames(['A'], '', 'B')).toEqual([])
    expect(replaceGlyphNames(['A'], 'A', 'A')).toEqual([])
  })

  it('treats replacement dollar expressions literally', () => {
    expect(replaceGlyphNames(['A.alt'], '.alt', '$&')).toEqual([
      { from: 'A.alt', to: 'A$&' },
    ])
  })

  it('rejects a destination occupied by an unchanged glyph', () => {
    const plan = planBatchRename(['A', 'A.alt'], [{ from: 'A.alt', to: 'A' }])
    expect(plan.command).toBeNull()
    expect(plan.issues).toEqual([{ kind: 'occupied', name: 'A' }])
  })

  it('rejects duplicate targets and invalid glyph names', () => {
    expect(
      planBatchRename(
        ['A', 'B'],
        [
          { from: 'A', to: 'C' },
          { from: 'B', to: 'C' },
        ],
      ).issues,
    ).toEqual([{ kind: 'duplicate', name: 'C' }])
    for (const to of ['', '9a', 'with space', 'é']) {
      expect(planBatchRename(['A'], [{ from: 'A', to }]).issues).toEqual([
        { kind: 'invalid', name: to },
      ])
    }
  })

  it('rejects a missing source without producing a command', () => {
    const plan = planBatchRename(['B'], [{ from: 'A', to: 'C' }])
    expect(plan.command).toBeNull()
    expect(plan.issues).toEqual([{ kind: 'missing', name: 'A' }])
  })

  it('moves all sources aside before assigning names for a swap', () => {
    const plan = planBatchRename(
      ['A', 'B', '_fcRename0'],
      [
        { from: 'A', to: 'B' },
        { from: 'B', to: 'A' },
      ],
    )
    expect(plan.issues).toEqual([])
    expect(plan.command).toEqual({
      Batch: [
        { RenameGlyph: { from: 'A', to: '_fcRename1' } },
        { RenameGlyph: { from: 'B', to: '_fcRename2' } },
        { RenameGlyph: { from: '_fcRename1', to: 'B' } },
        { RenameGlyph: { from: '_fcRename2', to: 'A' } },
      ],
    })
  })

  it('reserves destination names while allocating temporary names', () => {
    const plan = planBatchRename(
      ['A', 'B'],
      [
        { from: 'A', to: 'B' },
        { from: 'B', to: '_fcRename0' },
      ],
    )
    expect(plan.issues).toEqual([])
    expect(plan.command).toEqual({
      Batch: [
        { RenameGlyph: { from: 'A', to: '_fcRename1' } },
        { RenameGlyph: { from: 'B', to: '_fcRename2' } },
        { RenameGlyph: { from: '_fcRename1', to: 'B' } },
        { RenameGlyph: { from: '_fcRename2', to: '_fcRename0' } },
      ],
    })
  })

  it('does not create an empty undo action', () => {
    expect(planBatchRename(['A'], [{ from: 'A', to: 'A' }]).command).toBeNull()
  })
})

describe('regular-expression replacement', () => {
  it('replaces every match and inserts capture groups', () => {
    expect(
      replaceGlyphNames(['A.sc', 'B.sc', 'C'], '^(.)\\.sc$', '$1.small', {
        regex: true,
      }),
    ).toEqual([
      { from: 'A.sc', to: 'A.small' },
      { from: 'B.sc', to: 'B.small' },
    ])
    expect(replaceGlyphNames(['a_a_a'], 'a', 'b', { regex: true })).toEqual([
      { from: 'a_a_a', to: 'b_b_b' },
    ])
  })

  it('ignores case only when asked, in both modes', () => {
    expect(replaceGlyphNames(['Acute'], 'acute', 'grave')).toEqual([])
    expect(
      replaceGlyphNames(['Acute'], 'acute', 'grave', { ignoreCase: true }),
    ).toEqual([{ from: 'Acute', to: 'grave' }])
    // A literal search stays literal even when matching loosely.
    expect(replaceGlyphNames(['a.sc'], '.', 'X', { ignoreCase: true })).toEqual(
      [{ from: 'a.sc', to: 'aXsc' }],
    )
    expect(replaceGlyphNames(['a.sc'], '.', 'X', { regex: true })).toEqual([
      { from: 'a.sc', to: 'XXXX' },
    ])
  })

  it('reports a pattern that does not compile instead of throwing anonymously', () => {
    expect(() =>
      replaceGlyphNames(['A'], '(', 'x', { regex: true }),
    ).toThrowError(InvalidSearchError)
    try {
      replaceGlyphNames(['A'], '(', 'x', { regex: true })
    } catch (error) {
      expect((error as Error).message).toMatch(/group|parenthes/i)
    }
  })

  it('keeps an empty search inert and leaves unmatched names alone', () => {
    expect(replaceGlyphNames(['A'], '', 'x', { regex: true })).toEqual([])
    expect(replaceGlyphNames(['A'], 'Z+', 'x', { regex: true })).toEqual([])
  })
})
