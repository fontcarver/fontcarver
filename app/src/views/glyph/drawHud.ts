/**
 * The cursor HUD pill (design spec §6.2).
 *
 * It is painted on the canvas rather than mounted as DOM: it only exists while
 * a gesture is running, it must land exactly at the pointer offset every frame,
 * and a React element would have to re-render at pointer rate to keep up.
 */

import type { Point2, Viewport } from '../../canvas/Viewport'

import type { GlyphTheme } from './glyphTheme'

/** Offset from the pointer, design spec §6.2. */
const OFFSET = 16
const PADDING_X = 7
const PADDING_Y = 4
const FONT_SIZE = 11
const RADIUS = 3

/** Draws the HUD pill at the pointer, if there is anything to say. */
export function drawHud(
  ctx: CanvasRenderingContext2D,
  viewport: Viewport,
  pointer: Point2 | null,
  text: string | null,
  theme: GlyphTheme,
): void {
  if (!pointer || !text) return
  ctx.save()
  ctx.font = `${FONT_SIZE}px ${theme.fontFamily}`
  ctx.textAlign = 'left'
  ctx.textBaseline = 'top'
  const width = ctx.measureText(text).width + PADDING_X * 2
  const height = FONT_SIZE + PADDING_Y * 2

  // Keep the pill on screen when the pointer nears the right or bottom edge.
  const x = Math.min(
    pointer.x + OFFSET,
    Math.max(0, viewport.width - width - 2),
  )
  const y = Math.min(
    pointer.y + OFFSET,
    Math.max(0, viewport.height - height - 2),
  )

  ctx.fillStyle = theme.hudBackground
  roundedRect(ctx, x, y, width, height, RADIUS)
  ctx.fill()
  ctx.fillStyle = theme.hudInk
  ctx.fillText(text, x + PADDING_X, y + PADDING_Y)
  ctx.restore()
}

function roundedRect(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  width: number,
  height: number,
  radius: number,
): void {
  ctx.beginPath()
  ctx.moveTo(x + radius, y)
  ctx.lineTo(x + width - radius, y)
  ctx.quadraticCurveTo(x + width, y, x + width, y + radius)
  ctx.lineTo(x + width, y + height - radius)
  ctx.quadraticCurveTo(x + width, y + height, x + width - radius, y + height)
  ctx.lineTo(x + radius, y + height)
  ctx.quadraticCurveTo(x, y + height, x, y + height - radius)
  ctx.lineTo(x, y + radius)
  ctx.quadraticCurveTo(x, y, x + radius, y)
  ctx.closePath()
}
