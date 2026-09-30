import {
  pngDimensions,
  SHEET_MAX_BYTES,
  SheetError,
  type SheetPixels,
} from '../views/bitmap/bitmapSheet'

/** Decode only a PNG whose advertised size fits the editor's pixel and byte limits. */
export async function decodeSheetPng(file: File): Promise<SheetPixels> {
  if (file.size > SHEET_MAX_BYTES) throw new SheetError('dimensions')
  const header = pngDimensions(
    new Uint8Array(await file.slice(0, 33).arrayBuffer()),
  )
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
    }
  } catch (error) {
    throw error instanceof SheetError ? error : new SheetError('decode')
  } finally {
    bitmap?.close()
    canvas.width = 0
    canvas.height = 0
  }
}

/** Produce a standard PNG; browsers may normalize hidden RGB in transparent pixels. */
export async function encodeSheetPng(image: SheetPixels): Promise<Blob> {
  const canvas = document.createElement('canvas')
  canvas.width = image.width
  canvas.height = image.height
  try {
    const context = canvas.getContext('2d')
    if (!context) throw new SheetError('decode')
    const pixels = context.createImageData(image.width, image.height)
    pixels.data.set(image.data)
    context.putImageData(pixels, 0, 0)
    return await new Promise<Blob>((resolve, reject) => {
      canvas.toBlob(
        (blob) => (blob ? resolve(blob) : reject(new SheetError('decode'))),
        'image/png',
      )
    })
  } finally {
    canvas.width = 0
    canvas.height = 0
  }
}
