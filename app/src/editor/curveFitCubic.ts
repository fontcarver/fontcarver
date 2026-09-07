import type { Point2 } from '../canvas/Viewport'

import { findCorners } from './curveFitCorners'
import {
  add,
  basis,
  cubicAt,
  derivative,
  distanceSquared,
  dot,
  length,
  negate,
  quadraticAt,
  scale,
  sub,
  unit,
  type Cubic,
} from './curveFitGeometry'
import type { PathSegment } from './pathGeometry'

/** Default tolerance in font units: how far the fit may stray from a sample. */
export const DEFAULT_FIT_ERROR = 1

/** Newton-Raphson passes before the run is split instead. */
const REPARAMETERISE_PASSES = 4

/** Guards the recursion on pathological input. */
const MAX_DEPTH = 24

/** Samples that seed the nearest-point refinement. */
const NEAREST_SAMPLES = 8

/** Newton steps used to refine the best coarse nearest-point candidate. */
const NEAREST_PASSES = 4

/** Chord-length parameters of one run, normalised to 0 … 1. */
function parameterise(points: readonly Point2[]): number[] {
  const u = [0]
  for (let index = 1; index < points.length; index += 1)
    u.push(
      (u[index - 1] as number) +
        length(sub(points[index] as Point2, points[index - 1] as Point2)),
    )
  const total = u[u.length - 1] as number
  if (total <= 0) return points.map((_, index) => index / (points.length - 1))
  return u.map((value) => value / total)
}

/** The least-squares cubic through `points` with fixed end tangents. */
function generateBezier(
  points: readonly Point2[],
  u: readonly number[],
  tangent1: Point2,
  tangent2: Point2,
): Cubic {
  const first = points[0] as Point2
  const last = points[points.length - 1] as Point2
  let c00 = 0
  let c01 = 0
  let c11 = 0
  let x0 = 0
  let x1 = 0
  for (let index = 0; index < points.length; index += 1) {
    const ui = u[index] as number
    const a0 = scale(tangent1, basis(1, ui))
    const a1 = scale(tangent2, basis(2, ui))
    c00 += dot(a0, a0)
    c01 += dot(a0, a1)
    c11 += dot(a1, a1)
    const onChord = add(
      add(scale(first, basis(0, ui)), scale(first, basis(1, ui))),
      add(scale(last, basis(2, ui)), scale(last, basis(3, ui))),
    )
    const residual = sub(points[index] as Point2, onChord)
    x0 += dot(a0, residual)
    x1 += dot(a1, residual)
  }
  const det = c00 * c11 - c01 * c01
  const chord = length(sub(last, first))
  let alpha1 = det === 0 ? 0 : (x0 * c11 - x1 * c01) / det
  let alpha2 = det === 0 ? 0 : (c00 * x1 - c01 * x0) / det
  const epsilon = 1e-6 * chord
  if (alpha1 < epsilon || alpha2 < epsilon) {
    alpha1 = chord / 3
    alpha2 = chord / 3
  }
  return [
    first,
    add(first, scale(tangent1, alpha1)),
    add(last, scale(tangent2, alpha2)),
    last,
  ]
}

/** One Newton-Raphson step towards the parameter nearest to `point`. */
function newtonStep(bez: Cubic, point: Point2, u: number): number {
  const d1 = derivative(bez)
  const d2: [Point2, Point2] = [
    scale(sub(d1[1], d1[0]), 2),
    scale(sub(d1[2], d1[1]), 2),
  ]
  const on = sub(cubicAt(bez, u), point)
  const slope = quadraticAt(d1, u)
  const curve = add(scale(d2[0], 1 - u), scale(d2[1], u))
  const numerator = dot(on, slope)
  const denominator = dot(slope, slope) + dot(on, curve)
  if (Math.abs(denominator) < 1e-12) return u
  return u - numerator / denominator
}

/**
 * Geometric distance from a point to a cubic.
 *
 * A coarse pass includes both endpoints and the fit parameter, then a clamped
 * Newton solve refines the best candidate. Every reported value is the
 * distance to a point actually evaluated on the curve, so a cusp or a zero
 * derivative can make the estimate conservative but cannot make it accept a
 * curve outside the error budget.
 */
export function distanceToCubic(
  bez: Cubic,
  point: Point2,
  parameter = 0.5,
): number {
  let bestSquared = Infinity
  let at = 0
  const consider = (u: number): void => {
    const candidate = Math.min(1, Math.max(0, u))
    const squared = distanceSquared(cubicAt(bez, candidate), point)
    if (squared < bestSquared) {
      bestSquared = squared
      at = candidate
    }
  }

  for (let step = 0; step <= NEAREST_SAMPLES; step += 1)
    consider(step / NEAREST_SAMPLES)
  if (Number.isFinite(parameter)) consider(parameter)

  let candidate = at
  for (let pass = 0; pass < NEAREST_PASSES; pass += 1) {
    const stepped = newtonStep(bez, point, candidate)
    if (!Number.isFinite(stepped)) break
    const clamped = Math.min(1, Math.max(0, stepped))
    consider(clamped)
    if (Math.abs(clamped - candidate) < 1e-7) break
    candidate = clamped
  }
  return Math.sqrt(bestSquared)
}

