import type { GlyphId } from '../../api'
import type { FontBox, Point2, Viewport } from '../../canvas/Viewport'

import type { PositionedGlyph } from './textLayout'

export type GlyphBounds = [number, number, number, number]
export const MAX_VISIBLE_RUN_GLYPHS = 512
const HIT_MIN_PX = 8
const PREVIEW_WINDOW_PX = 4_096
const SMALLEST_PREVIEW_SIZE = 12

/** Outline bounds keyed by stable document glyph handle. */
export type TextGlyphBounds = ReadonlyMap<GlyphId, GlyphBounds>

function boundsFor(
  glyph: PositionedGlyph,
  bounds: TextGlyphBounds,
  unitsPerEm: number,
  scale: number,
): FontBox {
  const outline =
    glyph.documentGid === null ? undefined : bounds.get(glyph.documentGid)
  const minimum = HIT_MIN_PX / Math.max(scale, 0.0005)
  let x0 = Math.min(glyph.x, glyph.x + glyph.xAdvance)
  let x1 = Math.max(glyph.x, glyph.x + glyph.xAdvance)
  let y0 = glyph.y - unitsPerEm * 0.2
  let y1 = glyph.y + unitsPerEm * 0.8
  if (outline) {
    x0 = Math.min(x0, glyph.x + outline[0])
    x1 = Math.max(x1, glyph.x + outline[2])
    y0 = Math.min(y0, glyph.y + outline[1])
    y1 = Math.max(y1, glyph.y + outline[3])
  }
  if (x1 - x0 < minimum) {
    const centre = (x0 + x1) / 2
    x0 = centre - minimum / 2
    x1 = centre + minimum / 2
  }
  return { x0, y0, x1, y1 }
}

/** Bounds used by an initial Fit, deliberately capped for enormous runs. */
export function runBox(
  glyphs: readonly PositionedGlyph[],
  bounds: TextGlyphBounds,
  unitsPerEm: number,
): FontBox {
  const shown = glyphs.slice(0, MAX_VISIBLE_RUN_GLYPHS)
  if (shown.length === 0)
    return {
      x0: 0,
      y0: -unitsPerEm * 0.2,
      x1: unitsPerEm,
      y1: unitsPerEm * 0.8,
    }
  const boxes = shown.map((glyph) => boundsFor(glyph, bounds, unitsPerEm, 1))
  return boxes.reduce((box, next) => ({
    x0: Math.min(box.x0, next.x0),
    y0: Math.min(box.y0, next.y0),
    x1: Math.max(box.x1, next.x1),
    y1: Math.max(box.y1, next.y1),
  }))
}

/** Run glyph under an absolute run-space point, including blank advances. */
export function hitTextGlyph(
  glyphs: readonly PositionedGlyph[],
  bounds: TextGlyphBounds,
  point: Point2,
  unitsPerEm: number,
  scale: number,
): number | null {
  for (let index = glyphs.length - 1; index >= 0; index -= 1) {
    const glyph = glyphs[index]
    if (!glyph) continue
    const box = boundsFor(glyph, bounds, unitsPerEm, scale)
    if (
      point.x >= box.x0 &&
      point.x <= box.x1 &&
      point.y >= box.y0 &&
      point.y <= box.y1
    )
      return index
  }
  return null
}

function intersects(a: FontBox, b: FontBox): boolean {
  return a.x1 >= b.x0 && a.x0 <= b.x1 && a.y1 >= b.y0 && a.y0 <= b.y1
}

/** Visible glyph indexes, capped around the viewport centre for bounded IO. */
export function visibleGlyphIndexes(
  glyphs: readonly PositionedGlyph[],
  bounds: TextGlyphBounds,
  viewport: Pick<Viewport, 'width' | 'height' | 'scale' | 'toFont'>,
  activeOrigin: Point2,
  unitsPerEm: number,
): number[] {
  const topLeft = viewport.toFont({ x: 0, y: 0 })
  const bottomRight = viewport.toFont({ x: viewport.width, y: viewport.height })
  const margin = unitsPerEm
  const visible: FontBox = {
    x0: Math.min(topLeft.x, bottomRight.x) + activeOrigin.x - margin,
    y0: Math.min(topLeft.y, bottomRight.y) + activeOrigin.y - margin,
    x1: Math.max(topLeft.x, bottomRight.x) + activeOrigin.x + margin,
    y1: Math.max(topLeft.y, bottomRight.y) + activeOrigin.y + margin,
  }
  const matches = glyphs.flatMap((glyph, index) =>
    intersects(boundsFor(glyph, bounds, unitsPerEm, viewport.scale), visible)
      ? [index]
      : [],
  )
  if (matches.length <= MAX_VISIBLE_RUN_GLYPHS) return matches
  const centre = (visible.x0 + visible.x1) / 2
  return matches
    .sort(
      (left, right) =>
        Math.abs((glyphs[left]?.x ?? 0) - centre) -
        Math.abs((glyphs[right]?.x ?? 0) - centre),
    )
    .slice(0, MAX_VISIBLE_RUN_GLYPHS)
    .sort((left, right) => left - right)
}

/** Glyphs whose outlines can affect the fixed-width preview window. */
export function previewGlyphIndexes(
  glyphs: readonly PositionedGlyph[],
  unitsPerEm: number,
): number[] {
  if (glyphs.length === 0) return []
  const firstX = glyphs[0]?.x ?? 0
  const extent = (PREVIEW_WINDOW_PX * unitsPerEm) / SMALLEST_PREVIEW_SIZE
  return glyphs
    .flatMap((glyph, index) =>
      Math.abs(glyph.x - firstX) <= extent ? [index] : [],
    )
    .slice(0, MAX_VISIBLE_RUN_GLYPHS)
}
