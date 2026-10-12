/**
 * What the drawing tools put on the canvas (design spec §4.1 items 1 and 14).
 *
 * The optional grid goes behind everything; the tool overlay — the pen's path
 * and handles, the knife's cut line, the measure's spans, the origin marker of
 * a transform — goes in front of the outline, because it is what the pointer
 * is doing right now and must never be hidden by the shape being drawn.
 */

import { drawAnnotations } from './drawAnnotations'

import type { Point2, Viewport } from '../../canvas/Viewport'
import type { ToolOverlay } from '../../editor/tools/toolTypes'

import { traceContours } from './drawOutline'
import type { GlyphTheme } from './glyphTheme'

/** Node markers of a tool preview, in CSS pixels. */
const MARK_RADIUS = 2.5
/** The cross drawn at a rotate or scale origin, in CSS pixels. */
const ORIGIN_SIZE = 7
/**
 * Grid lines closer together than this on screen are not drawn.
 *
 * §4.1 item 1 says "≥ 1 px spacing on screen", which on a modern display is a
 * solid wash rather than a grid: a 1-unit grid at a readable zoom would paint
 * a 0.5 px line every device pixel. Four CSS pixels is the point at which the
 * lines still read as lines, so the grid disappears below it instead of
 * greying the canvas out.
 */
export const MIN_GRID_SPACING = 4

/**
 * Draws the editing grid.
 *
 * The step is a preference in font units; below `MIN_GRID_SPACING` on screen
 * it is skipped rather than drawn as a solid wash.
 */
export function drawGrid(
  ctx: CanvasRenderingContext2D,
  viewport: Viewport,
  step: number,
  theme: GlyphTheme,
): void {
  if (step <= 0) return
  const spacing = step * viewport.scale
  if (spacing < MIN_GRID_SPACING) return
  const topLeft = viewport.toFont({ x: 0, y: 0 })
  const bottomRight = viewport.toFont({ x: viewport.width, y: viewport.height })
  ctx.save()
  ctx.strokeStyle = theme.grid
  ctx.lineWidth = 0.5
  ctx.beginPath()
  const firstX = Math.ceil(topLeft.x / step) * step
  for (let x = firstX; x <= bottomRight.x; x += step) {
    const at = Math.round(viewport.toScreen({ x, y: 0 }).x) + 0.5
    ctx.moveTo(at, 0)
    ctx.lineTo(at, viewport.height)
  }
  const firstY = Math.ceil(bottomRight.y / step) * step
  for (let y = firstY; y <= topLeft.y; y += step) {
    const at = Math.round(viewport.toScreen({ x: 0, y }).y) + 0.5
    ctx.moveTo(0, at)
    ctx.lineTo(viewport.width, at)
  }
  ctx.stroke()
  ctx.restore()
}

/** Draws whatever the active tool is showing this frame. */
export function drawToolOverlay(
  ctx: CanvasRenderingContext2D,
  viewport: Viewport,
  overlay: ToolOverlay,
  theme: GlyphTheme,
): void {
  drawAnnotations(ctx, viewport, overlay.annotations ?? [], theme)
  const toScreen = (point: Point2) => viewport.toScreen(point)
  ctx.save()

  if (overlay.contours.length > 0) {
    ctx.strokeStyle = theme.accent
    ctx.lineWidth = 1
    ctx.beginPath()
    traceContours(ctx, overlay.contours, toScreen)
    ctx.stroke()
  }

  if (overlay.lines.length > 0) {
    ctx.strokeStyle = theme.accent
    ctx.lineWidth = 1
    for (const line of overlay.lines) {
      ctx.setLineDash(line.dashed === true ? [4, 3] : [])
      const from = toScreen(line.from)
      const to = toScreen(line.to)
      ctx.beginPath()
      ctx.moveTo(from.x, from.y)
      ctx.lineTo(to.x, to.y)
      ctx.stroke()
    }
    ctx.setLineDash([])
  }

  if (overlay.marks.length > 0) {
    ctx.fillStyle = theme.accent
    for (const mark of overlay.marks) {
      const at = toScreen(mark)
      ctx.beginPath()
      ctx.arc(at.x, at.y, MARK_RADIUS, 0, Math.PI * 2)
      ctx.fill()
    }
  }

  if (overlay.origin) {
    const at = toScreen(overlay.origin)
    ctx.strokeStyle = theme.accent
    ctx.lineWidth = 1
    ctx.beginPath()
    ctx.moveTo(at.x - ORIGIN_SIZE, at.y)
    ctx.lineTo(at.x + ORIGIN_SIZE, at.y)
    ctx.moveTo(at.x, at.y - ORIGIN_SIZE)
    ctx.lineTo(at.x, at.y + ORIGIN_SIZE)
    ctx.stroke()
    ctx.beginPath()
    ctx.arc(at.x, at.y, ORIGIN_SIZE / 2, 0, Math.PI * 2)
    ctx.stroke()
  }

  if (overlay.labels.length > 0) {
    ctx.font = `11px ${theme.fontFamily}`
    ctx.fillStyle = theme.accent
    ctx.textAlign = 'center'
    ctx.textBaseline = 'bottom'
    for (const label of overlay.labels) {
      const at = toScreen(label.at)
      ctx.fillText(label.text, at.x, at.y - 4)
    }
  }

  ctx.restore()
}
