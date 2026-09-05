/**
 * Outline drawing: contour paths, on-curve and off-curve points, handles,
 * start markers, open-path ends and the selection overlay.
 *
 * Contours arrive as UFO-style point lists, which express cubic and quadratic
 * segments in the same structure — including TrueType contours whose on-curve
 * points are implied, and contours that begin on an off-curve point.
 * `resolveContour` turns one such list into an explicit sequence of segments
 * whose end points are always on-curve. The path, nodes, handles and start
 * marker are all derived from that sequence, so the overlays cannot disagree
 * with the outline they annotate.
 *
 * Node glyph sizes are the design spec's (§4.1) and are constant in CSS
 * pixels: a node is equally clickable at 20 % and at 2000 % zoom.
 */

import type { Contour, Point } from '../../api'
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

/** Node sizes in CSS pixels, design spec §4.1. */
const CORNER_SIZE = 5
const SMOOTH_RADIUS = 2.5
const IMPLIED_RADIUS = 2.5
const OFF_CURVE_RADIUS = 1.75
const START_SIZE = 7
const OPEN_END_TICK = 6
const DUPLICATE_RADIUS = 3
const SELECTED_SEGMENT_WIDTH = 2
/** A node sitting on a vertical metric wears a 7 px diamond (§4.1 item 9). */
const METRIC_DIAMOND = 7
/** How near a vertical metric a node counts as sitting on it, in font units. */
const METRIC_TOLERANCE = 0.5

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

