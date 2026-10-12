import type { Point2 } from '../canvas/Viewport'

/** A cubic Bézier as its four control points. */
export type Cubic = readonly [Point2, Point2, Point2, Point2]

export function sub(a: Point2, b: Point2): Point2 {
  return { x: a.x - b.x, y: a.y - b.y }
}

export function add(a: Point2, b: Point2): Point2 {
  return { x: a.x + b.x, y: a.y + b.y }
}

export function scale(a: Point2, by: number): Point2 {
  return { x: a.x * by, y: a.y * by }
}

export function dot(a: Point2, b: Point2): number {
  return a.x * b.x + a.y * b.y
}

export function length(a: Point2): number {
  return Math.hypot(a.x, a.y)
}

export function unit(a: Point2): Point2 {
  const size = length(a)
  return size < 1e-12 ? { x: 0, y: 0 } : { x: a.x / size, y: a.y / size }
}

export function negate(a: Point2): Point2 {
  return { x: -a.x, y: -a.y }
}

/** Bernstein basis of degree three. */
export function basis(index: 0 | 1 | 2 | 3, u: number): number {
  const t = 1 - u
  if (index === 0) return t * t * t
  if (index === 1) return 3 * u * t * t
  if (index === 2) return 3 * u * u * t
  return u * u * u
}

export function cubicAt(bez: Cubic, u: number): Point2 {
  let point = { x: 0, y: 0 }
  for (let index = 0; index < 4; index += 1)
    point = add(
      point,
      scale(bez[index] as Point2, basis(index as 0 | 1 | 2 | 3, u)),
    )
  return point
}

/** The first derivative of a cubic, as a quadratic in the same basis. */
export function derivative(bez: Cubic): [Point2, Point2, Point2] {
  return [
    scale(sub(bez[1], bez[0]), 3),
    scale(sub(bez[2], bez[1]), 3),
    scale(sub(bez[3], bez[2]), 3),
  ]
}

export function quadraticAt(
  q: readonly [Point2, Point2, Point2],
  u: number,
): Point2 {
  const t = 1 - u
  return add(
    add(scale(q[0], t * t), scale(q[1], 2 * t * u)),
    scale(q[2], u * u),
  )
}

export function distanceSquared(a: Point2, b: Point2): number {
  const dx = a.x - b.x
  const dy = a.y - b.y
  return dx * dx + dy * dy
}
