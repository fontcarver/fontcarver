/**
 * Outline drawing: contour paths, on-curve and off-curve points, handles,
 * start markers and path-direction arrows.
 *
 * Contours arrive as UFO-style point lists, which express cubic and quadratic
 * segments in the same structure, including TrueType contours whose on-curve
 * points are implied. `traceContour` is the one place that resolves those
 * conventions into canvas path calls; everything else in the editor reuses it.
 */

import type { Contour, Point } from '../../api'
import type { Point2, Viewport } from '../../canvas/Viewport'

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
const OFF_CURVE_RADIUS = 2.6
const START_RADIUS = 6
const ARROW_LENGTH = 9

function isOnCurve(point: Point): boolean {
  return point.kind !== 'OffCurve'
}

function midpoint(a: Point2, b: Point2): Point2 {
  return { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 }
}

/**
 * Emits the segments that end at each on-curve point of `rest`, using the
 * off-curve points that precede it as control points.
 */
function emitSegments(
  sink: PathSink,
  rest: readonly Point[],
  map: PointMap,
): void {
  let pending: Point[] = []
  for (const point of rest) {
    if (!isOnCurve(point)) {
      pending.push(point)
      continue
    }
    const end = map(point)
    if (
      pending.length === 0 ||
      point.kind === 'Line' ||
      point.kind === 'Move'
    ) {
      sink.lineTo(end.x, end.y)
    } else if (point.kind === 'QCurve') {
      // A quadratic run: every pair of consecutive controls has an implied
      // on-curve point at its midpoint, which is what TrueType stores.
      for (let index = 0; index < pending.length - 1; index += 1) {
        const control = pending[index] as Point
        const next = pending[index + 1] as Point
        const implied = map(midpoint(control, next))
        const c = map(control)
        sink.quadraticCurveTo(c.x, c.y, implied.x, implied.y)
      }
      const last = map(pending[pending.length - 1] as Point)
      sink.quadraticCurveTo(last.x, last.y, end.x, end.y)
    } else if (pending.length >= 2) {
      const c1 = map(pending[pending.length - 2] as Point)
      const c2 = map(pending[pending.length - 1] as Point)
      sink.bezierCurveTo(c1.x, c1.y, c2.x, c2.y, end.x, end.y)
    } else {
      // A cubic segment with a single control point degenerates to a quadratic.
      const c = map(pending[0] as Point)
      sink.quadraticCurveTo(c.x, c.y, end.x, end.y)
    }
    pending = []
  }
}

/** Traces one contour into a path sink, mapping every point through `map`. */
export function traceContour(
  sink: PathSink,
  contour: Contour,
  map: PointMap,
): void {
  const points = contour.points
  if (points.length === 0) return

  if (!contour.closed) {
    const first = points[0] as Point
    const start = map(first)
    sink.moveTo(start.x, start.y)
    emitSegments(sink, points.slice(1), map)
    return
  }

  const startIndex = points.findIndex(isOnCurve)
  if (startIndex < 0) {
    // Every point is off-curve: a quadratic contour whose on-curve points are
    // all implied. It starts halfway between the last and the first control.
    const first = points[0] as Point
    const last = points[points.length - 1] as Point
    const start = map(midpoint(last, first))
    sink.moveTo(start.x, start.y)
    for (let index = 0; index < points.length; index += 1) {
      const control = points[index] as Point
      const next = points[(index + 1) % points.length] as Point
      const c = map(control)
      const end = map(midpoint(control, next))
      sink.quadraticCurveTo(c.x, c.y, end.x, end.y)
    }
    sink.closePath()
    return
  }

  const rotated = [...points.slice(startIndex), ...points.slice(0, startIndex)]
  const first = rotated[0] as Point
  const start = map(first)
  sink.moveTo(start.x, start.y)
  emitSegments(sink, [...rotated.slice(1), first], map)
  sink.closePath()
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

/** Draws the handle lines that join off-curve points to their on-curve node. */
function drawHandles(
  ctx: CanvasRenderingContext2D,
  contour: Contour,
  map: PointMap,
  theme: GlyphTheme,
): void {
  const points = contour.points
  ctx.strokeStyle = theme.handle
  ctx.lineWidth = 1
  ctx.beginPath()
  for (let index = 0; index < points.length; index += 1) {
    const point = points[index] as Point
    if (isOnCurve(point)) continue
    const from = map(point)
    const neighbours = [
      points[index - 1] ??
        (contour.closed ? points[points.length - 1] : undefined),
      points[index + 1] ?? (contour.closed ? points[0] : undefined),
    ]
    for (const neighbour of neighbours) {
      if (!neighbour || !isOnCurve(neighbour)) continue
      const to = map(neighbour)
      ctx.moveTo(from.x, from.y)
      ctx.lineTo(to.x, to.y)
    }
  }
  ctx.stroke()
}

/** Draws the start-point ring and the arrow showing the path direction. */
function drawStartMarker(
  ctx: CanvasRenderingContext2D,
  contour: Contour,
  map: PointMap,
  theme: GlyphTheme,
): void {
  const points = contour.points
  const first = points[0]
  if (!first) return
  const start = map(first)
  const next =
    points[1] ?? (contour.closed ? points[points.length - 1] : undefined)

  ctx.strokeStyle = theme.start
  ctx.lineWidth = 1.5
  ctx.beginPath()
  ctx.arc(start.x, start.y, START_RADIUS, 0, Math.PI * 2)
  ctx.stroke()

  if (!next) return
  const to = map(next)
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

/** Draws the on-curve and off-curve nodes of one contour. */
function drawNodes(
  ctx: CanvasRenderingContext2D,
  contour: Contour,
  map: PointMap,
  theme: GlyphTheme,
): void {
  for (const point of contour.points) {
    const at = map(point)
    if (!isOnCurve(point)) {
      ctx.fillStyle = theme.nodeOffCurve
      drawCircle(ctx, at, OFF_CURVE_RADIUS)
      continue
    }
    if (point.smooth) {
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
      drawHandles(ctx, contour, map, theme)
      drawNodes(ctx, contour, map, theme)
      drawStartMarker(ctx, contour, map, theme)
    }
  }
  ctx.restore()
}
