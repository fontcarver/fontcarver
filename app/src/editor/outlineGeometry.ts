/**
 * The editable index over a layer's contours.
 *
 * Drawing resolves a UFO point list into explicit segments; editing needs the
 * same segments *plus* the stored indexes behind them, because every command
 * addresses points as `PointRef { contour, point }`. Rather than resolve the
 * point list twice, this module reuses `resolveContour` and maps the resolved
 * geometry back onto stored indexes by object identity, which is exact: every
 * point in a resolved contour is one of the objects in `contour.points`.
 *
 * Geometric segment indexes match the kernel's (`fw_geom::segment_to_points`):
 * they start at the first stored on-curve point and a quadratic run of N
 * controls yields N segments.
 */

import type { Contour, Point, PointRef } from '../api'
import type { FontBox, Point2 } from '../canvas/Viewport'
import { resolveContour } from '../views/glyph/contourGeometry'

/** One stored point with the identity an editing command needs. */
export interface EditPoint {
  ref: PointRef
  x: number
  y: number
  /** Whether the point is on-curve; off-curve points are handles. */
  onCurve: boolean
  smooth: boolean
}

/** One geometric segment with its stored endpoints and controls. */
export interface EditSegment {
  contour: number
  /** Index within the contour, matching the kernel's segment numbering. */
  index: number
  kind: 'line' | 'quad' | 'cubic'
  from: Point2
  to: Point2
  /** Off-curve controls in path order; empty for a line. */
  controls: Point2[]
  /** Stored index of the start point, or `null` when it is implied. */
  fromRef: PointRef | null
  /** Stored index of the end point, or `null` when it is implied. */
  toRef: PointRef | null
  /** Stored indexes of the controls, in path order. */
  controlRefs: PointRef[]
}

/** A layer's contours with every editable feature indexed. */
export interface EditOutline {
  contours: readonly Contour[]
  points: EditPoint[]
  segments: EditSegment[]
}

function pointIndexes(contour: Contour): Map<Point, number> {
  const map = new Map<Point, number>()
  contour.points.forEach((point, index) => map.set(point, index))
  return map
}

/** Builds the editable index of one layer's contours. */
export function buildEditOutline(
  contours: readonly Contour[],
): EditOutline {
  const points: EditPoint[] = []
  const segments: EditSegment[] = []

  contours.forEach((contour, contourIndex) => {
    contour.points.forEach((point, index) => {
      points.push({
        ref: { contour: contourIndex, point: index },
        x: point.x,
        y: point.y,
        onCurve: point.kind !== 'OffCurve',
        smooth: point.smooth,
      })
    })

    const stored = pointIndexes(contour)
    const refOf = (point: Point | null): PointRef | null => {
      if (!point) return null
      const index = stored.get(point)
      return index === undefined
        ? null
        : { contour: contourIndex, point: index }
    }

    const resolved = resolveContour(contour)
    let from = resolved.start
    resolved.segments.forEach((segment, index) => {
      if (!from) return
      const controls: Point2[] =
        segment.kind === 'quad'
          ? [segment.control]
          : segment.kind === 'cubic'
            ? [segment.c1, segment.c2]
            : []
      const controlRefs = (
        segment.kind === 'quad'
          ? [refOf(segment.control)]
          : segment.kind === 'cubic'
            ? [refOf(segment.c1), refOf(segment.c2)]
            : []
      ).filter((ref): ref is PointRef => ref !== null)
      segments.push({
        contour: contourIndex,
        index,
        kind: segment.kind,
        from: { x: from.x, y: from.y },
        to: { x: segment.to.x, y: segment.to.y },
        controls,
        fromRef: refOf(from.source),
        toRef: refOf(segment.to.source),
        controlRefs,
      })
      from = segment.to
    })
  })

  return { contours, points, segments }
}

/** The point of a segment at parameter `t`. */
export function segmentPoint(segment: EditSegment, t: number): Point2 {
  const { from, to } = segment
  if (segment.kind === 'line')
    return { x: from.x + (to.x - from.x) * t, y: from.y + (to.y - from.y) * t }
  const u = 1 - t
  if (segment.kind === 'quad') {
    const c = segment.controls[0] ?? from
    return {
      x: u * u * from.x + 2 * u * t * c.x + t * t * to.x,
      y: u * u * from.y + 2 * u * t * c.y + t * t * to.y,
    }
  }
  const c1 = segment.controls[0] ?? from
  const c2 = segment.controls[1] ?? to
  return {
    x:
      u * u * u * from.x +
      3 * u * u * t * c1.x +
      3 * u * t * t * c2.x +
      t * t * t * to.x,
    y:
      u * u * u * from.y +
      3 * u * u * t * c1.y +
      3 * u * t * t * c2.y +
      t * t * t * to.y,
  }
}

/** How finely a segment is sampled before the nearest point is refined. */
const SAMPLES = 24

