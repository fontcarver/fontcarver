/** Bounded geometric helpers for linked construction editing. */
import type { Contour, PointRef } from '../../../api'
import type { Point2 } from '../../../canvas/Viewport'
import { buildEditOutline, segmentPoint } from '../../../editor/outlineGeometry'

/** An axis-aligned straight edge with stable command-boundary node indexes. */
export interface StemEdge {
  position: number
  points: PointRef[]
  from: number
  to: number
}

/** Two outline edges bounding ink, as opposed to a counter. */
export interface DetectedStem {
  axis: 'X' | 'Y'
  width: number
  low: StemEdge
  high: StemEdge
}

const EPSILON = 1e-7
const MAX_SEGMENTS = 2048
const MAX_EDGES = 512
const MAX_STEMS = 128

/** Finds the adjacent non-servant on-curve nodes, in contour order. */
export function servantNeighbors(
  contours: readonly Contour[],
  node: PointRef,
  servants: ReadonlySet<string>,
): [PointRef, PointRef] | null {
  const contour = contours[node.contour]
  if (
    !contour ||
    !contour.points[node.point] ||
    contour.points[node.point]?.kind === 'OffCurve'
  )
    return null
  const find = (direction: number): PointRef | null => {
    for (let step = 1; step < contour.points.length; step += 1) {
      const raw = node.point + direction * step
      if (!contour.closed && (raw < 0 || raw >= contour.points.length))
        return null
      const point = (raw + contour.points.length) % contour.points.length
      if (
        contour.points[point]?.kind !== 'OffCurve' &&
        !servants.has(`${node.contour}:${point}`)
      )
        return { contour: node.contour, point }
    }
    return null
  }
  const first = find(-1)
  const second = find(1)
  return first && second && first.point !== second.point
    ? [first, second]
    : null
}

/**
 * Detects straight horizontal/vertical stems. Curved boundaries participate in
 * fill checks using 32 samples per segment; candidate edges themselves are exact
 * straight segments. Very complex outlines return no automatic suggestions.
 */
export function detectStraightStems(
  contours: readonly Contour[],
  maxWidth: number,
): DetectedStem[] {
  if (!Number.isFinite(maxWidth) || maxWidth <= 0) return []
  const outline = buildEditOutline(contours)
  if (outline.segments.length > MAX_SEGMENTS) return []
  const lines: [Point2, Point2][] = []
  const candidates: Record<'X' | 'Y', StemEdge[]> = { X: [], Y: [] }
  for (const segment of outline.segments) {
    if (!contours[segment.contour]?.closed) continue
    if (
      ![segment.from, segment.to, ...segment.controls].every(
        (point) => Number.isFinite(point.x) && Number.isFinite(point.y),
      )
    )
      return []
    const steps = segment.kind === 'line' ? 1 : 32
    let previous = segment.from
    for (let step = 1; step <= steps; step += 1) {
      const point = segmentPoint(segment, step / steps)
      lines.push([previous, point])
      previous = point
    }
    if (segment.kind !== 'line' || !segment.fromRef || !segment.toRef) continue
    for (const axis of ['X', 'Y'] as const) {
      const across = axis === 'X' ? 'x' : 'y'
      const along = axis === 'X' ? 'y' : 'x'
      if (
        Math.abs(segment.from[across] - segment.to[across]) > EPSILON ||
        Math.abs(segment.from[along] - segment.to[along]) <= EPSILON
      )
        continue
      candidates[axis].push({
        position: segment.from[across],
        points: [segment.fromRef, segment.toRef],
        from: Math.min(segment.from[along], segment.to[along]),
        to: Math.max(segment.from[along], segment.to[along]),
      })
    }
  }
  if (candidates.X.length + candidates.Y.length > MAX_EDGES) return []
  const stems: DetectedStem[] = []
  for (const axis of ['X', 'Y'] as const) {
    const edges = candidates[axis].sort((a, b) => a.position - b.position)
    for (let i = 0; i < edges.length; i += 1) {
      const low = edges[i]!
      for (let j = i + 1; j < edges.length; j += 1) {
        const high = edges[j]!
        const width = high.position - low.position
        if (width > maxWidth) break
        const from = Math.max(low.from, high.from)
        const to = Math.min(low.to, high.to)
        if (width <= EPSILON || to - from < Math.max(1, width)) continue
        if (
          ![0.25, 0.5, 0.75].every((fraction) =>
            inkSpan(
              lines,
              axis,
              low.position,
              high.position,
              from + (to - from) * fraction,
            ),
          )
        )
          continue
        const existing = stems.find(
          (stem) =>
            stem.axis === axis &&
            Math.abs(stem.low.position - low.position) <= EPSILON &&
            Math.abs(stem.high.position - high.position) <= EPSILON,
        )
        if (existing) {
          mergePoints(existing.low.points, low.points)
          mergePoints(existing.high.points, high.points)
        } else {
          stems.push({
            axis,
            width,
            low: { ...low, points: [...low.points] },
            high: { ...high, points: [...high.points] },
          })
          if (stems.length >= MAX_STEMS) return stems
        }
      }
    }
  }
  return stems
}

function mergePoints(target: PointRef[], added: readonly PointRef[]): void {
  for (const ref of added)
    if (
      !target.some(
        (point) => point.contour === ref.contour && point.point === ref.point,
      )
    )
      target.push(ref)
}

function inkSpan(
  lines: readonly [Point2, Point2][],
  axis: 'X' | 'Y',
  low: number,
  high: number,
  along: number,
): boolean {
  const acrossKey = axis === 'X' ? 'x' : 'y'
  const alongKey = axis === 'X' ? 'y' : 'x'
  const middle = (low + high) / 2
  let winding = 0
  for (const [a, b] of lines) {
    if (
      (a[alongKey] <= along && b[alongKey] > along) ||
      (b[alongKey] <= along && a[alongKey] > along)
    ) {
      const t = (along - a[alongKey]) / (b[alongKey] - a[alongKey])
      const cross = a[acrossKey] + (b[acrossKey] - a[acrossKey]) * t
      if (cross > low + EPSILON && cross < high - EPSILON) return false
      if (cross > middle) winding += b[alongKey] > a[alongKey] ? 1 : -1
    }
  }
  return winding !== 0
}
