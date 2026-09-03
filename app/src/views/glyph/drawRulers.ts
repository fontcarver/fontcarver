/**
 * Rulers along the top and left edges, ticked in font units.
 *
 * The tick step is chosen from the 1 / 2 / 5 × 10ⁿ series so that a labelled
 * tick is never closer than `MIN_LABEL_GAP` screen pixels, which keeps the
 * scale readable from a 30-unit detail zoom out to a whole 65k-glyph em.
 */

import type { Viewport } from '../../canvas/Viewport'

import type { GlyphTheme } from './glyphTheme'

/** Thickness of each ruler strip, in CSS pixels. */
export const RULER_SIZE = 18

const MIN_LABEL_GAP = 56
const MINOR_PER_MAJOR = 5
const LABEL_FONT_SIZE = 9

/**
 * The smallest 1 / 2 / 5 × 10ⁿ step whose on-screen spacing is at least
 * `minPixels`. `scale` is CSS pixels per font unit.
 */
export function tickStep(scale: number, minPixels = MIN_LABEL_GAP): number {
  if (!Number.isFinite(scale) || scale <= 0) return 1
  const raw = minPixels / scale
  const magnitude = 10 ** Math.floor(Math.log10(raw))
  for (const factor of [1, 2, 5, 10]) {
    const step = factor * magnitude
    if (step >= raw) return step
  }
  return 10 * magnitude
}

/** What `drawRulers` renders. */
export interface RulersData {
  /** Pointer position in font units, marked on both rulers when present. */
  pointer: { x: number; y: number } | null
}

function drawTick(
  ctx: CanvasRenderingContext2D,
  x0: number,
  y0: number,
  x1: number,
  y1: number,
): void {
  ctx.beginPath()
  ctx.moveTo(x0, y0)
  ctx.lineTo(x1, y1)
  ctx.stroke()
}

/** Draws the horizontal and vertical rulers over the canvas content. */
export function drawRulers(
  ctx: CanvasRenderingContext2D,
  viewport: Viewport,
  data: RulersData,
  theme: GlyphTheme,
): void {
  const { width, height } = viewport
  if (width <= 0 || height <= 0) return
  const step = tickStep(viewport.scale)
  const minor = step / MINOR_PER_MAJOR

  ctx.save()
  ctx.fillStyle = theme.rulerBackground
  ctx.fillRect(0, 0, width, RULER_SIZE)
  ctx.fillRect(0, 0, RULER_SIZE, height)
  ctx.strokeStyle = theme.rulerLine
  ctx.lineWidth = 1
  drawTick(ctx, 0, RULER_SIZE + 0.5, width, RULER_SIZE + 0.5)
  drawTick(ctx, RULER_SIZE + 0.5, 0, RULER_SIZE + 0.5, height)

  ctx.font = `${LABEL_FONT_SIZE}px ${theme.fontFamily}`
  ctx.fillStyle = theme.rulerLabel
  ctx.textBaseline = 'top'
  ctx.textAlign = 'left'

  const left = viewport.toFont({ x: RULER_SIZE, y: 0 }).x
  const right = viewport.toFont({ x: width, y: 0 }).x
  for (
    let value = Math.ceil(left / minor) * minor;
    value <= right;
    value += minor
  ) {
    const x = Math.round(viewport.toScreen({ x: value, y: 0 }).x) + 0.5
    const major = Math.abs(value % step) < minor / 2
    drawTick(ctx, x, major ? 6 : 12, x, RULER_SIZE)
    if (major) ctx.fillText(String(Math.round(value)), x + 2, 1)
  }

  // The vertical ruler counts upwards, so it walks from the bottom edge up.
  const bottom = viewport.toFont({ x: 0, y: height }).y
  const top = viewport.toFont({ x: 0, y: RULER_SIZE }).y
  ctx.textAlign = 'left'
  for (
    let value = Math.ceil(bottom / minor) * minor;
    value <= top;
    value += minor
  ) {
    const y = Math.round(viewport.toScreen({ x: 0, y: value }).y) + 0.5
    const major = Math.abs(value % step) < minor / 2
    drawTick(ctx, major ? 6 : 12, y, RULER_SIZE, y)
    if (major) ctx.fillText(String(Math.round(value)), 1, y + 1)
  }

  if (data.pointer) {
    const at = viewport.toScreen(data.pointer)
    ctx.strokeStyle = theme.guideLocal
    drawTick(ctx, Math.round(at.x) + 0.5, 0, Math.round(at.x) + 0.5, RULER_SIZE)
    drawTick(ctx, 0, Math.round(at.y) + 0.5, RULER_SIZE, Math.round(at.y) + 0.5)
  }

  ctx.fillStyle = theme.rulerBackground
  ctx.fillRect(0, 0, RULER_SIZE, RULER_SIZE)
  ctx.strokeStyle = theme.rulerLine
  ctx.strokeRect(0.5, 0.5, RULER_SIZE, RULER_SIZE)
  ctx.restore()
}
