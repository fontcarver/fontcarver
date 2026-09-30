import type { BitmapGlyph } from '../../api'
import { blank, crop, put, sample, type PixelSelection } from './pixels'
export function moveSelection(
  source: BitmapGlyph,
  region: PixelSelection,
  dx: number,
  dy: number,
): BitmapGlyph {
  const next = structuredClone(source),
    patch = crop(source.bitmap, region)
  for (let y = 0; y < region.height; y++)
    for (let x = 0; x < region.width; x++)
      put(next.bitmap, region.x + x, region.y + y, 0)
  for (let y = 0; y < region.height; y++)
    for (let x = 0; x < region.width; x++)
      put(
        next.bitmap,
        region.x + x + dx,
        region.y + y + dy,
        sample(patch, x, y),
      )
  return next
}
export function cropGlyph(
  source: BitmapGlyph,
  region?: PixelSelection,
): BitmapGlyph {
  const b = source.bitmap
  if (!region) {
    let left = b.width,
      top = b.height,
      right = -1,
      bottom = -1
    for (let y = 0; y < b.height; y++)
      for (let x = 0; x < b.width; x++) {
        const v = sample(b, x, y)
        if (b.depth === 32 ? (v & 255) > 0 : v > 0) {
          left = Math.min(left, x)
          top = Math.min(top, y)
          right = Math.max(right, x)
          bottom = Math.max(bottom, y)
        }
      }
    region =
      right < 0
        ? { x: 0, y: 0, width: 0, height: 0 }
        : { x: left, y: top, width: right - left + 1, height: bottom - top + 1 }
  }
  const out = crop(b, region)
  out.origin_x = b.origin_x + region.x
  out.origin_y = b.origin_y + b.height - region.y - region.height
  return { ...source, bitmap: out }
}
export function invert(
  source: BitmapGlyph,
  region?: PixelSelection,
): BitmapGlyph {
  const result = structuredClone(source),
    b = result.bitmap,
    r = region ?? { x: 0, y: 0, width: b.width, height: b.height }
  for (let y = r.y; y < r.y + r.height; y++)
    for (let x = r.x; x < r.x + r.width; x++) {
      const v = sample(b, x, y)
      put(
        b,
        x,
        y,
        b.depth === 32 ? (v ^ 0xffffff00) >>> 0 : (1 << b.depth) - 1 - v,
      )
    }
  return result
}
/** Resample stored pixels, retaining the glyph's baseline origin and advances. */
export function resample(
  source: BitmapGlyph,
  width: number,
  height: number,
  smooth: boolean,
): BitmapGlyph {
  const src = source.bitmap,
    out = blank(width, height, src.depth)
  out.origin_x = src.origin_x
  out.origin_y = src.origin_y
  if (!src.width || !src.height) return { ...source, bitmap: out }
  for (let y = 0; y < height; y++)
    for (let x = 0; x < width; x++) {
      if (!smooth) {
        put(
          out,
          x,
          y,
          sample(
            src,
            Math.min(
              src.width - 1,
              Math.floor(((x + 0.5) * src.width) / width),
            ),
            Math.min(
              src.height - 1,
              Math.floor(((y + 0.5) * src.height) / height),
            ),
          ),
        )
        continue
      }
      const fx = Math.max(0, ((x + 0.5) * src.width) / width - 0.5),
        fy = Math.max(0, ((y + 0.5) * src.height) / height - 0.5),
        ix = Math.floor(fx),
        iy = Math.floor(fy),
        a = fx - ix,
        b = fy - iy
      const samples = [
        [ix, iy, (1 - a) * (1 - b)],
        [Math.min(ix + 1, src.width - 1), iy, a * (1 - b)],
        [ix, Math.min(iy + 1, src.height - 1), (1 - a) * b],
        [
          Math.min(ix + 1, src.width - 1),
          Math.min(iy + 1, src.height - 1),
          a * b,
        ],
      ]
      if (src.depth !== 32) {
        put(
          out,
          x,
          y,
          Math.round(
            samples.reduce((n, [sx, sy, w]) => n + sample(src, sx, sy) * w, 0),
          ),
        )
        continue
      }
      const rgba = [0, 0, 0, 0]
      for (const [sx, sy, w] of samples) {
        const v = sample(src, sx, sy),
          alpha = (v & 255) / 255
        rgba[0] += (v >>> 24) * alpha * w
        rgba[1] += ((v >>> 16) & 255) * alpha * w
        rgba[2] += ((v >>> 8) & 255) * alpha * w
        rgba[3] += alpha * w
      }
      const alpha = rgba[3]
      put(
        out,
        x,
        y,
        alpha
          ? (Math.round(rgba[0] / alpha) * 0x1000000 +
              (Math.round(rgba[1] / alpha) << 16) +
              (Math.round(rgba[2] / alpha) << 8) +
              Math.round(alpha * 255)) >>>
              0
          : 0,
      )
    }
  return { ...source, bitmap: out }
}
