import { expect, it, vi } from 'vitest'
import { Viewport } from '../../canvas/Viewport'
import { drawVariationPreview } from './drawVariationPreview'
import { FALLBACK_GLYPH_THEME } from './glyphTheme'

it('draws the live outline in the editable viewport with constant stroke width', () => {
  const ctx = {
    save: vi.fn(),
    restore: vi.fn(),
    translate: vi.fn(),
    scale: vi.fn(),
    fill: vi.fn(),
    stroke: vi.fn(),
    beginPath: vi.fn(),
    moveTo: vi.fn(),
    lineTo: vi.fn(),
    setLineDash: vi.fn(),
    globalAlpha: 1,
  } as unknown as CanvasRenderingContext2D
  const viewport = new Viewport()
  viewport.scale = 2
  viewport.offset = { x: 30, y: 400 }
  const path = {} as Path2D
  drawVariationPreview(
    ctx,
    viewport,
    { path, width: 620 },
    FALLBACK_GLYPH_THEME,
  )
  expect(ctx.translate).toHaveBeenCalledWith(30, 400)
  expect(ctx.scale).toHaveBeenCalledWith(2, -2)
  expect(ctx.stroke).toHaveBeenCalledWith(path)
  expect(ctx.lineWidth).toBe(0.75)
  expect(ctx.moveTo).toHaveBeenCalledWith(620, -6)
  expect(ctx.restore).toHaveBeenCalledOnce()
})