/** The 1 px inner ring that marks a node as selected (design spec §4.1). */
function drawSelectedRing(
  ctx: CanvasRenderingContext2D,
  centre: Point2,
  size: number,
  round: boolean,
  theme: GlyphTheme,
): void {
  ctx.strokeStyle = theme.selectionInk
  ctx.lineWidth = 1
  if (round) {
    ctx.beginPath()
    ctx.arc(centre.x, centre.y, Math.max(0.5, size / 2 - 1), 0, Math.PI * 2)
    ctx.stroke()
  } else {
    const inner = Math.max(1, size - 2)
    ctx.strokeRect(centre.x - inner / 2, centre.y - inner / 2, inner, inner)
  }
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

/**
 * Draws the start marker: a 7 px triangle pointing along the path, which says
 * both where the contour starts and which way it runs (design spec §4.1).
 */
function drawStartMarker(
  ctx: CanvasRenderingContext2D,
  resolved: ResolvedContour,
  map: PointMap,
  theme: GlyphTheme,
): void {
  if (!resolved.start) return
  const start = map(resolved.start)
  const target = directionTarget(resolved)
  const to = target ? map(target) : null
  const dx = to ? to.x - start.x : 1
  const dy = to ? to.y - start.y : 0
  const length = Math.hypot(dx, dy) || 1
  const ux = dx / length
  const uy = dy / length
  const half = START_SIZE / 2
  // Centre the triangle's centroid on the node so the marker reads as sitting
  // *on* the start point rather than trailing behind it.
  const cx = start.x + (ux * half) / 3
  const cy = start.y + (uy * half) / 3

  ctx.fillStyle = theme.start
  ctx.beginPath()
  ctx.moveTo(cx + ux * half, cy + uy * half)
  ctx.lineTo(cx - ux * half - uy * half, cy - uy * half + ux * half)
  ctx.lineTo(cx - ux * half + uy * half, cy - uy * half - ux * half)
  ctx.closePath()
  ctx.fill()
}

/** Perpendicular ticks marking the two ends of an open path. */
function drawOpenEnds(
  ctx: CanvasRenderingContext2D,
  resolved: ResolvedContour,
  map: PointMap,
  theme: GlyphTheme,
): void {
  if (resolved.closed) return
  const nodes = onCurvePoints(resolved)
  const first = nodes[0]
  const last = nodes[nodes.length - 1]
  if (!first || !last) return
  const target = directionTarget(resolved) ?? last
  const from = map(first)
  const towards = map(target)
  const dx = towards.x - from.x
  const dy = towards.y - from.y
  const length = Math.hypot(dx, dy) || 1
  const nx = -dy / length
  const ny = dx / length
  const half = OPEN_END_TICK / 2

  ctx.strokeStyle = theme.nodeCorner
  ctx.lineWidth = 1
  ctx.beginPath()
  for (const node of [first, last]) {
    const at = map(node)
    ctx.moveTo(at.x - nx * half, at.y - ny * half)
    ctx.lineTo(at.x + nx * half, at.y + ny * half)
  }
  ctx.stroke()
}

/**
 * Positions occupied by more than one point.
 *
 * Cached per contour list: it only changes when the outline does, and a drag
 * repaints sixty times a second over the same one.
 */
const duplicateCache = new WeakMap<object, Point2[]>()

function duplicatePositions(contours: readonly Contour[]): Point2[] {
  const cached = duplicateCache.get(contours)
  if (cached) return cached
  const seen = new Map<string, { at: Point2; count: number }>()
  for (const contour of contours)
    for (const point of contour.points) {
      const key = `${point.x}:${point.y}`
      const entry = seen.get(key)
      if (entry) entry.count += 1
      else seen.set(key, { at: { x: point.x, y: point.y }, count: 1 })
    }
  const found = [...seen.values()]
    .filter((entry) => entry.count > 1)
    .map((entry) => entry.at)
  duplicateCache.set(contours, found)
  return found
}

/** Red rings around nodes that sit on top of another node. */
function drawDuplicates(
  ctx: CanvasRenderingContext2D,
  contours: readonly Contour[],
  map: PointMap,
  theme: GlyphTheme,
): void {
  const duplicates = duplicatePositions(contours)
  if (duplicates.length === 0) return
  ctx.strokeStyle = theme.warning
  ctx.lineWidth = 1
  for (const position of duplicates) {
    const at = map(position)
    ctx.beginPath()
    ctx.arc(at.x, at.y, DUPLICATE_RADIUS, 0, Math.PI * 2)
    ctx.stroke()
  }
}

/** Whether a node sits on one of the layer's vertical metrics. */
function onVerticalMetric(y: number, metrics: readonly number[]): boolean {
  return metrics.some((metric) => Math.abs(y - metric) <= METRIC_TOLERANCE)
}

/** Draws the on-curve and off-curve nodes of one resolved contour. */
function drawNodes(
  ctx: CanvasRenderingContext2D,
  contourIndex: number,
  contour: Contour,
  resolved: ResolvedContour,
  map: PointMap,
  theme: GlyphTheme,
  selected: ReadonlySet<string>,
  verticalMetrics: readonly number[],
): void {
  const storedIndex = new Map<Point, number>()
  contour.points.forEach((point, index) => storedIndex.set(point, index))
  const keyOf = (
    source: Point | null,
    kind: 'node' | 'handle',
  ): string | null => {
    if (!source) return null
    const index = storedIndex.get(source)
    return index === undefined ? null : `${kind}:${contourIndex}:${index}`
  }

  for (const control of controlPoints(resolved)) {
    const at = map(control)
    const key = keyOf(control, 'handle')
    const isSelected = key !== null && selected.has(key)
    ctx.fillStyle = isSelected ? theme.selection : theme.nodeOffCurve
    drawCircle(ctx, at, OFF_CURVE_RADIUS)
    if (isSelected)
      drawSelectedRing(ctx, at, OFF_CURVE_RADIUS * 2 + 2, true, theme)
  }

  for (const node of onCurvePoints(resolved)) {
    const at = map(node)
    const key = keyOf(node.source, 'node')
    const isSelected = key !== null && selected.has(key)
    if (node.source === null) {
      // An on-curve point the font leaves implied gets its own hollow marker,
      // so a TrueType outline still reads as the point list it really is.
      ctx.strokeStyle = theme.nodeImplied
      ctx.lineWidth = 1
      drawDiamond(ctx, at, IMPLIED_RADIUS)
    } else if (node.source.smooth) {
      ctx.fillStyle = isSelected ? theme.selection : theme.nodeSmooth
      drawCircle(ctx, at, SMOOTH_RADIUS)
      if (isSelected) drawSelectedRing(ctx, at, SMOOTH_RADIUS * 2, true, theme)
    } else {
      ctx.fillStyle = isSelected ? theme.selection : theme.nodeCorner
      drawSquare(ctx, at, CORNER_SIZE)
      if (isSelected) drawSelectedRing(ctx, at, CORNER_SIZE, false, theme)
    }
    // A node exactly on the baseline, the x-height, the cap height or an
    // ascender/descender is worth seeing at a glance: it is what keeps an
    // outline aligned, and what a nudge silently breaks.
    if (onVerticalMetric(node.y, verticalMetrics)) {
      ctx.strokeStyle = theme.zone
      ctx.lineWidth = 1
      drawDiamond(ctx, at, METRIC_DIAMOND / 2)
    }
  }
}

/** Strokes the selected segments 2 px wide in the selection colour. */
function drawSelectedSegments(
  ctx: CanvasRenderingContext2D,
  contourIndex: number,
  resolved: ResolvedContour,
  map: PointMap,
  theme: GlyphTheme,
  selected: ReadonlySet<string>,
): void {
  let from = resolved.start
  ctx.strokeStyle = theme.selection
  ctx.lineWidth = SELECTED_SEGMENT_WIDTH
  resolved.segments.forEach((segment, index) => {
    const start = from
    from = segment.to
    if (!start || !selected.has(`segment:${contourIndex}:${index}`)) return
    const a = map(start)
    const to = map(segment.to)
    ctx.beginPath()
    ctx.moveTo(a.x, a.y)
    if (segment.kind === 'line') ctx.lineTo(to.x, to.y)
    else if (segment.kind === 'quad') {
      const control = map(segment.control)
      ctx.quadraticCurveTo(control.x, control.y, to.x, to.y)
    } else {
      const c1 = map(segment.c1)
      const c2 = map(segment.c2)
      ctx.bezierCurveTo(c1.x, c1.y, c2.x, c2.y, to.x, to.y)
    }
    ctx.stroke()
  })
}

/** What `drawOutline` renders. */
export interface OutlineData {
  contours: readonly Contour[]
  /** Fill the outline instead of stroking it (the space-bar preview). */
  filled: boolean
  /** Draw nodes, handles, start markers and direction arrows. */
  showPoints: boolean
  /** Selection keys (`node:c:p`, `handle:c:p`, `segment:c:s`). */
  selected?: ReadonlySet<string>
  /** Font-space y of every vertical metric, for the diamond marker. */
  verticalMetrics?: readonly number[]
}

const NOTHING_SELECTED: ReadonlySet<string> = new Set()

/** Draws one layer's own contours, with or without their editing overlays. */
export function drawOutline(
  ctx: CanvasRenderingContext2D,
  viewport: Viewport,
  data: OutlineData,
  theme: GlyphTheme,
): void {
  if (data.contours.length === 0) return
  const map: PointMap = (point) => viewport.toScreen(point)
  const selected = data.selected ?? NOTHING_SELECTED

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
    data.contours.forEach((contour, index) => {
      const resolved = resolveContour(contour)
      if (selected.size > 0)
        drawSelectedSegments(ctx, index, resolved, map, theme, selected)
      drawHandles(ctx, resolved, map, theme)
      drawNodes(
        ctx,
        index,
        contour,
        resolved,
        map,
        theme,
        selected,
        data.verticalMetrics ?? [],
      )
      drawOpenEnds(ctx, resolved, map, theme)
      drawStartMarker(ctx, resolved, map, theme)
    })
    drawDuplicates(ctx, data.contours, map, theme)
  }
  ctx.restore()
}
