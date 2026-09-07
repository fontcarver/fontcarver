import type { Point2 } from '../canvas/Viewport'

import type { PathSegment } from './pathGeometry'
import { add, length, scale, sub, type Cubic } from './curveFitGeometry'

/** How far the quadratic approximation of a cubic may stray, per unit of c. */
const QUAD_ERROR_FACTOR = Math.sqrt(3) / 18

/** Splits a cubic in half; used when one quadratic cannot express it. */
function halve(bez: Cubic): [Cubic, Cubic] {
  const mid = (a: Point2, b: Point2) => scale(add(a, b), 0.5)
  const a = mid(bez[0], bez[1])
  const b = mid(bez[1], bez[2])
  const c = mid(bez[2], bez[3])
  const d = mid(a, b)
  const e = mid(b, c)
  const f = mid(d, e)
  return [
    [bez[0], a, d, f],
    [f, e, c, bez[3]],
  ]
}

/**
 * One or more quadratics for a cubic, within `error`.
 *
 * The mid-point quadratic of a cubic is exact to `√3/18 · |c|`, where `c` is
 * the cubic's third difference, so the bound decides the subdivision rather
 * than a fixed depth.
 */
export function cubicToQuadratics(
  segment: PathSegment,
  error: number,
): PathSegment[] {
  if (segment.kind !== 'cubic') return [segment]
  const c1 = segment.controls[0] ?? segment.from
  const c2 = segment.controls[1] ?? segment.to
  const out: PathSegment[] = []
  const emit = (bez: Cubic, depth: number): void => {
    const third = sub(add(bez[3], scale(sub(bez[1], bez[2]), 3)), bez[0])
    if (depth >= 8 || QUAD_ERROR_FACTOR * length(third) <= error) {
      const control = scale(
        sub(scale(add(bez[1], bez[2]), 3), add(bez[0], bez[3])),
        0.25,
      )
      out.push({
        kind: 'quad',
        from: bez[0],
        to: bez[3],
        controls: [control],
      })
      return
    }
    const [left, right] = halve(bez)
    emit(left, depth + 1)
    emit(right, depth + 1)
  }
  emit([segment.from, c1, c2, segment.to], 0)
  return out
}
