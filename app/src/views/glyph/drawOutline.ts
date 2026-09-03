/**
 * Outline drawing: contour paths, on-curve and off-curve points, handles,
 * start markers and path-direction arrows.
 *
 * Contours arrive as UFO-style point lists, which express cubic and quadratic
 * segments in the same structure — including TrueType contours whose on-curve
 * points are implied, and contours that begin on an off-curve point.
 * `resolveContour` turns one such list into an explicit sequence of segments
 * whose end points are always on-curve. The path, nodes, handles and start
 * marker are all derived from that sequence, so the overlays cannot disagree
 * with the outline they annotate.
 */

import type { Contour } from '../../api'
import type { Point2, Viewport } from '../../canvas/Viewport'

import {
  controlPoints,
  directionTarget,
  handleLines,
  onCurvePoints,
  resolveContour,
  type ResolvedContour,
} from './contourGeometry'
import type { GlyphTheme } from './glyphTheme'

/** The subset of the canvas path API the tracer needs. */
export interface PathSink {
  moveTo(x: number, y: number): void
  lineTo(x: number, y: number): void
  quadraticCurveTo(cx: number, cy: number, x: number, y: number): void
  bezierCurveTo(
    c1x: number,
    c1y: number,
    c2x: number,
    c2y: number,
    x: number,
    y: number,
  ): void
  closePath(): void
}

/** Maps a font-space point into the space the sink draws in. */
export type PointMap = (point: Point2) => Point2

/** Node sizes in CSS pixels. */
const CORNER_SIZE = 6
const SMOOTH_RADIUS = 3.6
const IMPLIED_RADIUS = 3.4
const OFF_CURVE_RADIUS = 2.6
const START_RADIUS = 6
const ARROW_LENGTH = 9

/** Traces one contour into a path sink, mapping every point through `map`. */
export function traceContour(
  sink: PathSink,
  contour: Contour,
  map: PointMap,
): void {
  const resolved = resolveContour(contour)
  if (!resolved.start) return
  const start = map(resolved.start)
  sink.moveTo(start.x, start.y)
  for (const segment of resolved.segments) {
    const to = map(segment.to)
    if (segment.kind === 'line') {
      sink.lineTo(to.x, to.y)
    } else if (segment.kind === 'quad') {
      const control = map(segment.control)
      sink.quadraticCurveTo(control.x, control.y, to.x, to.y)
    } else {
      const c1 = map(segment.c1)
      const c2 = map(segment.c2)
      sink.bezierCurveTo(c1.x, c1.y, c2.x, c2.y, to.x, to.y)
    }
  }
  if (resolved.closed) sink.closePath()
}

/** Traces every contour of a layer into one path. */
export function traceContours(
  sink: PathSink,
  contours: readonly Contour[],
  map: PointMap,
): void {
  for (const contour of contours) traceContour(sink, contour, map)
}

function drawSquare(
  ctx: CanvasRenderingContext2D,
  centre: Point2,
  size: number,
): void {
  ctx.fillRect(centre.x - size / 2, centre.y - size / 2, size, size)
}

function drawCircle(
  ctx: CanvasRenderingContext2D,
  centre: Point2,
  radius: number,
): void {
  ctx.beginPath()
  ctx.arc(centre.x, centre.y, radius, 0, Math.PI * 2)
  ctx.fill()
}

/** A hollow diamond, marking an on-curve point the font leaves implied. */
function drawDiamond(
  ctx: CanvasRenderingContext2D,
  centre: Point2,
  radius: number,
): void {
  ctx.beginPath()
  ctx.moveTo(centre.x, centre.y - radius)
  ctx.lineTo(centre.x + radius, centre.y)
  ctx.lineTo(centre.x, centre.y + radius)
  ctx.lineTo(centre.x - radius, centre.y)
  ctx.closePath()
  ctx.stroke()
}

