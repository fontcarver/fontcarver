import { describe, expect, it } from 'vitest'

import { planBatchRename, replaceGlyphNames } from './batchRename'

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
