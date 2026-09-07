import { act, renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { setTransport, type GlyphCell } from '../../api'
import { createMockTransport } from '../../test/mockTransport'
import type { PositionedGlyph } from './textLayout'
import { useTextGlyphCells } from './useTextGlyphCells'

function glyph(index: number, gid = index + 1): PositionedGlyph {
  return {
    compiledGid: index,
    documentGid: gid,
    name: `g${index}`,
    editable: true,
    x: index * 600,
    y: 0,
    xAdvance: 600,
    yAdvance: 0,
    sourceByte: index,
    sourceKind: 'literal',
  }
}

function cell(gid: number): GlyphCell {
  return {
    gid,
    name: `g${gid}`,
    unicodes: [],
    category: '',
    sub_category: '',
    script: '',
    width: 600,
    bbox: [0, 0, 500, 700],
    path: `M${gid} 0Z`,
    color_label: null,
    export: true,
    has_components: false,
    is_empty: false,
    generation: 1,
  }
}

afterEach(() => {
  setTransport(undefined)
  vi.restoreAllMocks()
})

describe('useTextGlyphCells', () => {
  it('loads each visible document glyph once even when it repeats', async () => {
    const transport = createMockTransport({
      'glyphs.cell': ({ gid }) => cell(gid),
    })
    setTransport(transport)
    const glyphs = [glyph(0, 7), glyph(1, 7), glyph(2, 9)]
    const { result } = renderHook(() =>
      useTextGlyphCells(1, 'regular', 1, glyphs, [0, 1, 2]),
    )

    await vi.waitFor(() => expect(result.current.size).toBe(2))
    expect(
      transport.calls.filter((entry) => entry.method === 'glyphs.cell'),
    ).toHaveLength(2)
  })

  it('drops a late answer from an earlier master context', async () => {
    let release: ((value: GlyphCell) => void) | undefined
    const transport = createMockTransport({
      'glyphs.cell': ({ gid, master }) =>
        master === 'old'
          ? new Promise<GlyphCell>((resolve) => {
              release = resolve
            })
          : cell(gid),
    })
    setTransport(transport)
    const glyphs = [glyph(0, 7)]
    const { result, rerender } = renderHook(
      ({ master }) => useTextGlyphCells(1, master, 1, glyphs, [0]),
      { initialProps: { master: 'old' } },
    )

    rerender({ master: 'new' })
    await vi.waitFor(() => expect(result.current.get(7)?.path).toBe('M7 0Z'))
    await act(async () => release?.({ ...cell(7), path: 'late' }))

    expect(result.current.get(7)?.path).toBe('M7 0Z')
  })
})
