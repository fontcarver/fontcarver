import { describe, expect, it } from 'vitest'

import { Viewport } from '../../canvas/Viewport'
import type { PositionedGlyph } from './textLayout'
import {
  MAX_VISIBLE_RUN_GLYPHS,
  hitTextGlyph,
  previewGlyphIndexes,
  runBox,
  visibleGlyphIndexes,
} from './textRunGeometry'

function glyph(
  index: number,
  patch: Partial<PositionedGlyph> = {},
): PositionedGlyph {
  return {
    compiledGid: index,
    documentGid: index + 1,
    name: `g${index}`,
    editable: true,
    x: index * 600,
    y: 0,
    xAdvance: 600,
    yAdvance: 0,
    sourceByte: index,
    sourceKind: 'literal',
    ...patch,
  }
}

describe('text run geometry', () => {
  it('builds a fit box from advances and outline bounds', () => {
    expect(
      runBox(
        [glyph(0), glyph(1, { x: 600, xAdvance: 0 })],
        new Map([[2, [-80, -240, 320, 760]]]),
        1_000,
      ),
    ).toEqual({ x0: 0, y0: -240, x1: 920, y1: 800 })
  })

  it('gives zero-advance marks a usable screen-space hit target', () => {
    const mark = glyph(0, { x: 500, xAdvance: 0 })
    expect(hitTextGlyph([mark], new Map(), { x: 504, y: 0 }, 1_000, 1)).toBe(0)
  })

  it('keeps advance-only spaces selectable', () => {
    const space = glyph(0, { xAdvance: 300 })
    expect(hitTextGlyph([space], new Map(), { x: 150, y: 0 }, 1_000, 0.2)).toBe(
      0,
    )
  })

  it('culls to the viewport and caps pathological zoomed-out runs', () => {
    const viewport = new Viewport()
    viewport.setSize(800, 600)
    viewport.scale = 1
    viewport.offset = { x: 100, y: 400 }
    const run = Array.from({ length: 2_000 }, (_, index) => glyph(index))

    expect(
      visibleGlyphIndexes(run, new Map(), viewport, { x: 0, y: 0 }, 1_000),
    ).toEqual([0, 1, 2])

    viewport.scale = 0.0005
    expect(
      visibleGlyphIndexes(run, new Map(), viewport, { x: 0, y: 0 }, 1_000),
    ).toHaveLength(MAX_VISIBLE_RUN_GLYPHS)
  })

  it('bounds preview fetching instead of allocating a full CJK run', () => {
    const run = Array.from({ length: 65_535 }, (_, index) => glyph(index))
    const indexes = previewGlyphIndexes(run, 1_000)

    expect(indexes.length).toBe(MAX_VISIBLE_RUN_GLYPHS)
    expect(indexes[0]).toBe(0)
    expect(indexes.at(-1)).toBe(MAX_VISIBLE_RUN_GLYPHS - 1)
  })
})
