import { afterEach, expect, it, vi } from 'vitest'
import { decodeSheetPng } from './bitmapSheetImage'
import { SHEET_MAX_BYTES } from '../views/bitmap/bitmapSheet'

afterEach(() => {
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
})

function file(width: number, height: number, size = 33): File {
  const header = new Uint8Array(33)
  header.set([137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82])
  const view = new DataView(header.buffer)
  view.setUint32(16, width)
  view.setUint32(20, height)
  return {
    size,
    slice: () => ({ arrayBuffer: async () => header.buffer }),
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
