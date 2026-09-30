import type { Bitmap } from '../api'
import type { PixelPoint } from '../views/bitmap/pixels'

/** Visible pixel-space bounds, including a one-pixel guide margin. */
export interface BitmapFrame {
  left: number
  top: number
  width: number
  height: number
}

/** Include the editable pixels, resolved references, baseline and advance. */
export function bitmapFrame(
  bitmap: Bitmap,
  advance: number,
  resolved?: Bitmap,
): BitmapFrame {
  const images = resolved ? [bitmap, resolved] : [bitmap]
  const left = Math.min(0, advance, ...images.map((b) => b.origin_x)) - 1
  const right =
    Math.max(0, advance, ...images.map((b) => b.origin_x + b.width)) + 1
  const bottom = Math.min(0, ...images.map((b) => b.origin_y)) - 1
  const top = Math.max(0, ...images.map((b) => b.origin_y + b.height)) + 1
  return { left, top, width: right - left, height: top - bottom }
}

/** Convert coordinates measured from the frame's top-left into source pixels. */
export function bitmapPointer(
  bitmap: Bitmap,
  frame: BitmapFrame,
  x: number,
  y: number,
): PixelPoint {
  return [
    Math.floor(frame.left + x - bitmap.origin_x),
    Math.floor(y - frame.top + bitmap.origin_y + bitmap.height),
  ]
}
