/**
 * The path the pen accumulates before it is committed.
 *
 * A pen node is an on-curve point with the two handles that meet at it, which
 * is how the user thinks about drawing and what a drag on the node sets. It
 * becomes a stored contour only once — when the path closes or finishes — so a
 * whole path is one undo entry, and the conversion lives here so it can be
 * tested without a pointer.
 */

import type { Contour } from '../../api'
import type { Point2 } from '../../canvas/Viewport'
import {
  contourToPath,
  pathToContour,
  type GeometricPath,
  type LayerCurves,
  type PathSegment,
} from '../pathGeometry'

/** One on-curve point of the path being drawn, with its handles. */
export interface PenNode {
  point: Point2
  /** Handle towards the previous node, or `null` for a corner. */
  incoming: Point2 | null
  /** Handle towards the next node. */
  outgoing: Point2 | null
  smooth: boolean
}

/** A node with no handles. */
export function penNode(point: Point2): PenNode {
  return { point, incoming: null, outgoing: null, smooth: false }
}

/** The same node with its handles swapped, used when a path is reversed. */
function flipped(node: PenNode): PenNode {
  return {
    point: node.point,
    incoming: node.outgoing,
    outgoing: node.incoming,
    smooth: node.smooth,
  }
}

/** The geometric path a run of pen nodes describes. */
export function penPath(
  nodes: readonly PenNode[],
  closed: boolean,
  curves: LayerCurves,
): GeometricPath | null {
  const count = nodes.length
  if (count < 2) return null
  const segments: PathSegment[] = []
  const last = closed ? count : count - 1
  for (let index = 0; index < last; index += 1) {
    const from = nodes[index] as PenNode
    const to = nodes[(index + 1) % count] as PenNode
    if (curves === 'quadratic') {
      const control = from.outgoing ?? to.incoming
      segments.push(
        control
          ? {
              kind: 'quad',
              from: from.point,
              to: to.point,
              controls: [control],
            }
          : { kind: 'line', from: from.point, to: to.point, controls: [] },
      )
      continue
    }
    if (!from.outgoing && !to.incoming) {
      segments.push({
        kind: 'line',
        from: from.point,
        to: to.point,
        controls: [],
      })
      continue
    }
    segments.push({
      kind: 'cubic',
      from: from.point,
      to: to.point,
      controls: [from.outgoing ?? from.point, to.incoming ?? to.point],
    })
  }
  const first = nodes[0] as PenNode
  return { closed, start: first.point, segments }
}

/** The stored contour a run of pen nodes commits as. */
export function penContour(
  nodes: readonly PenNode[],
  closed: boolean,
  curves: LayerCurves,
): Contour | null {
  const path = penPath(nodes, closed, curves)
  if (!path) return null
  const count = nodes.length
  return pathToContour(path, (index) => {
    const node = index < 0 ? nodes[0] : nodes[(index + 1) % count]
    return node?.smooth ?? false
  })
}

/**
 * The pen nodes of an existing contour, so a click on one of its ends can
 * carry on drawing from there. `fromEnd` reverses the run when the path was
 * picked up at its first point, so new nodes always append at the tail.
 */
export function nodesOfContour(
  contour: Contour,
  fromEnd: 'start' | 'end',
): PenNode[] {
  const path = contourToPath(contour)
  const smooth = contour.points
    .filter((point) => point.kind !== 'OffCurve')
    .map((point) => point.smooth)
  const nodes: PenNode[] = [penNode(path.start)]
  path.segments.forEach((segment, index) => {
    const head = nodes[index] as PenNode
    if (segment.kind === 'cubic') {
      head.outgoing = segment.controls[0] ?? null
      nodes.push({
        point: segment.to,
        incoming: segment.controls[1] ?? null,
        outgoing: null,
        smooth: false,
      })
      return
    }
    if (segment.kind === 'quad') {
      head.outgoing = segment.controls[0] ?? null
      nodes.push({
        point: segment.to,
        incoming: null,
        outgoing: null,
        smooth: false,
      })
      return
    }
    nodes.push(penNode(segment.to))
  })
  nodes.forEach((node, index) => {
    node.smooth = smooth[index] ?? false
  })
  return fromEnd === 'end' ? nodes : nodes.reverse().map(flipped)
}
