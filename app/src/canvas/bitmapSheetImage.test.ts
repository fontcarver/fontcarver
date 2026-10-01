import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  decodeSheetPng,
  embedPngText,
  readPngText,
  MAX_PNG_TEXT_BYTES,
} from './bitmapSheetImage'
import {
  encodeSheetLayout,
  decodeSheetLayout,
  SHEET_LAYOUT_KEYWORD,
  SHEET_MAX_BYTES,
} from '../views/bitmap/bitmapSheet'

afterEach(() => {
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
})

function pngBytes(width: number, height: number): Uint8Array<ArrayBuffer> {
  // Signature, a complete IHDR chunk and an IEND chunk: enough for the header
  // check and for chunk walking, without a real image stream.
  const bytes = new Uint8Array(45)
  bytes.set([137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82])
  const view = new DataView(bytes.buffer)
  view.setUint32(16, width)
  view.setUint32(20, height)
  bytes.set([73, 69, 78, 68], 37)
  return bytes
}

function file(width: number, height: number, size = 45): File {
  const bytes = pngBytes(width, height)
  return {
    size,
    arrayBuffer: async () => bytes.buffer,
    slice: () => ({ arrayBuffer: async () => bytes.buffer }),
  } as unknown as File
}

it('checks both file-byte and advertised image bounds before decoding', async () => {
  const decoder = vi.fn()
  vi.stubGlobal('createImageBitmap', decoder)
  await expect(
    decodeSheetPng(file(1, 1, SHEET_MAX_BYTES + 1)),
  ).rejects.toThrow()
  await expect(decodeSheetPng(file(8193, 1))).rejects.toThrow()
  await expect(decodeSheetPng(file(4096, 4096))).rejects.toThrow()
  expect(decoder).not.toHaveBeenCalled()
})

it('closes a decoder result whose dimensions disagree with IHDR', async () => {
  const close = vi.fn()
  vi.stubGlobal(
    'createImageBitmap',
    vi.fn().mockResolvedValue({ width: 2, height: 1, close }),
  )
  await expect(decodeSheetPng(file(1, 1))).rejects.toThrow()
  expect(close).toHaveBeenCalledOnce()
})

it('closes the image even when canvas access fails', async () => {
  const close = vi.fn()
  vi.stubGlobal(
    'createImageBitmap',
    vi.fn().mockResolvedValue({ width: 1, height: 1, close }),
  )
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(null)
  await expect(decodeSheetPng(file(1, 1))).rejects.toThrow()
  expect(close).toHaveBeenCalledOnce()
})

describe('sheet layout carried by the PNG itself', () => {
  const layout = {
    cellWidth: 8,
    cellHeight: 10,
    columns: 4,
    names: ['A', 'B', 'uni4E2D'],
  }

  it('round-trips the layout through a tEXt chunk without moving the pixels', () => {
    const original = pngBytes(32, 10)
    const embedded = embedPngText(
      original,
      SHEET_LAYOUT_KEYWORD,
      encodeSheetLayout(layout),
    )
    // The signature and IHDR are byte-identical and IEND still follows.
    expect([...embedded.subarray(0, 33)]).toEqual([...original.subarray(0, 33)])
    expect([...embedded.subarray(embedded.length - 8)]).toEqual([
      ...original.subarray(original.length - 8),
    ])
    const text = readPngText(embedded, SHEET_LAYOUT_KEYWORD)
    expect(text).not.toBeNull()
    expect(decodeSheetLayout(text!)).toEqual(layout)
  })

  it('writes only printable ASCII so the chunk stays Latin-1', () => {
    const text = encodeSheetLayout({ ...layout, names: ['A', 'uni4E2D', 'B'] })
    expect(/^[\x20-\x7e]*$/u.test(text)).toBe(true)
    const wide = encodeSheetLayout({ ...layout, names: ['中', 'B', 'A'] })
    expect(/^[\x20-\x7e]*$/u.test(wide)).toBe(true)
    expect(decodeSheetLayout(wide)?.names).toEqual(['中', 'B', 'A'])
  })

  it('ignores another keyword and a truncated or absent chunk', () => {
    const bytes = embedPngText(pngBytes(32, 10), 'Other', 'hello')
    expect(readPngText(bytes, SHEET_LAYOUT_KEYWORD)).toBeNull()
    expect(readPngText(bytes, 'Other')).toBe('hello')
    expect(readPngText(pngBytes(32, 10), SHEET_LAYOUT_KEYWORD)).toBeNull()
    // A file cut off inside the chunk declares more data than it holds.
    expect(readPngText(bytes.subarray(0, 40), 'Other')).toBeNull()
  })

  it('skips an oversized chunk instead of decoding it', () => {
    // A payload far past the cap used to be spread into String.fromCharCode,
    // which throws RangeError on a few hundred kilobytes.
    const huge = 'x'.repeat(MAX_PNG_TEXT_BYTES + 1)
    const bytes = embedPngText(pngBytes(32, 10), SHEET_LAYOUT_KEYWORD, huge)
    expect(readPngText(bytes, SHEET_LAYOUT_KEYWORD)).toBeNull()
    // A chunk at the cap is still read, and the walk reaches later chunks.
    const atCap = embedPngText(
      pngBytes(32, 10),
      SHEET_LAYOUT_KEYWORD,
      'y'.repeat(MAX_PNG_TEXT_BYTES - SHEET_LAYOUT_KEYWORD.length - 1),
    )
    expect(readPngText(atCap, SHEET_LAYOUT_KEYWORD)).toHaveLength(
      MAX_PNG_TEXT_BYTES - SHEET_LAYOUT_KEYWORD.length - 1,
    )
  })

  it('rejects a layout whose numbers or names are unusable', () => {
    expect(decodeSheetLayout('not json')).toBeNull()
    expect(
      decodeSheetLayout(
        '{"names":["A","A"],"cellWidth":8,"cellHeight":8,"columns":1}',
      ),
    ).toBeNull()
    expect(
      decodeSheetLayout(
        '{"names":[1],"cellWidth":8,"cellHeight":8,"columns":1}',
      ),
    ).toBeNull()
    expect(
      decodeSheetLayout(
        '{"names":["A"],"cellWidth":0,"cellHeight":8,"columns":1}',
      ),
    ).toBeNull()
    expect(
      decodeSheetLayout(
        '{"names":[],"cellWidth":8,"cellHeight":8,"columns":1}',
      ),
    ).toBeNull()
  })

  it('reads a layout back out of a decoded file', async () => {
    const bytes = embedPngText(
      pngBytes(32, 10),
      SHEET_LAYOUT_KEYWORD,
      encodeSheetLayout(layout),
    )
    const close = vi.fn()
    vi.stubGlobal(
      'createImageBitmap',
      vi.fn().mockResolvedValue({ width: 32, height: 10, close }),
    )
    const data = new Uint8ClampedArray(32 * 10 * 4)
    vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue({
      drawImage: vi.fn(),
      getImageData: () => ({ data }),
    } as unknown as CanvasRenderingContext2D)
    const decoded = await decodeSheetPng({
      size: bytes.length,
      arrayBuffer: async () => bytes.buffer,
      slice: () => ({ arrayBuffer: async () => bytes.buffer }),
    } as unknown as File)
    expect(decoded.layout).toEqual(layout)
    expect(close).toHaveBeenCalledOnce()
  })
})
