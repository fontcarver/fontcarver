/**
 * Fitting a freehand stroke to cubic segments.
 *
 * The Pencil samples a few hundred points and has to hand the kernel a
 * *contour*, not a polyline. The fit happens here, in the browser, because the
 * only whole-path simplifier the kernel offers (`PathOp::Simplify`) rewrites
 * every contour of the layer: drawing one stroke would silently refit outlines
 * the user did not touch. Fitting locally keeps a stroke to exactly one
 * `AddContour`.
 *
 * The algorithm is Philip J. Schneider's least-squares cubic fit (Graphics
 * Gems, 1990) — chord-length parameterisation, a least-squares Bézier for the
 * two tangent magnitudes, Newton-Raphson reparameterisation when the first fit
 * is close, and a split at the worst point when it is not — preceded by a
 * corner pass, so a stroke with a deliberate corner keeps it instead of being
 * rounded away.
 */

import type { Contour } from '../api'
import type { Point2 } from '../canvas/Viewport'

import {
  pathToContour,
  type LayerCurves,
  type PathSegment,
} from './pathGeometry'

/** Default tolerance in font units: how far the fit may stray from a sample. */
export const DEFAULT_FIT_ERROR = 1

/** A turn sharper than this counts as a corner and is not fitted through. */
export const CORNER_ANGLE = Math.PI / 3

/** How many samples either side a corner test looks at, to ride out noise. */
const CORNER_WINDOW = 2

/** Newton-Raphson passes before the run is split instead. */
const REPARAMETERISE_PASSES = 4

/** Guards the recursion on pathological input. */
const MAX_DEPTH = 24

/** How far the quadratic approximation of a cubic may stray, per unit of c. */
const QUAD_ERROR_FACTOR = Math.sqrt(3) / 18

function sub(a: Point2, b: Point2): Point2 {
  return { x: a.x - b.x, y: a.y - b.y }
}

function add(a: Point2, b: Point2): Point2 {
  return { x: a.x + b.x, y: a.y + b.y }
}

function scale(a: Point2, by: number): Point2 {
  return { x: a.x * by, y: a.y * by }
}

function dot(a: Point2, b: Point2): number {
  return a.x * b.x + a.y * b.y
}

function length(a: Point2): number {
  return Math.hypot(a.x, a.y)
}

function unit(a: Point2): Point2 {
  const size = length(a)
  return size < 1e-12 ? { x: 0, y: 0 } : { x: a.x / size, y: a.y / size }
}

function negate(a: Point2): Point2 {
  return { x: -a.x, y: -a.y }
}

/** Bernstein basis of degree three. */
function basis(index: 0 | 1 | 2 | 3, u: number): number {
  const t = 1 - u
  if (index === 0) return t * t * t
  if (index === 1) return 3 * u * t * t
  if (index === 2) return 3 * u * u * t
  return u * u * u
}

/** A cubic as its four control points. */
type Cubic = [Point2, Point2, Point2, Point2]

function cubicAt(bez: Cubic, u: number): Point2 {
  let point = { x: 0, y: 0 }
  for (let index = 0; index < 4; index += 1)
    point = add(
      point,
      scale(bez[index] as Point2, basis(index as 0 | 1 | 2 | 3, u)),
    )
  return point
}

/** The first derivative of a cubic, as a quadratic in the same basis. */
function derivative(bez: Cubic): [Point2, Point2, Point2] {
  return [
    scale(sub(bez[1], bez[0]), 3),
    scale(sub(bez[2], bez[1]), 3),
    scale(sub(bez[3], bez[2]), 3),
  ]
}

function quadraticAt(q: readonly [Point2, Point2, Point2], u: number): Point2 {
  const t = 1 - u
  return add(
    add(scale(q[0], t * t), scale(q[1], 2 * t * u)),
    scale(q[2], u * u),
  )
}

/**
 * One Newton-Raphson pass over every parameter.
 *
 * The result is clamped to 0 … 1 and forced to stay increasing: an unbounded
 * step can push a sample's parameter past its neighbours', and a
 * non-monotonic parameterisation makes the next least-squares fit meaningless.
 */
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
  // The ends are fixed by construction; nudging them only distorts the fit.
  next[0] = 0
  next[next.length - 1] = 1
  return next
}

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

/**
 * The least-squares cubic through `points` with the given end tangents.
 *
 * Only the two tangent magnitudes are free — the endpoints and the tangent
 * directions are fixed — so the fit is a 2×2 linear system.
 */
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
  // A degenerate or inverted solution falls back to Wu/Barsky's thirds, which
  // is what a straight run wants anyway.
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

