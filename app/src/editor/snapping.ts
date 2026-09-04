/**
 * Snapping while dragging.
 *
 * Design spec §4.2: 6 CSS pixels, tried in the order node > handle > anchor >
 * guide > metric > alignment line, and suspended entirely while Ctrl is held.
 * Point targets snap both axes; guides, metric lines and alignment lines snap
 * only the axis they constrain, so dragging along a baseline stays free in x.
 *
 * The grid is not a snap target yet: the grid overlay itself is a preference
 * that ships with the drawing tools (P1 T7), and snapping to something that is
 * never drawn would be a mystery rather than a help.
 */

import type { Anchor } from '../api'
import type { Point2, Viewport } from '../canvas/Viewport'
import type { SceneMetrics } from '../views/glyph/glyphScene'

import type { ScopedGuide } from './hitTest'
import type { EditOutline } from './outlineGeometry'

/** Snap distance in CSS pixels, design spec §4.2. */
export const SNAP_DISTANCE = 6

/** What a snap latched onto. */
export interface SnapMatch {
  /** Adjusted delta. */
  delta: Point2
  /** Human name of the target, for the HUD; `null` when nothing snapped. */
  label: string | null
  /** Font-space x an alignment line was drawn at, when one matched. */
  alignedX: number | null
  /** Font-space y an alignment line was drawn at, when one matched. */
  alignedY: number | null
  /** The snapped point, for the halo. */
  point: Point2 | null
}

/** Everything snapping looks at. */
export interface SnapInput {
  outline: EditOutline
  anchors: readonly Anchor[]
  guides: readonly ScopedGuide[]
  metrics: SceneMetrics
  metricLabels: Readonly<
    Record<
      'baseline' | 'xHeight' | 'capHeight' | 'ascender' | 'descender',
      string
    >
  >
  viewport: Viewport
  /** `contour:point` keys excluded from snapping, i.e. what is being dragged. */
  exclude: ReadonlySet<string>
}

interface PointTarget {
  point: Point2
  label: string
  rank: number
}

/** The unsnapped result for a delta. */
function unsnapped(delta: Point2): SnapMatch {
  return {
    delta,
    label: null,
    alignedX: null,
    alignedY: null,
    point: null,
  }
}

function pointTargets(input: SnapInput): PointTarget[] {
  const targets: PointTarget[] = []
  for (const point of input.outline.points) {
    const key = `${point.ref.contour}:${point.ref.point}`
    if (input.exclude.has(key)) continue
    targets.push({
      point: { x: point.x, y: point.y },
      label: point.onCurve ? 'node' : 'handle',
      rank: point.onCurve ? 0 : 1,
    })
  }
  for (const anchor of input.anchors)
    targets.push({
      point: { x: anchor.x, y: anchor.y },
      label: anchor.name,
      rank: 2,
    })
  return targets
}

/** Horizontal metric lines, in font units, with their labels. */
function metricLines(input: SnapInput): { y: number; label: string }[] {
  const { metrics, metricLabels } = input
  return [
    { y: 0, label: metricLabels.baseline },
    { y: metrics.xHeight, label: metricLabels.xHeight },
    { y: metrics.capHeight, label: metricLabels.capHeight },
    { y: metrics.ascender, label: metricLabels.ascender },
    { y: metrics.descender, label: metricLabels.descender },
  ]
}

/**
 * Snaps a proposed delta.
 *
 * `moved` are the points the gesture is dragging, already offset by `delta`;
 * the first is the one under the pointer and the one snapping aims at.
 */
export function snapDelta(
  input: SnapInput,
  delta: Point2,
  moved: readonly Point2[],
): SnapMatch {
  const primary = moved[0]
  if (!primary) return unsnapped(delta)
  const scale = input.viewport.scale > 0 ? input.viewport.scale : 1
  const reach = SNAP_DISTANCE / scale

  let best: { target: PointTarget; distance: number } | null = null
  for (const target of pointTargets(input)) {
    const distance = Math.hypot(
      target.point.x - primary.x,
      target.point.y - primary.y,
    )
    if (distance > reach) continue
    if (
      !best ||
      target.rank < best.target.rank ||
      (target.rank === best.target.rank && distance < best.distance)
    )
      best = { target, distance }
  }
  if (best)
    return {
      delta: {
        x: delta.x + (best.target.point.x - primary.x),
        y: delta.y + (best.target.point.y - primary.y),
      },
      label: best.target.label,
      alignedX: null,
      alignedY: null,
      point: best.target.point,
    }

  // Axis-constrained targets: guides, then metric lines, then alignment lines.
  let snappedX: { value: number; label: string } | null = null
  let snappedY: { value: number; label: string } | null = null

  for (const entry of input.guides) {
    const { guide } = entry
    const angle = ((guide.angle % 180) + 180) % 180
    const name = guide.name ?? 'guide'
    if (angle === 0 && Math.abs(guide.y - primary.y) <= reach && !snappedY)
      snappedY = { value: guide.y, label: name }
    if (angle === 90 && Math.abs(guide.x - primary.x) <= reach && !snappedX)
      snappedX = { value: guide.x, label: name }
  }

  if (!snappedY)
    for (const line of metricLines(input))
      if (Math.abs(line.y - primary.y) <= reach) {
        snappedY = { value: line.y, label: line.label }
        break
      }

  let alignedX: number | null = null
  let alignedY: number | null = null
  if (!snappedX || !snappedY)
    for (const point of input.outline.points) {
      const key = `${point.ref.contour}:${point.ref.point}`
      if (input.exclude.has(key)) continue
      if (!snappedX && Math.abs(point.x - primary.x) <= reach) {
        snappedX = { value: point.x, label: 'align' }
        alignedX = point.x
      }
      if (!snappedY && Math.abs(point.y - primary.y) <= reach) {
        snappedY = { value: point.y, label: 'align' }
        alignedY = point.y
      }
      if (snappedX && snappedY) break
    }

  if (!snappedX && !snappedY) return unsnapped(delta)
  const next = {
    x: snappedX ? delta.x + (snappedX.value - primary.x) : delta.x,
    y: snappedY ? delta.y + (snappedY.value - primary.y) : delta.y,
  }
  const labels = [snappedY?.label, snappedX?.label].filter(
    (label): label is string => label !== undefined && label !== 'align',
  )
  return {
    delta: next,
    label: labels[0] ?? null,
    alignedX,
    alignedY,
    point: {
      x: snappedX ? snappedX.value : primary.x,
      y: snappedY ? snappedY.value : primary.y,
    },
  }
}