/** One Newton-Raphson pass over every fit parameter. */
function reparameterise(
  points: readonly Point2[],
  bez: Cubic,
  u: readonly number[],
): number[] {
  const next: number[] = []
  for (let index = 0; index < u.length; index += 1) {
    const stepped = newtonStep(bez, points[index] as Point2, u[index] as number)
    const floor = index === 0 ? 0 : (next[index - 1] as number)
    next.push(
      Math.min(
        1,
        Math.max(
          floor,
          Number.isFinite(stepped) ? stepped : (u[index] as number),
        ),
      ),
    )
  }
  next[0] = 0
  next[next.length - 1] = 1
  return next
}

/** The worst geometric sample distance and where it occurs. */
function maxError(
  points: readonly Point2[],
  bez: Cubic,
  u: readonly number[],
): { error: number; split: number } {
  let error = 0
  let split = Math.floor(points.length / 2)
  for (let index = 1; index < points.length - 1; index += 1) {
    const distance = distanceToCubic(
      bez,
      points[index] as Point2,
      u[index] as number,
    )
    if (distance >= error) {
      error = distance
      split = index
    }
  }
  return { error, split }
}

/** Fits one run of samples, splitting at its worst point when necessary. */
function fitRun(
  points: readonly Point2[],
  tangent1: Point2,
  tangent2: Point2,
  error: number,
  depth: number,
  out: Cubic[],
): void {
  if (points.length === 2) {
    const first = points[0] as Point2
    const last = points[1] as Point2
    const third = length(sub(last, first)) / 3
    out.push([
      first,
      add(first, scale(tangent1, third)),
      add(last, scale(tangent2, third)),
      last,
    ])
    return
  }

  let u = parameterise(points)
  let bez = generateBezier(points, u, tangent1, tangent2)
  let worst = maxError(points, bez, u)
  let best = { bez, worst }
  for (let pass = 0; pass < REPARAMETERISE_PASSES; pass += 1) {
    if (best.worst.error < error) break
    u = reparameterise(points, bez, u)
    bez = generateBezier(points, u, tangent1, tangent2)
    worst = maxError(points, bez, u)
    if (worst.error < best.worst.error) best = { bez, worst }
  }
  if (best.worst.error < error || depth >= MAX_DEPTH) {
    out.push(best.bez)
    return
  }
  const split = Math.min(Math.max(1, best.worst.split), points.length - 2)
  const centre = centreTangent(points, split)
  fitRun(points.slice(0, split + 1), tangent1, centre, error, depth + 1, out)
  fitRun(points.slice(split), negate(centre), tangent2, error, depth + 1, out)
}

function centreTangent(points: readonly Point2[], index: number): Point2 {
  return unit(sub(points[index - 1] as Point2, points[index + 1] as Point2))
}

function dedupe(points: readonly Point2[]): Point2[] {
  const out: Point2[] = []
  for (const point of points) {
    const last = out[out.length - 1]
    if (
      last &&
      Math.abs(last.x - point.x) < 1e-9 &&
      Math.abs(last.y - point.y) < 1e-9
    )
      continue
    out.push({ x: point.x, y: point.y })
  }
  return out
}

function isLine(bez: Cubic, error: number): boolean {
  const chord = sub(bez[3], bez[0])
  const size = length(chord)
  if (size < 1e-9) return true
  const direction = scale(chord, 1 / size)
  for (const control of [bez[1], bez[2]]) {
    const offset = sub(control, bez[0])
    const along = dot(offset, direction)
    const across = Math.hypot(
      offset.x - direction.x * along,
      offset.y - direction.y * along,
    )
    if (across > error || along < -error || along > size + error) return false
  }
  return true
}

function toSegment(bez: Cubic, error: number): PathSegment {
  if (isLine(bez, error))
    return { kind: 'line', from: bez[0], to: bez[3], controls: [] }
  return { kind: 'cubic', from: bez[0], to: bez[3], controls: [bez[1], bez[2]] }
}

/** What a fit produced: the segments and which of their ends are smooth. */
export interface FittedPath {
  segments: PathSegment[]
  /** `smooth[i]` is whether the point ending segment `i` keeps its tangent. */
  smooth: boolean[]
}

/** Fits a run of samples with cubics, honouring corners. */
export function fitSamples(
  samples: readonly Point2[],
  error = DEFAULT_FIT_ERROR,
): FittedPath {
  const points = dedupe(samples)
  const segments: PathSegment[] = []
  const smooth: boolean[] = []
  if (points.length < 2) return { segments, smooth }
  const bounds = [0, ...findCorners(points), points.length - 1]
  for (let index = 0; index + 1 < bounds.length; index += 1) {
    const from = bounds[index] as number
    const to = bounds[index + 1] as number
    if (to <= from) continue
    const run = points.slice(from, to + 1)
    const cubics: Cubic[] = []
    fitRun(
      run,
      unit(sub(run[1] as Point2, run[0] as Point2)),
      unit(sub(run[run.length - 2] as Point2, run[run.length - 1] as Point2)),
      error,
      0,
      cubics,
    )
    cubics.forEach((bez, position) => {
      segments.push(toSegment(bez, error))
      smooth.push(position < cubics.length - 1)
    })
    if (smooth.length > 0) smooth[smooth.length - 1] = false
  }
  return { segments, smooth }
}
