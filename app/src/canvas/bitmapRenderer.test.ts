import { describe, expect, it, vi } from 'vitest'
import { blank, put } from '../views/bitmap/pixels'
import { bitmapFrame } from './bitmapGeometry'
import { renderBitmap } from './bitmapRenderer'

function surface() {
  const ctx = {
    fillRect: vi.fn(),
    beginPath: vi.fn(),
    moveTo: vi.fn(),
    lineTo: vi.fn(),
    stroke: vi.fn(),
    strokeRect: vi.fn(),
    setLineDash: vi.fn(),
    save: vi.fn(),
    translate: vi.fn(),
    scale: vi.fn(),
    restore: vi.fn(),
  }
  const canvas = {
    width: 0,
    height: 0,
    getContext: () => ctx,
  } as unknown as HTMLCanvasElement
  return { ctx, canvas }
}
const colors = { paper: 'white', ink: 'black', grid: 'gray', accent: 'blue' }

describe('bitmap renderer guides', () => {
  it('keeps baseline and advance when grid is off and respects bitmap origins', () => {
    const { ctx, canvas } = surface()
    const bitmap = { ...blank(7, 5, 1), origin_x: -2, origin_y: -1 }
    put(bitmap, 0, 0, 1)
    renderBitmap(
      canvas,
      bitmap,
      10,
      colors,
      { x: 0, y: 0, width: 1, height: 1 },
      true,
      { frame: bitmapFrame(bitmap, 9), advance: 9, grid: false },
    )
    expect([canvas.width, canvas.height]).toEqual([130, 70])
    expect(ctx.fillRect).toHaveBeenCalledWith(10, 10, 10, 10)
    expect(ctx.moveTo.mock.calls).toEqual([
      [0, 50],
      [30, 0],
      [120, 0],
    ])
    expect(ctx.lineTo).toHaveBeenCalledWith(120, 70)
    expect(ctx.strokeRect).toHaveBeenCalledWith(10, 10, 10, 10)
  })
  it('maps outline font units to y-up bitmap space', () => {
    const path = vi.fn(function (this: { data: string }, data: string) {
      this.data = data
    })
    vi.stubGlobal('Path2D', path)
    const { ctx, canvas } = surface()
    const bitmap = { ...blank(7, 5, 1), origin_x: -2, origin_y: -1 }
    renderBitmap(canvas, bitmap, 10, colors, undefined, true, {
      frame: bitmapFrame(bitmap, 9),
      outlinePath: 'M 0 0 L 500 500',
      unitsPerPixel: 50,
    })
    expect(path).toHaveBeenCalledWith('M 0 0 L 500 500')
    expect(ctx.translate).toHaveBeenCalledWith(30, 50)
    expect(ctx.scale).toHaveBeenCalledWith(0.2, -0.2)
    vi.unstubAllGlobals()
  })
  it('keeps waterfall transparent and free of editor guides', () => {
    const { ctx, canvas } = surface()
    const bitmap = blank(3, 2, 1)
    put(bitmap, 0, 0, 1)
    renderBitmap(canvas, bitmap, 2, colors, undefined, false)
    expect([canvas.width, canvas.height]).toEqual([6, 4])
    expect(ctx.fillRect.mock.calls).toEqual([[0, 0, 2, 2]])
    expect(ctx.stroke).not.toHaveBeenCalled()
  })
})