/** How finely a curve is sampled before the nearest point is refined. */
const NEAREST_SAMPLES = 16

/**
 * The true distance from a point to a cubic.
 *
 * The parametric distance — sample `i` against the curve at `u[i]` — is what
 * the least-squares fit minimises, but it is not what the tolerance means, and
 * on a badly parameterised run it reads far smaller than the curve's real
 * deviation. So acceptance is decided on the geometric distance.
 */
function distanceToCubic(bez: Cubic, point: Point2): number {
  let best = Infinity
  let at = 0
  for (let step = 0; step <= NEAREST_SAMPLES; step += 1) {
    const t = step / NEAREST_SAMPLES
    const distance = length(sub(cubicAt(bez, t), point))
    if (distance < best) {
      best = distance
      at = t
    }
  }
  let span = 1 / NEAREST_SAMPLES
  for (let pass = 0; pass < 10; pass += 1) {
    span /= 2
    for (const t of [Math.max(0, at - span), Math.min(1, at + span)]) {
      const distance = length(sub(cubicAt(bez, t), point))
      if (distance < best) {
        best = distance
        at = t
      }
    }
  }
  return best
}

/** The worst distance from a sample to the fitted curve, and where it is. */
function maxError(
  points: readonly Point2[],
  bez: Cubic,
): { error: number; split: number } {
  let error = 0
  let split = Math.floor(points.length / 2)
  for (let index = 1; index < points.length - 1; index += 1) {
    const distance = distanceToCubic(bez, points[index] as Point2)
    if (distance >= error) {
      error = distance
      split = index
    }
  }
  return { error, split }
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

/** Fits one run of samples, splitting at its worst point when it has to. */
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
  let worst = maxError(points, bez)
  let best = { bez, worst }
  // Improve the parameterisation before giving up on one segment. Schneider
  // gates this on `error²`, which only bites for tolerances above one unit —
  // at the one-unit tolerance a font editor wants it would never run, and the
  // chord-length guess alone splits a smooth stroke far more than it needs to.
  for (let pass = 0; pass < REPARAMETERISE_PASSES; pass += 1) {
    if (best.worst.error < error) break
    u = reparameterise(points, bez, u)
    bez = generateBezier(points, u, tangent1, tangent2)
    worst = maxError(points, bez)
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

/** The tangent a split inherits: the direction across the split point. */
function centreTangent(points: readonly Point2[], index: number): Point2 {
  const before = points[index - 1] as Point2
  const after = points[index + 1] as Point2
  return unit(sub(before, after))
}

/** Drops samples that repeat the previous position. */
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

/** Whether a fitted cubic is within `error` of its own chord. */
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
    // A control off the chord, or past either end, means real curvature.
    if (across > error || along < -error || along > size + error) return false
  }
  return true
}

/** A cubic as a path segment, straightened when it is really a line. */
function toSegment(bez: Cubic, error: number): PathSegment {
  if (isLine(bez, error))
    return { kind: 'line', from: bez[0], to: bez[3], controls: [] }
  return { kind: 'cubic', from: bez[0], to: bez[3], controls: [bez[1], bez[2]] }
}

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
      // Inside a run the fit is tangent-continuous, so those nodes are
      // smooth; the node that ends a run is a corner the user drew.
      smooth.push(position < cubics.length - 1)
    })
    if (smooth.length > 0) smooth[smooth.length - 1] = false
  }
  return { segments, smooth }
}

/**
 * How the tolerance is split on a quadratic layer.
 *
 * A quadratic layer is fitted twice — samples to cubics, cubics to quadratics
 * — and the two errors add, so each gets a share rather than the whole budget.
 * The conversion takes the larger share because its bound falls by eight for
 * every subdivision: giving it room costs a fraction of a segment, while
 * giving the cubic fit room costs whole segments.
 */
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
  const fitted = fitSamples(samples, fitError)
  if (fitted.segments.length === 0) return null
  const segments: PathSegment[] = []
  const smooth: boolean[] = []
  fitted.segments.forEach((segment, index) => {
    const parts = quadratic ? cubicToQuadratics(segment, quadError) : [segment]
    parts.forEach((part, position) => {
      segments.push(part)
      // A quadratic run stands in for one cubic, so its inner joins are
      // smooth whatever the cubic's own end was.
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
