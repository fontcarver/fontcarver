/**
 * Affine transforms in the kernel's own coefficient order.
 *
 * `Command::TransformSelection` takes kurbo's `[a, b, c, d, e, f]`, which maps
 * `(x, y)` to `(a·x + c·y + e, b·x + d·y + f)`, and composes components as
 * `affine * component.transform` — the component's own transform first. Both
 * conventions are reproduced here so an optimistic preview and the applied
 * command cannot disagree about which way round a rotation goes.
 */

import type { FontBox, Point2 } from '../canvas/Viewport'

/** Kurbo affine coefficients. */
export type Affine = [number, number, number, number, number, number]

/** The transform that changes nothing. */
export const IDENTITY: Affine = [1, 0, 0, 1, 0, 0]

/** Maps one point. */
export function applyAffine(affine: Affine, point: Point2): Point2 {
  const [a, b, c, d, e, f] = affine
  return {
    x: a * point.x + c * point.y + e,
    y: b * point.x + d * point.y + f,
  }
}

/** `a ∘ b`: `b` is applied first, then `a`. */
export function composeAffine(a: Affine, b: Affine): Affine {
  const [a1, b1, c1, d1, e1, f1] = a
  const [a2, b2, c2, d2, e2, f2] = b
  return [
    a1 * a2 + c1 * b2,
    b1 * a2 + d1 * b2,
    a1 * c2 + c1 * d2,
    b1 * c2 + d1 * d2,
    a1 * e2 + c1 * f2 + e1,
    b1 * e2 + d1 * f2 + f1,
  ]
}

/** Whether a transform can be undone; a zero scale cannot. */
export function isInvertible(affine: Affine): boolean {
  const det = affine[0] * affine[3] - affine[1] * affine[2]
  return Number.isFinite(det) && Math.abs(det) > 1e-12
}

/**
 * The inverse, or the identity when the transform is singular.
 *
 * A degenerate component transform (a zero scale) cannot be undone; returning
 * the identity keeps the caller drawing something rather than `NaN`.
 */
export function invertAffine(affine: Affine): Affine {
  if (!isInvertible(affine)) return IDENTITY
  const [a, b, c, d, e, f] = affine
  const det = a * d - b * c
  const ia = d / det
  const ib = -b / det
  const ic = -c / det
  const id = a / det
  return [ia, ib, ic, id, -(ia * e + ic * f), -(ib * e + id * f)]
}

/** Wraps a linear transform so it happens about `origin`. */
export function about(origin: Point2, linear: Affine): Affine {
  const [a, b, c, d] = linear
  return [
    a,
    b,
    c,
    d,
    origin.x - (a * origin.x + c * origin.y),
    origin.y - (b * origin.x + d * origin.y),
  ]
}

/** A rotation by `radians`, counter-clockwise in font space, about `origin`. */
export function rotateAbout(origin: Point2, radians: number): Affine {
  const cos = Math.cos(radians)
  const sin = Math.sin(radians)
  return about(origin, [cos, sin, -sin, cos, 0, 0])
}

/** A scale about `origin`. */
export function scaleAbout(origin: Point2, sx: number, sy: number): Affine {
  return about(origin, [sx, 0, 0, sy, 0, 0])
}

/** The box that contains the four transformed corners of `box`. */
export function transformBox(affine: Affine, box: FontBox): FontBox {
  const corners = [
    applyAffine(affine, { x: box.x0, y: box.y0 }),
    applyAffine(affine, { x: box.x1, y: box.y0 }),
    applyAffine(affine, { x: box.x1, y: box.y1 }),
    applyAffine(affine, { x: box.x0, y: box.y1 }),
  ]
  return {
    x0: Math.min(...corners.map((corner) => corner.x)),
    y0: Math.min(...corners.map((corner) => corner.y)),
    x1: Math.max(...corners.map((corner) => corner.x)),
    y1: Math.max(...corners.map((corner) => corner.y)),
  }
}
