import type { BitmapGlyph } from '../../api'
import { blank, put, sample } from './pixels'

export const SHEET_MAX_BYTES = 16 * 1024 * 1024
export const SHEET_MAX_PIXELS = 4 * 1024 * 1024
export const SHEET_MAX_GLYPHS = 256

export interface SheetOptions {
  cellWidth: number
  cellHeight: number
  columns: number
}

export interface SheetPixels {
  width: number
  height: number
  data: Uint8ClampedArray
}

/** A stable code that the UI translates without exposing decoder diagnostics. */
export class SheetError extends Error {
  constructor(
    readonly code:
      | 'mapping'
      | 'dimensions'
      | 'png'
      | 'clipped'
      | 'padding'
      | 'changed'
      | 'missing'
      | 'decode',
  ) {
    super(code)
  }
}

export function parseSheetNames(text: string): string[] {
  const names = text
    .trim()
    .split(/[\s,]+/u)
    .filter(Boolean)
  if (
    !names.length ||
    names.length > SHEET_MAX_GLYPHS ||
    new Set(names).size !== names.length
  )
    throw new SheetError('mapping')
  return names
}

function checkDimensions(width: number, height: number) {
  if (
    ![width, height].every(
      (n) => Number.isSafeInteger(n) && n > 0 && n <= 8192,
    ) ||
    width * height > SHEET_MAX_PIXELS
  )
    throw new SheetError('dimensions')
  return { width, height }
}

export function sheetDimensions(count: number, options: SheetOptions) {
  if (
    !Number.isInteger(count) ||
    count < 1 ||
    count > SHEET_MAX_GLYPHS ||
    ![options.cellWidth, options.cellHeight, options.columns].every(
      (n) => Number.isSafeInteger(n) && n > 0,
    ) ||
    options.cellWidth > 4096 ||
    options.cellHeight > 4096 ||
    options.columns > SHEET_MAX_GLYPHS
  )
    throw new SheetError('dimensions')
  return checkDimensions(
    options.cellWidth * options.columns,
    Math.ceil(count / options.columns) * options.cellHeight,
  )
}

/** Read the bounded header before allocating or passing data to an image decoder. */
export function pngDimensions(bytes: Uint8Array) {
  const signature = [
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82,
  ]
  if (bytes.length < 33 || !signature.every((v, i) => bytes[i] === v))
    throw new SheetError('png')
  const data = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
  return checkDimensions(data.getUint32(16), data.getUint32(20))
}

/** Encode a fixed grid; row bytes are top-down and every cell has a bottom-left origin. */
export function exportSheetPixels(
  glyphs: BitmapGlyph[],
  options: SheetOptions,
): SheetPixels {
  const { width, height } = sheetDimensions(glyphs.length, options)
  const data = new Uint8ClampedArray(width * height * 4)
  glyphs.forEach(({ bitmap: b }, index) => {
    if (b.width > options.cellWidth || b.height > options.cellHeight)
      throw new SheetError('clipped')
    const left = (index % options.columns) * options.cellWidth
    const top =
      Math.floor(index / options.columns) * options.cellHeight +
      options.cellHeight -
      b.height
    for (let y = 0; y < b.height; y++)
      for (let x = 0; x < b.width; x++) {
        const at = ((top + y) * width + left + x) * 4,
          value = sample(b, x, y)
        if (b.depth === 32) {
          data[at] = value >>> 24
          data[at + 1] = (value >>> 16) & 255
          data[at + 2] = (value >>> 8) & 255
          data[at + 3] = value & 255
        } else data[at + 3] = Math.round((value * 255) / ((1 << b.depth) - 1))
      }
  })
  return { width, height, data }
}

/** Replace selected pixels while retaining existing metrics and rejecting lost ink. */
export function importSheetPixels(
  image: SheetPixels,
  names: string[],
  options: SheetOptions,
  depth: number,
  descent: number,
  existing: Record<string, BitmapGlyph>,
): Record<string, BitmapGlyph> {
  const expected = sheetDimensions(names.length, options)
  if (new Set(names).size !== names.length) throw new SheetError('mapping')
  if (
    expected.width !== image.width ||
    expected.height !== image.height ||
    image.data.length !== image.width * image.height * 4
  )
    throw new SheetError('dimensions')
  const result: Record<string, BitmapGlyph> = Object.create(null)
  const rows = Math.ceil(names.length / options.columns)
  for (let index = 0; index < rows * options.columns; index++) {
    const name = names[index],
      previous =
        name === undefined || !Object.hasOwn(existing, name)
          ? undefined
          : existing[name]
    const width = previous?.bitmap.width ?? options.cellWidth
    const height = previous?.bitmap.height ?? options.cellHeight
    if (width > options.cellWidth || height > options.cellHeight)
      throw new SheetError('clipped')
    const bitmap =
      name === undefined
        ? undefined
        : {
            ...blank(width, height, depth),
            origin_x: previous?.bitmap.origin_x ?? 0,
            origin_y: previous?.bitmap.origin_y ?? -descent,
          }
    const left = (index % options.columns) * options.cellWidth
    const top = Math.floor(index / options.columns) * options.cellHeight
    const padding = options.cellHeight - height
    for (let y = 0; y < options.cellHeight; y++)
      for (let x = 0; x < options.cellWidth; x++) {
        const at = ((top + y) * image.width + left + x) * 4
        if (!bitmap || x >= width || y < padding) {
          if (image.data[at + 3] !== 0) throw new SheetError('padding')
          continue
        }
        const value =
          depth === 32
            ? (image.data[at] * 0x1000000 +
                (image.data[at + 1] << 16) +
                (image.data[at + 2] << 8) +
                image.data[at + 3]) >>>
              0
            : Math.round((image.data[at + 3] * ((1 << depth) - 1)) / 255)
        put(bitmap, x, y - padding, value)
      }
    if (bitmap)
      result[name] = {
        ...previous,
        bitmap,
        advance: previous?.advance ?? options.cellWidth,
        vertical_advance: previous?.vertical_advance ?? null,
        refs: [],
      }
  }
  return result
}
