import { describe, expect, it, vi } from 'vitest'
import { Viewport } from '../../canvas/Viewport'
import { drawTextRun } from './drawTextRun'

describe('text run color painting', () => {
  it('paints color-only glyphs at shaped offsets in y-up font space', () => {
    const calls: unknown[] = []
    const context = {
      save: vi.fn(),
      restore: vi.fn(),
      setTransform: vi.fn((...args: number[]) =>
        calls.push(['transform', args]),
      ),
      translate: vi.fn((...args: number[]) => calls.push(['translate', args])),
      fill: vi.fn(),
    } as unknown as CanvasRenderingContext2D
    const color = { paint: vi.fn(() => calls.push(['color'])) }
    const viewport = new Viewport()
    viewport.setSize(1000, 500, 2)
    viewport.zoomTo(0.5)
    drawTextRun(
      context,
      viewport,
      [{ index: 0, path: null, color, x: 600, y: 30 }],
      '#fff',
    )
    expect(calls).toEqual([
      [
        'transform',
        [1, 0, 0, -1, viewport.offset.x * 2, viewport.offset.y * 2],
      ],
      ['translate', [600, 30]],
      ['color'],
    ])
    expect(color.paint).toHaveBeenCalledWith(context, '#fff')
    expect(context.fill).not.toHaveBeenCalled()
    expect(context.save).toHaveBeenCalledTimes(2)
    expect(context.restore).toHaveBeenCalledTimes(2)
  })

  it('prefers color over a base outline and retains the monochrome fallback', () => {
    const context = {
      save: vi.fn(),
      restore: vi.fn(),
      setTransform: vi.fn(),
      translate: vi.fn(),
      fill: vi.fn(),
    } as unknown as CanvasRenderingContext2D
    const color = { paint: vi.fn() }
    const path = new Path2D('M0 0L10 10Z')
    drawTextRun(
      context,
      new Viewport(),
      [
        { index: 0, path, color, x: 0, y: 0 },
        { index: 1, path, x: 500, y: 0 },
      ],
      '#123',
    )
    expect(color.paint).toHaveBeenCalledOnce()
    expect(context.fill).toHaveBeenCalledExactlyOnceWith(path)
    expect(context.fillStyle).toBe('#123')
  })
})
