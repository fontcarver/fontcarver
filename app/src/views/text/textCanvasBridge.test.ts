import { describe, expect, it, vi } from 'vitest'
import { Viewport } from '../../canvas/Viewport'
import { makeCell } from '../../test/fontFixtures'
import { shiftRunOrigin, surroundingGlyphs } from './textCanvasBridge'
import type { TextCanvasRun } from './textCanvas'
import type { PositionedGlyph } from './textLayout'

const glyph: PositionedGlyph = {
  compiledGid: 1,
  documentGid: 7,
  name: 'A',
  editable: true,
  x: 0,
  y: 0,
  xAdvance: 600,
  yAdvance: 0,
  sourceByte: 0,
  sourceKind: 'literal',
}
describe('canvas run bridge', () => {
  const run: TextCanvasRun = {
    glyphs: [{ ...glyph, x: 600, y: 50 }],
    cells: new Map([[7, makeCell({ gid: 7, path: null })]]),
    activeIndex: null,
    activeOrigin: { x: 100, y: 20 },
    mode: 'text',
    unitsPerEm: 1000,
    fitBox: { x0: 0, y0: 0, x1: 1000, y1: 1000 },
    requestedScale: 1,
    onActivate: () => undefined,
    onExitEdit: () => undefined,
    onVisibleIndexes: () => undefined,
  }
  it('includes positioned color glyphs without a monochrome outline', () => {
    const color = { paint: vi.fn() }
    const shown = surroundingGlyphs(
      { ...run, colors: new Map([[7, color]]) },
      new Map(),
      [0],
    )
    expect(shown).toEqual([{ index: 0, path: null, color, x: 500, y: 30 }])
  })
  it('omits the active glyph while editing even when it has color paint', () => {
    expect(
      surroundingGlyphs(
        {
          ...run,
          mode: 'edit',
          activeIndex: 0,
          colors: new Map([[7, { paint: vi.fn() }]]),
        },
        new Map(),
        [0],
      ),
    ).toEqual([])
  })
  it('bounds draw entries even when every offscreen occurrence shares a cached path', () => {
    const run: TextCanvasRun = {
      glyphs: Array.from({ length: 65535 }, (_, i) => ({
        ...glyph,
        x: i * 600,
      })),
      cells: new Map([[7, makeCell({ gid: 7, path: 'M0 0L10 10Z' })]]),
      activeIndex: null,
      activeOrigin: { x: 0, y: 0 },
      mode: 'text',
      unitsPerEm: 1000,
      fitBox: { x0: 0, y0: 0, x1: 1000, y1: 1000 },
      requestedScale: 1,
      onActivate: () => undefined,
      onExitEdit: () => undefined,
      onVisibleIndexes: () => undefined,
    }
    const cache = new Map<string, Path2D>()
    const shown = surroundingGlyphs(run, cache, [300, 301, 302])
    expect(shown).toHaveLength(3)
    expect(shown[0]?.x).toBe(180000)
    expect(cache.size).toBe(1)
  })
  it('shifts viewport origin without changing absolute run placement or glyph-local units', () => {
    const viewport = new Viewport()
    viewport.setSize(1000, 500, 1)
    viewport.zoomTo(0.5)
    const before = viewport.toScreen({ x: 1240, y: 80 })
    shiftRunOrigin(viewport, { x: 0, y: 0 }, { x: 1200, y: 50 })
    expect(viewport.toScreen({ x: 40, y: 30 })).toEqual(before)
    expect(viewport.toFont(before)).toEqual({ x: 40, y: 30 })
  })
})
