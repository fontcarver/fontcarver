import type { Point2 } from '../canvas/Viewport'

import { dot, length, sub, unit } from './curveFitGeometry'

/** A turn sharper than this counts as a corner and is not fitted through. */
export const CORNER_ANGLE = Math.PI / 3

/** How many samples either side a corner test looks at, to ride out noise. */
const CORNER_WINDOW = 2

/**
 * The indexes where the stroke turns sharply.
 *
 * The direction is measured across a small window rather than between
 * neighbours, so the jitter of a hand-drawn line does not read as a corner.
 */
export function findCorners(
  points: readonly Point2[],
  angle = CORNER_ANGLE,
): number[] {
  const corners: number[] = []
  for (let index = 1; index < points.length - 1; index += 1) {
    const back = points[Math.max(0, index - CORNER_WINDOW)] as Point2
    const forward = points[
      Math.min(points.length - 1, index + CORNER_WINDOW)
    ] as Point2
    const here = points[index] as Point2
    const incoming = unit(sub(here, back))
    const outgoing = unit(sub(forward, here))
    if (length(incoming) === 0 || length(outgoing) === 0) continue
    const turn = Math.acos(Math.min(1, Math.max(-1, dot(incoming, outgoing))))
    if (turn > angle) corners.push(index)
  }
  return corners
}
