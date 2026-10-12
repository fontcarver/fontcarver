import { expect, it } from 'vitest'
import type { BitmapGlyph, BitmapPreviewLine } from '../../api'
import { layoutBitmapWaterfall } from './bitmapWaterfallLayout'
import { blank } from './pixels'

function glyph(overrides: Partial<BitmapGlyph> = {}): BitmapGlyph {
  return {
    bitmap: { ...blank(3, 4, 1), origin_x: -1, origin_y: -2 },
    advance: 8,
    vertical_advance: 12,
    vertical_origin: [2, 7],
    refs: [],
    ...overrides,
  }
}
function line(glyphs: (BitmapGlyph | null)[]): BitmapPreviewLine {
  return { ppem: 10, depth: 1, ascent: 8, descent: 2, glyphs }
}

it('positions upright vertical pixels using both origin coordinates and vertical advances', () => {
  const first = glyph(),
    second = glyph({ vertical_origin: [4, 9] })
  const result = layoutBitmapWaterfall(line([first, second]), true)
  expect(result.placements).toEqual([
    { glyph: first, x: 2, y: 15 },
    { glyph: second, x: 0, y: 29 },
  ])
  expect([result.width, result.height]).toEqual([10, 44])
})

it('preserves zero advances, missing-glyph spacing and defined legacy fallback metrics', () => {
  const first = glyph({ vertical_advance: 0 }),
    last = glyph({ vertical_advance: null, vertical_origin: undefined })
  const result = layoutBitmapWaterfall(line([first, null, last]), true)
  expect(result.placements[1].y - result.placements[0].y).toBe(11)
  expect(result.placements[1].x - result.placements[0].x).toBe(-2)
  expect(result.height).toBe(40)
})

it('retains negative bearings and negative horizontal advances without cropping ink', () => {
  const first = glyph({ advance: -20 }),
    second = glyph()
  const result = layoutBitmapWaterfall(line([first, second]), false)
  expect(result.placements[0].x - result.placements[1].x).toBe(20)
  for (const item of result.placements) {
    expect(item.x).toBeGreaterThanOrEqual(0)
    expect(item.x + item.glyph.bitmap.width).toBeLessThanOrEqual(result.width)
    expect(item.y).toBeGreaterThanOrEqual(0)
    expect(item.y + item.glyph.bitmap.height).toBeLessThanOrEqual(result.height)
  }
})

it('reports truncation when the canvas exceeds bounded pixel dimensions', () => {
  const result = layoutBitmapWaterfall(
    line(Array.from({ length: 128 }, () => glyph({ vertical_advance: 4096 }))),
    true,
  )
  expect(result.height).toBe(16384)
  expect(result.clipped).toBe(true)
  expect(result.width * result.height).toBeLessThanOrEqual(4_000_000)
})
