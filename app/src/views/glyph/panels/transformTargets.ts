/**
 * What the Transform panel acts on.
 *
 * `TransformSelection` names points, components and anchors explicitly, so the
 * panel has to turn a canvas selection into those three lists — and, when
 * nothing is selected, into the whole layer, which is what design spec §7.7
 * asks for. Align and distribute need the same selection grouped differently:
 * per *object*, because distributing individual points is meaningless.
 */

import type { Anchor, ComponentView, Contour, PointRef } from '../../../api'
import type { FontBox } from '../../../canvas/Viewport'
import { boundsOf } from '../../../editor/outlineGeometry'
import {
  selectedIndexes,
  selectedPointRefs,
  type Selection,
} from '../../../editor/selection'

import type { AlignItem } from './transformMath'

/** A layer as the Transform panel reads it. */
export interface TransformLayer {
  contours: readonly Contour[]
  components: readonly ComponentView[]
  anchors: readonly Anchor[]
}

/** Everything one `TransformSelection` names, plus the box it spans. */
export interface TransformTargets {
  points: PointRef[]
  components: number[]
  anchors: number[]
  box: FontBox | null
  /** Whether the targets are the whole layer rather than a real selection. */
  wholeLayer: boolean
}

function everyPoint(contours: readonly Contour[]): PointRef[] {
  const refs: PointRef[] = []
  contours.forEach((contour, index) =>
    contour.points.forEach((_, point) => refs.push({ contour: index, point })),
  )
  return refs
}

function boxOf(
  layer: TransformLayer,
  targets: {
    points: readonly PointRef[]
    components: readonly number[]
    anchors: readonly number[]
  },
): FontBox | null {
  const positions: { x: number; y: number }[] = []
  for (const ref of targets.points) {
    const point = layer.contours[ref.contour]?.points[ref.point]
    if (point) positions.push({ x: point.x, y: point.y })
  }
  for (const index of targets.anchors) {
    const anchor = layer.anchors[index]
    if (anchor) positions.push({ x: anchor.x, y: anchor.y })
  }
  for (const index of targets.components) {
    const bbox = layer.components[index]?.bbox
    if (bbox)
      positions.push({ x: bbox[0], y: bbox[1] }, { x: bbox[2], y: bbox[3] })
  }
  return boundsOf(positions)
}

/** The points, components and anchors an Apply would move. */
export function transformTargets(
  layer: TransformLayer,
  selection: Selection,
): TransformTargets {
  const points = selectedPointRefs(selection)
  const components = selectedIndexes(selection, 'component')
  const anchors = selectedIndexes(selection, 'anchor')
  const wholeLayer =
    points.length === 0 && components.length === 0 && anchors.length === 0
  const targets = wholeLayer
    ? {
        points: everyPoint(layer.contours),
        components: layer.components.map((_, index) => index),
        anchors: layer.anchors.map((_, index) => index),
      }
    : { points, components, anchors }
  return { ...targets, box: boxOf(layer, targets), wholeLayer }
}

/**
 * The selection as objects that can be aligned or distributed.
 *
 * Points are grouped by their contour, so "align left" moves each contour by
 * one delta rather than collapsing every point onto a single x; anchors and
 * components are objects in their own right.
 */
export function alignItems(
  layer: TransformLayer,
  selection: Selection,
): AlignItem[] {
  const targets = transformTargets(layer, selection)
  const byContour = new Map<number, PointRef[]>()
  for (const ref of targets.points) {
    const list = byContour.get(ref.contour)
    if (list) list.push(ref)
    else byContour.set(ref.contour, [ref])
  }

  const items: AlignItem[] = []
  for (const [contour, points] of byContour) {
    const positions = points
      .map((ref) => layer.contours[contour]?.points[ref.point])
      .filter((point): point is NonNullable<typeof point> => point != null)
      .map((point) => ({ x: point.x, y: point.y }))
    const box = boundsOf(positions)
    if (box) items.push({ box, points })
  }
  for (const index of targets.anchors) {
    const anchor = layer.anchors[index]
    if (!anchor) continue
    items.push({
      box: { x0: anchor.x, y0: anchor.y, x1: anchor.x, y1: anchor.y },
      points: [],
      anchor: index,
    })
  }
  for (const index of targets.components) {
    const bbox = layer.components[index]?.bbox
    if (!bbox) continue
    items.push({
      box: { x0: bbox[0], y0: bbox[1], x1: bbox[2], y1: bbox[3] },
      points: [],
      component: index,
    })
  }
  return items
}
