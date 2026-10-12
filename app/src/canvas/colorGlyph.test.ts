import { describe, expect, it, vi } from 'vitest'
import { colorLayers } from './colorGlyph'

describe('color glyph painter', () => {
  it('preserves layer ordering, palette alpha and dynamic foreground', () => {
    const fill = vi.fn()
    const context = {
      save: vi.fn(),
      restore: vi.fn(),
      fill,
      fillStyle: '',
      globalAlpha: 1,
    } as unknown as CanvasRenderingContext2D
    const layers = colorLayers([
      {
        path: 'M0 0L100 0L100 100Z',
        color: { red: 200, green: 20, blue: 30, alpha: 128 },
        glyph: 'red',
        bounds: [0, 0, 100, 100],
      },
      {
        path: 'M10 10L20 10L20 20Z',
        color: null,
        glyph: 'ink',
        bounds: [10, 10, 20, 20],
      },
    ])
    fill.mockImplementation(() => ({
      fill: context.fillStyle,
      alpha: context.globalAlpha,
    }))
    layers.paint(context, '#fafafa')
    expect(fill.mock.results.map((r) => r.value)).toEqual([
      { fill: 'rgb(200 20 30)', alpha: 128 / 255 },
      { fill: '#fafafa', alpha: 1 },
    ])
  })
})
