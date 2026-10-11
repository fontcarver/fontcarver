import {
  decodeSheetLayout,
  encodeSheetLayout,
  pngDimensions,
  SHEET_LAYOUT_KEYWORD,
  SHEET_MAX_BYTES,
  SheetError,
  type SheetLayout,
  type SheetPixels,
} from '../views/bitmap/bitmapSheetModel'

/** Table-driven CRC-32, the checksum every PNG chunk carries. */
const CRC_TABLE = (() => {
  const table = new Uint32Array(256)
  for (let index = 0; index < 256; index++) {
    let value = index
    for (let bit = 0; bit < 8; bit++)
      value = value & 1 ? 0xedb88320 ^ (value >>> 1) : value >>> 1
    table[index] = value >>> 0
  }
  return table
})()

function crc32(bytes: Uint8Array): number {
  let crc = 0xffffffff
  for (const byte of bytes) crc = CRC_TABLE[(crc ^ byte) & 0xff] ^ (crc >>> 8)
  return (crc ^ 0xffffffff) >>> 0
}

/** Insert a `tEXt` chunk right after IHDR, leaving the image data untouched. */
export function embedPngText(
  bytes: Uint8Array,
  keyword: string,
  text: string,
): Uint8Array<ArrayBuffer> {
  // Signature (8) + IHDR length/type/data/CRC (4 + 4 + 13 + 4).
  const insertAt = 33
  if (bytes.length < insertAt) throw new SheetError('png')
  const payload = new Uint8Array(keyword.length + 1 + text.length)
  for (let index = 0; index < keyword.length; index++)
    payload[index] = keyword.charCodeAt(index) & 0xff
  payload[keyword.length] = 0
  for (let index = 0; index < text.length; index++)
    payload[keyword.length + 1 + index] = text.charCodeAt(index) & 0xff
  const chunk = new Uint8Array(payload.length + 12)
  const view = new DataView(chunk.buffer)
  view.setUint32(0, payload.length)
  chunk.set([0x74, 0x45, 0x58, 0x74], 4)
  chunk.set(payload, 8)
  view.setUint32(chunk.length - 4, crc32(chunk.subarray(4, chunk.length - 4)))
  const out = new Uint8Array(bytes.length + chunk.length)
  out.set(bytes.subarray(0, insertAt), 0)
  out.set(chunk, insertAt)
  out.set(bytes.subarray(insertAt), insertAt + chunk.length)
  return out
}

/**
 * Largest `tEXt` payload this reader will decode.
 *
 * A sheet layout for the 256-glyph maximum is a few kilobytes; a chunk larger
 * than this is another tool's data and is skipped rather than decoded.
 */
export const MAX_PNG_TEXT_BYTES = 64 * 1024

/** `tEXt` is Latin-1 by specification, decoded without any per-byte spread. */
const LATIN1 = new TextDecoder('latin1')

/** Read the first `tEXt` chunk with this keyword, or null when there is none. */
export function readPngText(bytes: Uint8Array, keyword: string): string | null {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
  let offset = 8
  while (offset + 8 <= bytes.length) {
    const length = view.getUint32(offset)
    // A declared length past the end of the file means the PNG is truncated.
    if (length > bytes.length - offset - 12) return null
    const type = LATIN1.decode(bytes.subarray(offset + 4, offset + 8))
    if (type === 'IEND') return null
    if (type === 'tEXt' && length <= MAX_PNG_TEXT_BYTES) {
      const data = bytes.subarray(offset + 8, offset + 8 + length)
      const separator = data.indexOf(0)
      if (separator >= 0) {
        const found = LATIN1.decode(data.subarray(0, separator))
        if (found === keyword)
          return LATIN1.decode(data.subarray(separator + 1))
      }
    }
    offset += length + 12
  }
  return null
}

/** Decode only a PNG whose advertised size fits the editor's pixel and byte limits. */
export async function decodeSheetPng(
  file: File,
): Promise<SheetPixels & { layout?: SheetLayout }> {
  if (file.size > SHEET_MAX_BYTES) throw new SheetError('dimensions')
  const bytes = new Uint8Array(await file.arrayBuffer())
  const header = pngDimensions(bytes)
  const stored = readPngText(bytes, SHEET_LAYOUT_KEYWORD)
  const layout = stored ? decodeSheetLayout(stored) : null
  let bitmap: ImageBitmap | undefined
  const canvas = document.createElement('canvas')
  try {
    bitmap = await createImageBitmap(file)
    if (bitmap.width !== header.width || bitmap.height !== header.height)
      throw new SheetError('dimensions')
    canvas.width = header.width
    canvas.height = header.height
    const context = canvas.getContext('2d', { willReadFrequently: true })
    if (!context) throw new SheetError('decode')
    context.drawImage(bitmap, 0, 0)
    return {
      ...header,
      data: context.getImageData(0, 0, header.width, header.height).data,
      ...(layout ? { layout } : {}),
    }
  } catch (error) {
    throw error instanceof SheetError ? error : new SheetError('decode')
  } finally {
    bitmap?.close()
    canvas.width = 0
    canvas.height = 0
  }
}

/**
 * Produce a standard PNG; browsers may normalize hidden RGB in transparent
 * pixels. When a layout is supplied it travels with the file in a `tEXt` chunk,
 * so re-importing the sheet needs no remembered settings.
 */
export async function encodeSheetPng(
  image: SheetPixels,
  layout?: SheetLayout,
): Promise<Blob> {
  const canvas = document.createElement('canvas')
  canvas.width = image.width
  canvas.height = image.height
  try {
    const context = canvas.getContext('2d')
    if (!context) throw new SheetError('decode')
    const pixels = context.createImageData(image.width, image.height)
    pixels.data.set(image.data)
    context.putImageData(pixels, 0, 0)
    const blob = await new Promise<Blob>((resolve, reject) => {
      canvas.toBlob(
        (result) =>
          result ? resolve(result) : reject(new SheetError('decode')),
        'image/png',
      )
    })
    if (!layout) return blob
    const bytes = new Uint8Array(await blob.arrayBuffer())
    return new Blob(
      [embedPngText(bytes, SHEET_LAYOUT_KEYWORD, encodeSheetLayout(layout))],
      { type: 'image/png' },
    )
  } finally {
    canvas.width = 0
    canvas.height = 0
  }
}
