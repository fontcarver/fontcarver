/**
 * Resolves UFO-style contour point lists into explicit drawable segments.
 * Implied quadratic midpoints live here so path tracing and editing overlays
 * share exactly the same geometry.
 */

import type { Contour, Point } from '../../api'
import type { Point2 } from '../../canvas/Viewport'

/** An explicit or implied on-curve point. */
export interface ResolvedPoint extends Point2 {
  /** Source point, or `null` for a midpoint implied by quadratic controls. */
  source: Point | null
}

/** One resolved segment. Its end point is always on-curve. */
export type ResolvedSegment =
  | { kind: 'line'; to: ResolvedPoint }
  | { kind: 'quad'; control: Point; to: ResolvedPoint }
  | { kind: 'cubic'; c1: Point; c2: Point; to: ResolvedPoint }

/** A contour with every implied point made explicit. */
export interface ResolvedContour {
  closed: boolean
  /** The point the path starts at, or `null` for an empty contour. */
  start: ResolvedPoint | null
  segments: ResolvedSegment[]
}

function isOnCurve(point: Point): boolean {
  return point.kind !== 'OffCurve'
}

function onCurve(point: Point): ResolvedPoint {
  return { x: point.x, y: point.y, source: point }
}

function impliedBetween(a: Point2, b: Point2): ResolvedPoint {
  return { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2, source: null }
}

/** Builds segments ending at each on-curve point, consuming prior controls. */
function segmentsOf(rest: readonly Point[]): ResolvedSegment[] {
  const segments: ResolvedSegment[] = []
  let pending: Point[] = []
  for (const point of rest) {
    if (!isOnCurve(point)) {
      pending.push(point)
      continue
    }
    const to = onCurve(point)
    if (
      pending.length === 0 ||
      point.kind === 'Line' ||
      point.kind === 'Move'
    ) {
      segments.push({ kind: 'line', to })
    } else if (point.kind === 'QCurve') {
      for (let index = 0; index < pending.length - 1; index += 1) {
        const control = pending[index] as Point
        const next = pending[index + 1] as Point
        segments.push({
          kind: 'quad',
          control,
          to: impliedBetween(control, next),
        })
      }
      segments.push({
        kind: 'quad',
        control: pending[pending.length - 1] as Point,
        to,
      })
    } else if (pending.length >= 2) {
      segments.push({
        kind: 'cubic',
        c1: pending[pending.length - 2] as Point,
        c2: pending[pending.length - 1] as Point,
        to,
      })
    } else {
      segments.push({ kind: 'quad', control: pending[0] as Point, to })
    }
    pending = []
  }
  return segments
}

/**
 * Resolves one point-list contour into a start point plus explicit segments.
 * Closed contours start on the first explicit on-curve point, or at the
 * implied midpoint between the last and first controls when every point is
 * off-curve.
 */
export function resolveContour(contour: Contour): ResolvedContour {
  const points = contour.points
  if (points.length === 0)
    return { closed: contour.closed, start: null, segments: [] }

  if (!contour.closed) {
    const first = points[0] as Point
    return {
      closed: false,
      start: onCurve(first),
      segments: segmentsOf(points.slice(1)),
    }
  }

  const startIndex = points.findIndex(isOnCurve)
  if (startIndex < 0) {
    const first = points[0] as Point
    const last = points[points.length - 1] as Point
    const start = impliedBetween(last, first)
    const segments: ResolvedSegment[] = points.map((control, index) => ({
      kind: 'quad',
      control,
      to:
        index === points.length - 1
          ? start
          : impliedBetween(control, points[index + 1] as Point),
    }))
    return { closed: true, start, segments }
  }

  const rotated = [...points.slice(startIndex), ...points.slice(0, startIndex)]
  const first = rotated[0] as Point
  const start = onCurve(first)
  const segments = segmentsOf([...rotated.slice(1), first])
  const closing = segments[segments.length - 1]
  if (closing && closing.to.source === first) closing.to = start
  return { closed: true, start, segments }
}

/** The on-curve points in path order, without a repeated closing point. */
export function onCurvePoints(resolved: ResolvedContour): ResolvedPoint[] {
  const seen = new Set<ResolvedPoint>()
  const nodes: ResolvedPoint[] = []
  for (const node of [
    resolved.start,
    ...resolved.segments.map((segment) => segment.to),
  ]) {
    if (!node || seen.has(node)) continue
    seen.add(node)
    nodes.push(node)
  }
  return nodes
}

/** The off-curve control points in path order. */
export function controlPoints(resolved: ResolvedContour): Point[] {
  return resolved.segments.flatMap((segment) =>
    segment.kind === 'quad'
      ? [segment.control]
      : segment.kind === 'cubic'
        ? [segment.c1, segment.c2]
        : [],
  )
}

/** Joins every control to the effective on-curve neighbours it governs. */
export function handleLines(resolved: ResolvedContour): [Point2, Point2][] {
  const lines: [Point2, Point2][] = []
  let from: Point2 | null = resolved.start
  for (const segment of resolved.segments) {
    if (segment.kind === 'quad') {
      if (from) lines.push([segment.control, from])
      lines.push([segment.control, segment.to])
    } else if (segment.kind === 'cubic') {
      if (from) lines.push([segment.c1, from])
      lines.push([segment.c2, segment.to])
    }
    from = segment.to
  }
  return lines
}

/** The first segment target used to orient the path-direction arrow. */
export function directionTarget(resolved: ResolvedContour): Point2 | null {
  const first = resolved.segments[0]
  if (!first) return null
  if (first.kind === 'line') return first.to
  if (first.kind === 'quad') return first.control
  return first.c1
}
