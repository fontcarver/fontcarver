/**
 * The three delta regimes a drag can be in.
 *
 * Most gestures move everything they carry by one delta. Two do not: ⌥ on a
 * segment keeps every control on its own handle line, and Ctrl+⌥ on a node
 * rescales the neighbouring nodes' handles so each segment keeps its share of
 * a chord the move just stretched (design spec §4.3). Both express themselves
 * as one delta per stored point, which is what this module computes.
 */

import type { Contour, PointRef } from '../../../api'
import type { Point2 } from '../../../canvas/Viewport'
import { buildEditOutline, nodeOfHandle, pointAt } from '../../outlineGeometry'

import { refKey } from './plan'

/** Constrains a delta to the nearest 0/45/90° direction. */
export function constrainDelta(delta: Point2): Point2 {
  const angle = Math.atan2(delta.y, delta.x)
  const step = Math.PI / 4
  const snapped = Math.round(angle / step) * step
  const length = Math.hypot(delta.x, delta.y)
  const x = Math.cos(snapped) * length
  const y = Math.sin(snapped) * length
  // Zero the axis that the constraint collapsed, so an axis-locked drag is
  // exactly axis-locked rather than a rounding error away from it.
  return {
    x: Math.abs(x) < 1e-9 ? 0 : x,
    y: Math.abs(y) < 1e-9 ? 0 : y,
  }
}

/** Projects a delta onto the direction from `from` to `to`. */
export function projectOnto(delta: Point2, from: Point2, to: Point2): Point2 {
  const dx = to.x - from.x
  const dy = to.y - from.y
  const length = Math.hypot(dx, dy)
  if (length < 1e-9) return delta
  const unit = { x: dx / length, y: dy / length }
  const amount = delta.x * unit.x + delta.y * unit.y
  return { x: unit.x * amount, y: unit.y * amount }
}

/** The deltas that keep every control of a segment on its own handle line. */
export function perHandleDeltas(
  contours: readonly Contour[],
  refs: readonly PointRef[],
  delta: Point2,
): Map<string, Point2> | null {
  const deltas = new Map<string, Point2>()
  for (const ref of refs) {
    const handle = pointAt(contours, ref)
    const node = nodeOfHandle(contours, ref)
    const nodePoint = node ? pointAt(contours, node) : null
    if (!handle || !nodePoint) return null
    deltas.set(refKey(ref), projectOnto(delta, nodePoint, handle))
  }
  return deltas.size > 0 ? deltas : null
}

/** A neighbouring node's handle that a proportional nudge rescales. */
export interface ProportionalHandle {
  ref: PointRef
  /** The handle's current position. */
  handle: Point2
  /** The node the handle belongs to, which stays put. */
  anchor: Point2
  /** The node being dragged, whose move changes the chord. */
  node: Point2
}

/**
 * A proportional nudge: everything the gesture carries moves by `delta`, and
 * each neighbouring node's handle keeps its share of the segment whose chord
 * the move just stretched or shortened.
 */
export function nudgeDeltas(
  movedRefs: readonly PointRef[],
  proportional: readonly ProportionalHandle[],
  delta: Point2,
): Map<string, Point2> {
  const deltas = new Map<string, Point2>()
  for (const ref of movedRefs) deltas.set(refKey(ref), delta)
  for (const entry of proportional) {
    const before = Math.hypot(
      entry.node.x - entry.anchor.x,
      entry.node.y - entry.anchor.y,
    )
    if (before < 1e-9) continue
    const after = Math.hypot(
      entry.node.x + delta.x - entry.anchor.x,
      entry.node.y + delta.y - entry.anchor.y,
    )
    const scale = after / before
    deltas.set(refKey(entry.ref), {
      x:
        entry.anchor.x +
        (entry.handle.x - entry.anchor.x) * scale -
        entry.handle.x,
      y:
        entry.anchor.y +
        (entry.handle.y - entry.anchor.y) * scale -
        entry.handle.y,
    })
  }
  return deltas
}

/**
 * The handles that a proportional nudge rescales.
 *
 * For every segment that ends at a dragged node, the control belonging to the
 * *other* end is collected — unless it is itself part of the gesture, in which
 * case it already travels with the selection and must not be scaled on top.
 */
export function proportionalHandles(
  contours: readonly Contour[],
  refs: readonly PointRef[],
  moving: ReadonlySet<string>,
): ProportionalHandle[] {
  const outline = buildEditOutline(contours)
  const dragged = new Set(refs.map(refKey))
  const found = new Map<string, ProportionalHandle>()
  for (const segment of outline.segments) {
    if (segment.controlRefs.length === 0) continue
    const ends: [PointRef | null, PointRef | null][] = [
      [segment.fromRef, segment.toRef],
      [segment.toRef, segment.fromRef],
    ]
    for (const [near, far] of ends) {
      if (!near || !far) continue
      if (!dragged.has(refKey(near))) continue
      if (dragged.has(refKey(far))) continue
      // The control adjacent to `far` is the one that belongs to it.
      const ref =
        far === segment.fromRef
          ? segment.controlRefs[0]
          : segment.controlRefs[segment.controlRefs.length - 1]
      if (!ref) continue
      const key = refKey(ref)
      if (moving.has(key) || found.has(key)) continue
      const handle = pointAt(contours, ref)
      const anchor = pointAt(contours, far)
      const node = pointAt(contours, near)
      if (!handle || !anchor || !node) continue
      found.set(key, {
        ref,
        handle: { x: handle.x, y: handle.y },
        anchor: { x: anchor.x, y: anchor.y },
        node: { x: node.x, y: node.y },
      })
    }
  }
  return [...found.values()]
}
