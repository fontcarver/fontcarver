import { describe, expect, it } from 'vitest'
import type { BitmapGlyph } from '../../api'
import {
  exportSheetPixels,
  importSheetPixels,
  parseSheetNames,
  sheetDimensions,
  pngDimensions,
} from './bitmapSheet'

const options = { cellWidth: 3, cellHeight: 3, columns: 2 }
const glyph: BitmapGlyph = {
  bitmap: {
    width: 2,
    height: 2,
    depth: 2,
    origin_x: -2,
    origin_y: -1,
    rows: [0b01100000, 0b11000000],
  },
  advance: 7,
  vertical_advance: 11,
  vertical_origin: [3, 8],
  refs: [],
}

describe('bitmap sheet codec', () => {
  it('maps explicit names and refuses duplicate or excessive mappings', () => {
    expect(parseSheetNames('A\nB C,zero')).toEqual(['A', 'B', 'C', 'zero'])
    expect(() => parseSheetNames('A A')).toThrow()
    expect(() => parseSheetNames('')).toThrow()
    expect(() =>
      parseSheetNames(Array.from({ length: 257 }, (_, i) => `g${i}`).join(' ')),
    ).toThrow()
  })

  it('exports row-major cells with independent bottom-left pixel positions', () => {
    const result = exportSheetPixels([glyph, glyph, glyph], options)
    expect([result.width, result.height]).toEqual([6, 6])
    const alpha = (x: number, y: number) => result.data[(y * 6 + x) * 4 + 3]
    expect(alpha(0, 0)).toBe(0)
    expect(alpha(0, 1)).toBe(85)
    expect(alpha(1, 1)).toBe(170)
    expect(alpha(3, 1)).toBe(85)
    expect(alpha(0, 4)).toBe(85)
    expect(alpha(2, 2)).toBe(0)
    expect(alpha(3, 4)).toBe(0)
  })

  it('retains all metrics and original bounds for an existing bitmap', () => {
    const image = exportSheetPixels([glyph], options)
    expect(
      importSheetPixels(image, ['A'], options, 2, 2, { A: glyph }),
    ).toEqual({ A: glyph })
  })

  it('reads alpha coverage, packs each row independently and quantizes depth', () => {
    const image = {
      width: 3,
      height: 1,
      data: new Uint8ClampedArray([
        255, 255, 255, 0, 255, 0, 0, 128, 0, 0, 0, 255,
      ]),
    }
    const result = importSheetPixels(
      image,
      ['A'],
      { cellWidth: 3, cellHeight: 1, columns: 1 },
      2,
      1,
      {},
    )
    expect(result.A.bitmap).toEqual({
      width: 3,
      height: 1,
      depth: 2,
      origin_x: 0,
      origin_y: -1,
      rows: [0b00101100],
    })
    expect(result.A.advance).toBe(3)
  })

  it('preserves exact RGBA values including transparent colored samples', () => {
    const rgba: BitmapGlyph = {
      ...glyph,
      bitmap: {
        ...glyph.bitmap,
        width: 2,
        height: 1,
        depth: 32,
        rows: [250, 12, 33, 87, 100, 200, 30, 0],
      },
    }
    const image = exportSheetPixels([rgba], options)
    expect(Array.from(image.data.slice(48, 56))).toEqual(rgba.bitmap.rows)
    expect(
      importSheetPixels(image, ['A'], options, 32, 2, { A: rgba }).A,
    ).toEqual(rgba)
  })

  it.each([1, 2, 4, 8])(
    'round-trips odd-width row boundaries at %i bpp',
    (depth) => {
      const max = (1 << depth) - 1
      const data = new Uint8ClampedArray(3 * 2 * 4)
      ;[0, 255, 128, 255, 0, 255].forEach((alpha, index) => {
        data[index * 4 + 3] = alpha
      })
      const settings = { cellWidth: 3, cellHeight: 2, columns: 1 }
      const result = importSheetPixels(
        { width: 3, height: 2, data },
        ['A'],
        settings,
        depth,
        1,
        {},
      )
      const out = exportSheetPixels([result.A], settings)
      expect([0, 1, 2, 3, 4, 5].map((i) => out.data[i * 4 + 3])).toEqual([
        0,
        255,
        Math.round((Math.round((128 * max) / 255) * 255) / max),
        255,
        0,
        255,
      ])
      expect(result.A.bitmap.rows).toHaveLength(Math.ceil((3 * depth) / 8) * 2)
    },
  )

  it('rejects ink in an unassigned trailing cell', () => {
    const image = exportSheetPixels([glyph], options)
    image.data[3 * 4 + 3] = 1
    expect(() =>
      importSheetPixels(image, ['A'], options, 2, 2, { A: glyph }),
    ).toThrow('padding')
  })

  it('accepts prototype-like glyph names as ordinary names', () => {
    const image = {
      width: 1,
      height: 1,
      data: new Uint8ClampedArray([0, 0, 0, 255]),
    }
    const result = importSheetPixels(
      image,
      ['__proto__'],
      { cellWidth: 1, cellHeight: 1, columns: 1 },
      8,
      1,
      {},
    )
    expect(Object.keys(result)).toEqual(['__proto__'])
    expect(result.__proto__.bitmap.rows).toEqual([255])
  })

  it('retains empty bitmap dimensions and materializes reference metadata', () => {
    const empty = {
      ...glyph,
      bitmap: { ...glyph.bitmap, width: 0, height: 0, rows: [] },
    }
    const image = exportSheetPixels([empty], options)
    expect(
      importSheetPixels(image, ['space'], options, 2, 2, { space: empty })
        .space,
    ).toEqual(empty)
  })

  it('rejects clipping, painted padding, image size mismatches and allocation overflow', () => {
    expect(() =>
      exportSheetPixels([glyph], { ...options, cellWidth: 1 }),
    ).toThrow()
    const image = exportSheetPixels([glyph], options)
    image.data[3] = 255
    expect(() =>
      importSheetPixels(image, ['A'], options, 2, 2, { A: glyph }),
    ).toThrow()
    expect(() =>
      importSheetPixels({ ...image, width: 7 }, ['A'], options, 2, 2, {}),
    ).toThrow()
    for (const cellWidth of [NaN, Infinity, -1, 1.5, 8193])
      expect(() => sheetDimensions(1, { ...options, cellWidth })).toThrow()
    expect(() =>
      sheetDimensions(256, { cellWidth: 4096, cellHeight: 4096, columns: 16 }),
    ).toThrow()
  })

  it('rejects non-PNG and oversized IHDR before a decoder sees the image', () => {
    const png = new Uint8Array(33)
    png.set([137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82])
    const view = new DataView(png.buffer)
    view.setUint32(16, 64)
    view.setUint32(20, 32)
    expect(pngDimensions(png)).toEqual({ width: 64, height: 32 })
    view.setUint32(16, 8193)
    expect(() => pngDimensions(png)).toThrow()
    expect(() => pngDimensions(png.subarray(0, 24))).toThrow()
    expect(() => pngDimensions(new Uint8Array(33))).toThrow()
  })
})
