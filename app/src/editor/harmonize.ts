/** Optimistic G2 projection, matching `fc_geom::harmonize_nodes`. */

import type { Contour } from '../api'
import type { Point2 } from '../canvas/Viewport'

interface Join {
  node: number
  before: number
  after: number
  outerBefore: number
  outerAfter: number
  center: Point2
  tangent: Point2
  total: number
}

function prepareJoin(contour: Contour, node: number): Join | null {
  const { points, closed } = contour
  const count = points.length
  const center = points[node]
  if (
    !center ||
    center.kind !== 'Curve' ||
    count < 6 ||
    (!closed && (node < 3 || node + 3 >= count))
  )
    return null
  const before = (node + count - 1) % count
  const outerBefore = (node + count - 2) % count
  const previous = (node + count - 3) % count
  const after = (node + 1) % count
  const outerAfter = (node + 2) % count
  const next = (node + 3) % count
  if (
    [before, outerBefore, after, outerAfter].some(
      (index) => points[index].kind !== 'OffCurve',
    ) ||
    points[previous].kind === 'OffCurve' ||
    points[next].kind !== 'Curve'
  )
    return null
  const incoming = {
    x: center.x - points[before].x,
    y: center.y - points[before].y,
  }
  const outgoing = {
    x: points[after].x - center.x,
    y: points[after].y - center.y,
  }
  const incomingLength = Math.hypot(incoming.x, incoming.y)
  const outgoingLength = Math.hypot(outgoing.x, outgoing.y)
  const total = incomingLength + outgoingLength
  if (!Number.isFinite(total) || incomingLength <= 0 || outgoingLength <= 0)
    return null
  const x = incoming.x / incomingLength + outgoing.x / outgoingLength
  const y = incoming.y / incomingLength + outgoing.y / outgoingLength
  const length = Math.hypot(x, y)
  if (!Number.isFinite(length) || length <= 1e-12) return null
  return {
    node,
    before,
    after,
    outerBefore,
    outerAfter,
    center,
    tangent: { x: x / length, y: y / length },
    total,
  }
}

function beforeLength(contour: Contour, join: Join): number {
  const before = contour.points[join.before]
  return Math.hypot(join.center.x - before.x, join.center.y - before.y)
}

function balancedLength(contour: Contour, join: Join): number | null {
  const before = contour.points[join.outerBefore]
  const after = contour.points[join.outerAfter]
  const bx = before.x - join.center.x
  const by = before.y - join.center.y
  const ax = after.x - join.center.x
  const ay = after.y - join.center.y
  const incoming = join.tangent.x * by - join.tangent.y * bx
  const outgoing = join.tangent.x * ay - join.tangent.y * ax
  if (!Number.isFinite(incoming) || !Number.isFinite(outgoing)) return null
  const tolerance = Math.max(Math.hypot(bx, by), Math.hypot(ax, ay)) * 1e-14
  if (Math.abs(incoming) <= tolerance && Math.abs(outgoing) <= tolerance)
    return beforeLength(contour, join)
  if (
    Math.abs(incoming) <= tolerance ||
    Math.abs(outgoing) <= tolerance ||
    incoming > 0 !== outgoing > 0
  )
    return null
  const left = Math.sqrt(Math.abs(incoming))
  const right = Math.sqrt(Math.abs(outgoing))
  const length = join.total * (left / (left + right))
  return Number.isFinite(length) && length > 0 && length < join.total
    ? length
    : null
}

function writeHandles(contour: Contour, join: Join, incoming: number): void {
  const before = contour.points[join.before]
  const after = contour.points[join.after]
  before.x = join.center.x - join.tangent.x * incoming
  before.y = join.center.y - join.tangent.y * incoming
  after.x = join.center.x + join.tangent.x * (join.total - incoming)
  after.y = join.center.y + join.tangent.y * (join.total - incoming)
}

function alreadyHarmonized(contour: Contour, join: Join): boolean {
  const before = contour.points[join.before]
  const after = contour.points[join.after]
  const incoming = beforeLength(contour, join)
  const outgoing = Math.hypot(after.x - join.center.x, after.y - join.center.y)
  const ix = (join.center.x - before.x) / incoming
  const iy = (join.center.y - before.y) / incoming
  const ox = (after.x - join.center.x) / outgoing
  const oy = (after.y - join.center.y) / outgoing
  const expected = balancedLength(contour, join)
  return (
    Math.abs(ix * oy - iy * ox) <= 1e-12 &&
    ix * ox + iy * oy > 0 &&
    expected !== null &&
    Math.abs(incoming - expected) <= join.total * 1e-12
  )
}

/**
 * Projects cubic joins together without changing the input or moving nodes.
 * Returns `null` when the kernel would reject an unsupported, incompatible,
 * degenerate, or non-convergent join. Retains persistent super-smooth flags.
 */
export function harmonizeNodes(
  contour: Contour,
  nodes: readonly number[],
): Contour | null {
  if (nodes.length === 0) return contour
  const joins: Join[] = []
  for (const node of [...new Set(nodes)].sort((a, b) => a - b)) {
    const join = prepareJoin(contour, node)
    if (!join) return null
    joins.push(join)
  }
  const result = {
    ...contour,
    points: contour.points.map((point) => ({ ...point })),
  }
  if (joins.every((join) => alreadyHarmonized(contour, join))) {
    for (const join of joins) result.points[join.node].smooth = true
    return result
  }
  for (const join of joins)
    writeHandles(result, join, beforeLength(result, join))
  for (let pass = 0; pass < 128; pass += 1) {
    for (const join of joins) {
      const length = balancedLength(result, join)
      if (length === null) return null
      writeHandles(result, join, length)
    }
    let converged = true
    for (const join of joins) {
      const expected = balancedLength(result, join)
      if (expected === null) return null
      if (Math.abs(beforeLength(result, join) - expected) > join.total * 1e-12)
        converged = false
    }
    if (converged) {
      for (const join of joins) result.points[join.node].smooth = true
      return result
    }
  }
  return null
}

/** Reapplies persistent G2 constraints atomically across a layer preview. */
export function maintainSuperSmoothContours(
  contours: readonly Contour[],
): Contour[] | null {
  const result: Contour[] = []
  for (const contour of contours) {
    const nodes = contour.points.flatMap((point, index) =>
      point.super_smooth ? [index] : [],
    )
    const projected = harmonizeNodes(contour, nodes)
    if (!projected) return null
    result.push(projected)
  }
  return result
}
