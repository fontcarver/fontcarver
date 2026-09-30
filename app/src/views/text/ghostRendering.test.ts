import { expect, it, vi } from 'vitest'
import { Viewport } from '../../canvas/Viewport'
import { FALLBACK_GLYPH_THEME } from '../glyph/glyphTheme'
import { drawTextGaps } from './drawTextGaps'
import { drawTextRun } from './drawTextRun'
import { surroundingGlyphs } from './textCanvasBridge'
import type { TextCanvasRun } from './textCanvas'

function run(): TextCanvasRun {
  return {
    glyphs: [],
    cells: new Map(),
    activeIndex: null,
    activeOrigin: { x: 100, y: 50 },
    mode: 'text',
    unitsPerEm: 1000,
    fitBox: { x0: 0, x1: 1000, y0: 0, y1: 1000 },
    requestedScale: 1,
    onActivate: vi.fn(),
    onExitEdit: vi.fn(),
    onVisibleIndexes: vi.fn(),
    selectedGap: 0,
    gaps: [{ index: 0, left: 'A', right: 'V', x: 500, reason: null }],
    kerningGhosts: [
      { index: 0, name: 'a', path: 'M0 0L10 10Z', x: 120, y: 80 },
      { index: 1, name: 'v', path: 'M0 0L20 20Z', x: 700, y: 80 },
    ],
  }
}
function context() {
  return {
    save: vi.fn(),
    restore: vi.fn(),
    setTransform: vi.fn(),
    translate: vi.fn(),
    fill: vi.fn(),
    stroke: vi.fn(),
    setLineDash: vi.fn(),
    beginPath: vi.fn(),
    moveTo: vi.fn(),
    lineTo: vi.fn(),
    closePath: vi.fn(),
    fillStyle: '',
    strokeStyle: '',
    globalAlpha: 1,
    lineWidth: 1,
  } as unknown as CanvasRenderingContext2D
}

it('culls member ghosts with their owning glyph and uses active local coordinates', () => {
  const paths = new Map<string, Path2D>()
  const shown = surroundingGlyphs(run(), paths, [0])
  expect(shown).toHaveLength(1)
  expect(shown[0]).toMatchObject({ x: 20, y: 30, ghost: true })
  expect(paths.size).toBe(1)
  expect(surroundingGlyphs({ ...run(), mode: 'edit' }, paths, [0])).toEqual([])
  const ctx = context()
  const viewport = new Viewport()
  viewport.scale = 0.5
  drawTextRun(ctx, viewport, shown, '#111')
  expect(ctx.fill).not.toHaveBeenCalled()
  expect(ctx.stroke).toHaveBeenCalledWith(shown[0]?.path)
  expect(ctx.setLineDash).toHaveBeenCalledWith([6, 6])
  expect(ctx.globalAlpha).toBe(0.35)
  expect(ctx.lineWidth).toBe(2)
})

it.each([
  [-40, 'warning'],
  [40, 'nodeSmooth'],
  [0, 'accent'],
  [null, 'accent'],
] as const)(
  'colors resolved value %s without changing the gap position',
  (value, color) => {
    const ctx = context()
    const viewport = new Viewport()
    drawTextGaps(
      ctx,
      viewport,
      { ...run(), selectedKerningValue: value },
      FALLBACK_GLYPH_THEME,
      [0],
    )
    expect(ctx.strokeStyle).toBe(FALLBACK_GLYPH_THEME[color])
    expect(ctx.fillStyle).toBe(FALLBACK_GLYPH_THEME[color])
    const point = viewport.toScreen({ x: 400, y: -170 })
    expect(ctx.moveTo).toHaveBeenCalledWith(point.x, point.y)
  },
)