/** Draws the handle lines that join controls to the points they govern. */
function drawHandles(
  ctx: CanvasRenderingContext2D,
  resolved: ResolvedContour,
  map: PointMap,
  theme: GlyphTheme,
): void {
  const lines = handleLines(resolved)
  if (lines.length === 0) return
  ctx.strokeStyle = theme.handle
  ctx.lineWidth = 1
  ctx.beginPath()
  for (const [control, node] of lines) {
    const from = map(control)
    const to = map(node)
    ctx.moveTo(from.x, from.y)
    ctx.lineTo(to.x, to.y)
  }
  ctx.stroke()
}

/** Draws the start-point ring and the arrow showing the path direction. */
function drawStartMarker(
  ctx: CanvasRenderingContext2D,
  resolved: ResolvedContour,
  map: PointMap,
  theme: GlyphTheme,
): void {
  if (!resolved.start) return
  const start = map(resolved.start)

  ctx.strokeStyle = theme.start
  ctx.lineWidth = 1.5
  ctx.beginPath()
  ctx.arc(start.x, start.y, START_RADIUS, 0, Math.PI * 2)
  ctx.stroke()

  const target = directionTarget(resolved)
  if (!target) return
  const to = map(target)
  const dx = to.x - start.x
  const dy = to.y - start.y
  const length = Math.hypot(dx, dy)
  if (length < 1) return
  const ux = dx / length
  const uy = dy / length
  const tip = {
    x: start.x + ux * (START_RADIUS + ARROW_LENGTH),
    y: start.y + uy * (START_RADIUS + ARROW_LENGTH),
  }
  const base = {
    x: start.x + ux * (START_RADIUS + 2),
    y: start.y + uy * (START_RADIUS + 2),
  }
  const half = 3.2
  ctx.fillStyle = theme.start
  ctx.beginPath()
  ctx.moveTo(tip.x, tip.y)
  ctx.lineTo(base.x - uy * half, base.y + ux * half)
  ctx.lineTo(base.x + uy * half, base.y - ux * half)
  ctx.closePath()
  ctx.fill()
}

/** Draws the on-curve and off-curve nodes of one resolved contour. */
function drawNodes(
  ctx: CanvasRenderingContext2D,
  resolved: ResolvedContour,
  map: PointMap,
  theme: GlyphTheme,
): void {
  ctx.fillStyle = theme.nodeOffCurve
  for (const control of controlPoints(resolved))
    drawCircle(ctx, map(control), OFF_CURVE_RADIUS)

  for (const node of onCurvePoints(resolved)) {
    const at = map(node)
    if (node.source === null) {
      // An on-curve point the font leaves implied gets its own hollow marker,
      // so a TrueType outline still reads as the point list it really is.
      ctx.strokeStyle = theme.nodeImplied
      ctx.lineWidth = 1
      drawDiamond(ctx, at, IMPLIED_RADIUS)
    } else if (node.source.smooth) {
      ctx.fillStyle = theme.nodeSmooth
      drawCircle(ctx, at, SMOOTH_RADIUS)
    } else {
      ctx.fillStyle = theme.nodeCorner
      drawSquare(ctx, at, CORNER_SIZE)
    }
  }
}

/** What `drawOutline` renders. */
export interface OutlineData {
  contours: readonly Contour[]
  /** Fill the outline instead of stroking it (the space-bar preview). */
  filled: boolean
  /** Draw nodes, handles, start markers and direction arrows. */
  showPoints: boolean
}

/** Draws one layer's own contours, with or without their editing overlays. */
export function drawOutline(
  ctx: CanvasRenderingContext2D,
  viewport: Viewport,
  data: OutlineData,
  theme: GlyphTheme,
): void {
  if (data.contours.length === 0) return
  const map: PointMap = (point) => viewport.toScreen(point)

  ctx.save()
  ctx.beginPath()
  traceContours(ctx, data.contours, map)
  if (data.filled) {
    ctx.fillStyle = theme.fill
    ctx.fill('nonzero')
  } else {
    ctx.strokeStyle = theme.outline
    ctx.lineWidth = 1
    ctx.stroke()
  }

  if (data.showPoints) {
    for (const contour of data.contours) {
      const resolved = resolveContour(contour)
      drawHandles(ctx, resolved, map, theme)
      drawNodes(ctx, resolved, map, theme)
      drawStartMarker(ctx, resolved, map, theme)
    }
  }
  ctx.restore()
}
