/**
 * The optimistic geometry a drag is showing.
 *
 * The canvas draws from this on every pointer move, so it must describe
 * exactly what the gesture's commands will do and nothing more: anchors and
 * components ride a single `TransformSelection`, which a per-point regime
 * never sends, so previewing them moving there would promise something the
 * gesture cannot apply and they would snap back on the reload that follows.
 */

import type { Anchor, ComponentView, Contour, PointRef } from '../../../api'
import type { Point2 } from '../../../canvas/Viewport'
import {
  setPointPosition,
  translateEach,
  translatePoints,
} from '../../outlineGeometry'

/** The opposite handle a Ctrl+⌥ handle drag mirrors onto. */
export interface MirrorTarget {
  handle: PointRef
  /** The node both handles hang from, which stays put. */
  node: Point2
  /** Where the mirrored handle started. */
  from: Point2
}

/** The optimistic local geometry a drag is showing. */
export interface DragPreview {
  contours: readonly Contour[]
  anchors: readonly Anchor[]
  components: readonly ComponentView[]
}

/** Everything the preview is assembled from. */
export interface PreviewInput {
  contours: readonly Contour[]
  anchors: readonly Anchor[]
  components: readonly ComponentView[]
  /** Stored points the gesture translates by `delta`. */
  movedRefs: readonly PointRef[]
  /** Anchor indexes the gesture's `TransformSelection` carries. */
  anchorIndexes: readonly number[]
  /** Component indexes the gesture's `TransformSelection` carries. */
  componentIndexes: readonly number[]
  delta: Point2
  /** Per-point deltas, when the gesture constrains each point separately. */
  deltas: ReadonlyMap<string, Point2> | null
  mirror: MirrorTarget | null
  /** Where the mirrored handle currently sits. */
  mirrorPoint: Point2 | null
}

/** Translates one component's stored transform and bounds. */
function moveComponent(view: ComponentView, delta: Point2): ComponentView {
  return {
    ...view,
    component: {
      ...view.component,
      transform: [
        view.component.transform[0],
        view.component.transform[1],
        view.component.transform[2],
        view.component.transform[3],
        view.component.transform[4] + delta.x,
        view.component.transform[5] + delta.y,
      ] as [number, number, number, number, number, number],
    },
    bbox: view.bbox
      ? ([
          view.bbox[0] + delta.x,
          view.bbox[1] + delta.y,
          view.bbox[2] + delta.x,
          view.bbox[3] + delta.y,
        ] as [number, number, number, number])
      : view.bbox,
  }
}

/** Builds the geometry the canvas should draw for the current delta. */
export function buildDragPreview(input: PreviewInput): DragPreview {
  const { contours, anchors, components, delta } = input
  const moved = input.deltas
    ? translateEach(contours, input.deltas)
    : translatePoints(contours, input.movedRefs, delta.x, delta.y)
  const withMirror =
    input.mirror && input.mirrorPoint
      ? setPointPosition(
          moved,
          input.mirror.handle,
          input.mirrorPoint.x,
          input.mirrorPoint.y,
        )
      : moved
  // Anchors and components ride the gesture's single `TransformSelection`,
  // which a per-point regime does not send.
  const transforming =
    input.deltas === null &&
    (input.anchorIndexes.length > 0 || input.componentIndexes.length > 0)
  const movedAnchors =
    !transforming || input.anchorIndexes.length === 0
      ? anchors
      : anchors.map((anchor, index) =>
          input.anchorIndexes.includes(index)
            ? { ...anchor, x: anchor.x + delta.x, y: anchor.y + delta.y }
            : anchor,
        )
  const movedComponents =
    !transforming || input.componentIndexes.length === 0
      ? components
      : components.map((view, index) =>
          input.componentIndexes.includes(index)
            ? moveComponent(view, delta)
            : view,
        )
  return {
    contours: withMirror,
    anchors: movedAnchors,
    components: movedComponents,
  }
}
