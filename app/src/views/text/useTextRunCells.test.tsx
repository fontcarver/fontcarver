import { renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { makeCell } from '../../test/fontFixtures'
import { useGlyphColors } from '../color/useGlyphColor'
import type { PositionedGlyph } from './textLayout'
import { useTextGlyphCells } from './useTextGlyphCells'
import { useTextRunCells } from './useTextRunCells'

vi.mock('./useTextGlyphCells', () => ({ useTextGlyphCells: vi.fn() }))
vi.mock('../color/useGlyphColor', () => ({ useGlyphColors: vi.fn() }))

function glyph(gid: number): PositionedGlyph {
  return {
    compiledGid: gid,
    documentGid: gid,
    name: `g${gid}`,
    editable: true,
    x: (gid - 1) * 600,
    y: 0,
    xAdvance: 600,
    yAdvance: 0,
    sourceByte: gid - 1,
    sourceKind: 'literal',
  }
}

afterEach(() => vi.clearAllMocks())

describe('text run color data', () => {
  it('keeps color-only preview glyphs and shares the bounded cell set', () => {
    const color = { paint: vi.fn() }
    const cells = new Map([
      [1, makeCell({ gid: 1, path: null })],
      [2, makeCell({ gid: 2, path: 'M0 0L10 10Z' })],
      [3, makeCell({ gid: 3, path: null })],
    ])
    const colors = new Map([[1, color]])
    const glyphs = [glyph(1), glyph(2), glyph(3)]
    vi.mocked(useTextGlyphCells).mockReturnValue(cells)
    vi.mocked(useGlyphColors).mockReturnValue(colors)
    const { result } = renderHook(() =>
      useTextRunCells(5, 'regular', 4, 1000, glyphs, null, [1], true),
    )
    expect(useTextGlyphCells).toHaveBeenCalledWith(
      5,
      'regular',
      4,
      glyphs,
      [1, 0, 2],
    )
    expect(useGlyphColors).toHaveBeenCalledWith(5, 'regular', 4, cells)
    expect(result.current.cells).toBe(cells)
    expect(result.current.colors).toBe(colors)
    expect(result.current.previewGlyphs).toEqual([
      { ...glyphs[0], path: '', color },
      { ...glyphs[1], path: 'M0 0L10 10Z' },
    ])
  })

  it('updates color data with the selected master and excludes distant previews', () => {
    const cells = new Map([[1, makeCell({ gid: 1, path: null })]])
    const first = { paint: vi.fn() }
    const second = { paint: vi.fn() }
    const glyphs = [glyph(1), { ...glyph(2), x: 1_000_000 }]
    vi.mocked(useTextGlyphCells).mockReturnValue(cells)
    vi.mocked(useGlyphColors).mockImplementation(
      (_doc, master) => new Map([[1, master === 'regular' ? first : second]]),
    )
    const { result, rerender } = renderHook(
      ({ master }) =>
        useTextRunCells(5, master, 4, 1000, glyphs, null, [0], false),
      { initialProps: { master: 'regular' } },
    )
    expect(result.current.previewGlyphs).toEqual([
      { ...glyphs[0], path: '', color: first },
    ])
    rerender({ master: 'bold' })
    expect(useGlyphColors).toHaveBeenLastCalledWith(5, 'bold', 4, cells)
    expect(result.current.previewGlyphs).toEqual([
      { ...glyphs[0], path: '', color: second },
    ])
  })
})
