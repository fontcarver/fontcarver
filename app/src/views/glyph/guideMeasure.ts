/**
 * Where a guide crosses the outline, and how far apart those crossings are
 * (design spec §4.1 item 12: "measurement guides show intersection
 * distances").
 *
 * A measurement guide answers "how wide is this stem here?", so what matters
 * is the distance between successive crossings *along the guide*. Each segment
 * is flattened into a short polyline and intersected with the guide's line
 * analytically; a font unit or two of flattening error is invisible next to a
 * stem width, and it keeps one routine covering lines, quadratics and cubics.
 */

import type { Contour } from '../../api'
import type { Point2 } from '../../canvas/Viewport'

import { resolveContour, type ResolvedSegment } from './contourGeometry'

/** Polyline steps per curved segment; 24 keeps the error well under a unit. */
const FLATTEN_STEPS = 24

/** A crossing of the guide, given as its distance along the guide's direction. */
export interface Crossing {
  /** Signed distance from the guide's origin, in font units. */
  t: number
  point: Point2
}

/** The gap between two successive crossings. */
export interface Measurement {
  from: Point2
  to: Point2
  /** Distance in font units. */
  length: number
}

function pointOn(from: Point2, segment: ResolvedSegment, t: number): Point2 {
  if (segment.kind === 'line')
    return {
      x: from.x + (segment.to.x - from.x) * t,
      y: from.y + (segment.to.y - from.y) * t,
    }
  const u = 1 - t
  if (segment.kind === 'quad')
    return {
      x: u * u * from.x + 2 * u * t * segment.control.x + t * t * segment.to.x,
      y: u * u * from.y + 2 * u * t * segment.control.y + t * t * segment.to.y,
    }
  return {
    x:
      u * u * u * from.x +
      3 * u * u * t * segment.c1.x +
      3 * u * t * t * segment.c2.x +
      t * t * t * segment.to.x,
    y:
      u * u * u * from.y +
      3 * u * u * t * segment.c1.y +
      3 * u * t * t * segment.c2.y +
      t * t * t * segment.to.y,
  }
}

/** Flattens one contour into the polyline the intersection walks. */
function polyline(contour: Contour): Point2[] {
  const resolved = resolveContour(contour)
  if (!resolved.start) return []
  const points: Point2[] = [resolved.start]
  let from: Point2 = resolved.start
  for (const segment of resolved.segments) {
    if (segment.kind === 'line') {
      points.push(segment.to)
    } else {
      for (let step = 1; step <= FLATTEN_STEPS; step += 1)
        points.push(pointOn(from, segment, step / FLATTEN_STEPS))
    }
    from = segment.to
  }
  if (resolved.closed && points.length > 1) points.push(resolved.start)
  return points
}

/** A guide as a point and a direction. */
export interface GuideLine {
  x: number
  y: number
  /** Counter-clockwise angle in degrees. */
  angle: number
}

/**
 * Every crossing of the guide with the outline, ordered along the guide.
 *
 * A segment that merely touches the line — an extremum sitting exactly on it —
 * produces one crossing, not two, because the polyline's shared endpoint is
 * only counted for the segment it starts.
 */
export function guideCrossings(
  contours: readonly Contour[],
  guide: GuideLine,
): Crossing[] {
  const radians = (guide.angle * Math.PI) / 180
  const dx = Math.cos(radians)
  const dy = Math.sin(radians)
  // The normal of the guide: a point's signed distance from the line.
  const side = (point: Point2): number =>
    (point.x - guide.x) * -dy + (point.y - guide.y) * dx
  const along = (point: Point2): number =>
    (point.x - guide.x) * dx + (point.y - guide.y) * dy

  const found: Crossing[] = []
  for (const contour of contours) {
    const points = polyline(contour)
    for (let index = 0; index + 1 < points.length; index += 1) {
      const a = points[index]
      const b = points[index + 1]
      if (!a || !b) continue
      const sa = side(a)
      const sb = side(b)
      if (sa === sb) continue
      // Half-open in `a`: a vertex exactly on the line belongs to the
      // segment that starts there, so it is not counted twice.
      if ((sa > 0 && sb > 0) || (sa < 0 && sb < 0)) continue
      if (sb === 0) continue
      const t = sa / (sa - sb)
      const point = { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t }
      found.push({ t: along(point), point })
    }
  }
  return found.sort((a, b) => a.t - b.t)
}

/** The distances between successive crossings, for the on-canvas labels. */
export function guideMeasurements(
  contours: readonly Contour[],
  guide: GuideLine,
): Measurement[] {
  const crossings = guideCrossings(contours, guide)
  const spans: Measurement[] = []
  for (let index = 0; index + 1 < crossings.length; index += 1) {
    const from = crossings[index]
    const to = crossings[index + 1]
    if (!from || !to) continue
    const length = to.t - from.t
    if (length <= 0) continue
    spans.push({ from: from.point, to: to.point, length })
  }
  return spans
}
