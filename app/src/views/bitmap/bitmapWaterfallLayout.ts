import type { BitmapGlyph, BitmapPreviewLine } from '../../api'

/** Pixel positions relative to a bounded canvas, without scaling or rotating ink. */
export function layoutBitmapWaterfall(
  line: BitmapPreviewLine,
  vertical: boolean,
) {
  const placements: { glyph: BitmapGlyph; x: number; y: number }[] = []
  let pen = 0
  let left = vertical ? -line.ppem / 2 : 0
  let right = vertical ? line.ppem / 2 : 0
  let top = vertical ? 0 : -line.ascent
  let bottom = vertical ? 0 : line.descent
  for (const glyph of line.glyphs) {
    if (glyph) {
      // The kernel resolves document/VORG inheritance. Legacy callers with no
      // vertical metrics use the strike ascent, half advance and one pixel em.
      const [originX, originY] = glyph.vertical_origin ?? [
        glyph.advance / 2,
        line.ascent,
      ]
      const x = glyph.bitmap.origin_x + (vertical ? -originX : pen)
      const y =
        -glyph.bitmap.height -
        glyph.bitmap.origin_y +
        (vertical ? pen + originY : 0)
      placements.push({ glyph, x, y })
      left = Math.min(left, x)
      right = Math.max(right, x + glyph.bitmap.width)
      top = Math.min(top, y)
      bottom = Math.max(bottom, y + glyph.bitmap.height)
    }
    pen += vertical
      ? (glyph?.vertical_advance ?? line.ppem)
      : (glyph?.advance ?? line.ppem)
    if (vertical) {
      top = Math.min(top, pen)
      bottom = Math.max(bottom, pen)
    } else {
      left = Math.min(left, pen)
      right = Math.max(right, pen)
    }
  }
  left = Math.floor(left) - (vertical ? 0 : line.ppem)
  top = Math.floor(top) - (vertical ? line.ppem : 0)
  const wantedWidth = Math.max(
    1,
    Math.ceil(right) - left + (vertical ? 0 : line.ppem),
  )
  const wantedHeight = Math.max(
    1,
    Math.ceil(bottom) - top + (vertical ? line.ppem : 0),
  )
  const width = Math.min(16384, wantedWidth)
  const height = Math.min(16384, wantedHeight, Math.floor(4_000_000 / width))
  return {
    width,
    height,
    clipped: width < wantedWidth || height < wantedHeight,
    placements: placements.map(({ glyph, x, y }) => ({
      glyph,
      x: x - left,
      y: y - top,
    })),
  }
}