/** The point of `segment` nearest to `target`, with its parameter. */
export function nearestOnSegment(
  segment: EditSegment,
  target: Point2,
): { t: number; point: Point2; distance: number } {
  let best = { t: 0, point: segment.from, distance: Infinity }
  const consider = (t: number) => {
    const point = segmentPoint(segment, t)
    const distance = Math.hypot(point.x - target.x, point.y - target.y)
    if (distance < best.distance) best = { t, point, distance }
  }
  for (let index = 0; index <= SAMPLES; index += 1) consider(index / SAMPLES)
  // Two bisection passes around the best sample are enough for a hit test:
  // the residual error is well under a tenth of a font unit at any zoom.
  let span = 1 / SAMPLES
  for (let pass = 0; pass < 12; pass += 1) {
    span /= 2
    consider(Math.max(0, best.t - span))
    consider(Math.min(1, best.t + span))
  }
  return best
}

/** Bounding box of the listed font-space points, or `null` when empty. */
export function boundsOf(points: readonly Point2[]): FontBox | null {
  if (points.length === 0) return null
  let box = {
    x0: Infinity,
    y0: Infinity,
    x1: -Infinity,
    y1: -Infinity,
  }
  for (const point of points) {
    box = {
      x0: Math.min(box.x0, point.x),
      y0: Math.min(box.y0, point.y),
      x1: Math.max(box.x1, point.x),
      y1: Math.max(box.y1, point.y),
    }
  }
  return box
}

/** The stored points of a contour adjacent to `index`, wrapping when closed. */
export function adjacentIndexes(contour: Contour, index: number): number[] {
  const count = contour.points.length
  if (count < 2) return []
  const previous = index === 0 ? (contour.closed ? count - 1 : -1) : index - 1
  const next = index === count - 1 ? (contour.closed ? 0 : -1) : index + 1
  return [previous, next].filter((value) => value >= 0)
}

/**
 * The off-curve handles a node carries: the adjacent stored points that are
 * off-curve. `MovePoints { handles: "WithHandles" }` moves exactly these.
 */
export function handlesOfNode(
  contours: readonly Contour[],
  ref: PointRef,
): PointRef[] {
  const contour = contours[ref.contour]
  if (!contour) return []
  return adjacentIndexes(contour, ref.point)
    .filter((index) => contour.points[index]?.kind === 'OffCurve')
    .map((index) => ({ contour: ref.contour, point: index }))
}

/**
 * The on-curve node a handle belongs to: the adjacent stored point that is
 * on-curve. A handle between two off-curve points (a quadratic run) has none.
 */
export function nodeOfHandle(
  contours: readonly Contour[],
  ref: PointRef,
): PointRef | null {
  const contour = contours[ref.contour]
  if (!contour) return null
  const index = adjacentIndexes(contour, ref.point).find(
    (candidate) => contour.points[candidate]?.kind !== 'OffCurve',
  )
  return index === undefined ? null : { contour: ref.contour, point: index }
}

/**
 * The handle opposite `handle` across its node, used by the `Ctrl+Alt` mirror
 * on a smooth node.
 */
export function oppositeHandle(
  contours: readonly Contour[],
  handle: PointRef,
): PointRef | null {
  const node = nodeOfHandle(contours, handle)
  if (!node) return null
  const contour = contours[node.contour]
  if (!contour) return null
  const index = adjacentIndexes(contour, node.point).find(
    (candidate) =>
      candidate !== handle.point && contour.points[candidate]?.kind === 'OffCurve',
  )
  return index === undefined ? null : { contour: node.contour, point: index }
}

/** Applies a translation to the listed stored points, returning new contours. */
export function translatePoints(
  contours: readonly Contour[],
  refs: readonly PointRef[],
  dx: number,
  dy: number,
): Contour[] {
  if (refs.length === 0 || (dx === 0 && dy === 0))
    return contours.map((contour) => contour)
  const moved = new Set(refs.map((ref) => `${ref.contour}:${ref.point}`))
  return contours.map((contour, contourIndex) => {
    if (!refs.some((ref) => ref.contour === contourIndex)) return contour
    return {
      ...contour,
      points: contour.points.map((point, index) =>
        moved.has(`${contourIndex}:${index}`)
          ? { ...point, x: point.x + dx, y: point.y + dy }
          : point,
      ),
    }
  })
}

/** Sets one stored point's absolute position, returning new contours. */
export function setPointPosition(
  contours: readonly Contour[],
  ref: PointRef,
  x: number,
  y: number,
): Contour[] {
  return contours.map((contour, contourIndex) =>
    contourIndex !== ref.contour
      ? contour
      : {
          ...contour,
          points: contour.points.map((point, index) =>
            index === ref.point ? { ...point, x, y } : point,
          ),
        },
  )
}

/** Reads one stored point, or `null` when the index does not exist. */
export function pointAt(
  contours: readonly Contour[],
  ref: PointRef,
): Point | null {
  return contours[ref.contour]?.points[ref.point] ?? null
}
