/**
 * The layers drawn *behind* the one being edited (design spec §4.1 items 4
 * and 5): the same glyph in the other masters, and this master's background
 * layer.
 *
 * Both are references rather than editable geometry, so they are stroked thin
 * and carry no handles: the other masters have no nodes at all, and the
 * background gets 3 px dots so its points can be read off without inviting a
 * click. The paths come from `glyphs.detail`, which already returns one
 * flattened path per layer, so an underlay costs no extra request.
 */

import type { Point2, Viewport } from '../../canvas/Viewport'

import type { GlyphTheme } from './glyphTheme'

/** Radius of a background layer's point markers, in CSS pixels. */
const BACKGROUND_DOT = 1.5

/** What `drawUnderlays` renders. */
export interface UnderlayData {
  /** Flattened path of the background layer, in font units. */
  background: Path2D | null
  /** Stored points of the background layer, for its dots. */
  backgroundPoints: readonly Point2[]
  /** Flattened paths of the same glyph in the other masters. */
  otherMasters: readonly Path2D[]
}

/** Nothing behind the outline. */
export const NO_UNDERLAY: UnderlayData = {
  background: null,
  backgroundPoints: [],
  otherMasters: [],
}

function strokeFontPath(
  ctx: CanvasRenderingContext2D,
  viewport: Viewport,
  path: Path2D,
  colour: string,
): void {
  ctx.save()
  ctx.translate(viewport.offset.x, viewport.offset.y)
  ctx.scale(viewport.scale, -viewport.scale)
  ctx.strokeStyle = colour
  ctx.lineWidth = 1 / (viewport.scale || 1)
  ctx.stroke(path)
  ctx.restore()
}

/** Draws the other-master and background underlays. */
export function drawUnderlays(
  ctx: CanvasRenderingContext2D,
  viewport: Viewport,
  data: UnderlayData,
  theme: GlyphTheme,
): void {
  for (const path of data.otherMasters)
    strokeFontPath(ctx, viewport, path, theme.otherMaster)

  if (data.background) {
    strokeFontPath(ctx, viewport, data.background, theme.backgroundLayer)
  }
  if (data.backgroundPoints.length === 0) return

  ctx.save()
  ctx.fillStyle = theme.backgroundLayer
  for (const point of data.backgroundPoints) {
    const at = viewport.toScreen(point)
    ctx.beginPath()
    ctx.arc(at.x, at.y, BACKGROUND_DOT, 0, Math.PI * 2)
    ctx.fill()
  }
  ctx.restore()
}
