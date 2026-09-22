import { describe, expect, it } from 'vitest'
import { blank, paint, put, resize, sample, transform } from './pixels'
import type { BitmapGlyph } from '../../api'
const glyph = (depth = 1): BitmapGlyph => ({
  bitmap: blank(7, 5, depth),
  advance: 8,
  vertical_advance: null,
  refs: [],
})
describe('packed bitmap tools', () => {
  it('packs every depth MSB-first with zero padding and unsigned RGBA', () => {
    for (const depth of [1, 2, 4, 8, 32]) {
      const b = blank(7, 3, depth),
        max = depth === 32 ? 0xff234580 : (1 << depth) - 1
      put(b, 6, 2, max)
      expect(sample(b, 6, 2)).toBe(max)
      expect(sample(b, 5, 2)).toBe(0)
      if (depth < 8)
        expect(b.rows.at(-1)! & ((1 << (8 - ((7 * depth) % 8))) - 1)).toBe(0)
    }
  })
  it('closes rectangle fills, preserves baseline on resize and rotates dimensions', () => {
    let g = paint(glyph(), 'rectangle', [1, 1], [5, 3], 1)
    g = paint(g, 'fill', [2, 2], [2, 2], 1)
    expect(sample(g.bitmap, 3, 2)).toBe(1)
    expect(sample(g.bitmap, 0, 0)).toBe(0)
    expect(sample(resize(g, 7, 7).bitmap, 3, 4)).toBe(1)
    const rotated = transform(g, 'rotate')
    expect([rotated.bitmap.width, rotated.bitmap.height]).toEqual([5, 7])
    expect(sample(rotated.bitmap, 2, 3)).toBe(1)
  })
})

import { cropGlyph, invert, moveSelection, resample } from './bitmapTransforms'
it('moves floating pixels, crops baseline bounds, interpolates coverage and inverts', () => {
  const g = glyph(8)
  g.bitmap.origin_y = -2
  put(g.bitmap, 1, 1, 255)
  const moved = moveSelection(g, { x: 1, y: 1, width: 1, height: 1 }, 2, 1)
  expect(sample(moved.bitmap, 1, 1)).toBe(0)
  expect(sample(moved.bitmap, 3, 2)).toBe(255)
  const cropped = cropGlyph(moved)
  expect([
    cropped.bitmap.width,
    cropped.bitmap.height,
    cropped.bitmap.origin_x,
    cropped.bitmap.origin_y,
  ]).toEqual([1, 1, 3, 0])
  expect(sample(invert(cropped).bitmap, 0, 0)).toBe(0)
  const scaled = resample(g, 14, 10, true)
  expect(scaled.bitmap.rows.some((v) => v > 0 && v < 255)).toBe(true)
  expect(
    resample(g, 14, 10, false).bitmap.rows.every((v) => v === 0 || v === 255),
  ).toBe(true)
  const ellipse = paint(glyph(), 'ellipse', [1, 0], [5, 4], 1)
  expect(sample(ellipse.bitmap, 3, 0)).toBe(1)
  expect(sample(ellipse.bitmap, 3, 2)).toBe(0)
})
