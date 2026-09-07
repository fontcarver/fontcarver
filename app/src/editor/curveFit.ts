/**
 * Fitting a freehand stroke to cubic or quadratic contour segments.
 *
 * The Pencil fits locally because the kernel's whole-path simplifier would
 * rewrite every contour in the layer. The implementation is split into the
 * cubic fitter, corner detection and quadratic conversion so each numerical
 * responsibility stays independently reviewable.
 */

import type { Contour } from '../api'
import type { Point2 } from '../canvas/Viewport'

import { DEFAULT_FIT_ERROR, fitSamples, type FittedPath } from './curveFitCubic'
import { cubicToQuadratics } from './curveFitQuadratic'
import {
  pathToContour,
  type LayerCurves,
  type PathSegment,
} from './pathGeometry'

export {
  DEFAULT_FIT_ERROR,
  distanceToCubic,
  fitSamples,
  type FittedPath,
} from './curveFitCubic'
export { CORNER_ANGLE, findCorners } from './curveFitCorners'
export { cubicToQuadratics } from './curveFitQuadratic'

/** Cubic fitting's share of a quadratic layer's total error budget. */
const CUBIC_SHARE = 0.4

/** The contour a freehand stroke commits as: one fitted path, no kernel pass. */
export function fitContour(
  samples: readonly Point2[],
  closed: boolean,
  curves: LayerCurves,
  error = DEFAULT_FIT_ERROR,
): Contour | null {
  const quadratic = curves === 'quadratic'
  const fitError = quadratic ? error * CUBIC_SHARE : error
  const quadError = error * (1 - CUBIC_SHARE)
  const fitted: FittedPath = fitSamples(samples, fitError)
  if (fitted.segments.length === 0) return null
  const segments: PathSegment[] = []
  const smooth: boolean[] = []
  fitted.segments.forEach((segment, index) => {
    const parts = quadratic ? cubicToQuadratics(segment, quadError) : [segment]
    parts.forEach((part, position) => {
      segments.push(part)
      smooth.push(
        position < parts.length - 1 ? true : (fitted.smooth[index] ?? false),
      )
    })
  })
  const first = segments[0] as PathSegment
  return pathToContour({ closed, start: first.from, segments }, (index) =>
    index < 0 ? false : (smooth[index] ?? false),
  )
}
