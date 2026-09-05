/**
 * Path geometry the drawing tools work in.
 *
 * Editing commands address *stored* point indexes, but the knife, the measure
 * and the pen think in explicit segments: where a line crosses a curve, how a
 * contour splits at a parameter, which way a contour winds. This module is
 * that second view — a contour resolved into segments, and the pure operations
 * over it — with `pathToContour` turning the result back into the UFO point
 * list a `Command::AddContour` takes.
 *
 * Everything here is font space, y-up, and free of the kernel: a cut or a
 * measurement is computed locally so the pointer never waits on a round trip.
 */

import type { Contour, Point, PointKind } from '../api'
import type { FontBox, Point2 } from '../canvas/Viewport'

import { resolveContour } from '../views/glyph/contourGeometry'

/** Which curve flavour a layer is drawn in. */
export type LayerCurves = 'cubic' | 'quadratic'

/** One geometric segment: a line, or a curve with its controls in order. */
export interface PathSegment {
  kind: 'line' | 'quad' | 'cubic'
  from: Point2
  to: Point2
  /** Off-curve controls in path order; empty for a line. */
  controls: Point2[]
}

/** A contour as geometry rather than as a stored point list. */
export interface GeometricPath {
  closed: boolean
  start: Point2
  segments: PathSegment[]
}

/** Where a straight line meets a path. */
export interface PathLineHit {
  /** Position along the path: segment index plus the parameter inside it. */
  u: number
  /** Position along the query line, 0 at its start and 1 at its end. */
  s: number
  point: Point2
}

/** How finely a curved segment is sampled before a crossing is refined. */
export const LINE_HIT_SAMPLES = 16

function at(point: Point2): Point2 {
  return { x: point.x, y: point.y }
}

function lerp(a: Point2, b: Point2, t: number): Point2 {
  return { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t }
}

/** Resolves one stored contour into explicit segments. */
export function contourToPath(contour: Contour): GeometricPath {
  const resolved = resolveContour(contour)
  const start = resolved.start ?? { x: 0, y: 0 }
  let from: Point2 = at(start)
  const segments: PathSegment[] = []
  for (const segment of resolved.segments) {
    const to = at(segment.to)
    const controls =
      segment.kind === 'quad'
        ? [at(segment.control)]
        : segment.kind === 'cubic'
          ? [at(segment.c1), at(segment.c2)]
          : []
    segments.push({ kind: segment.kind, from, to, controls })
    from = to
  }
  return { closed: contour.closed, start: at(start), segments }
}

function kindOf(segment: PathSegment): PointKind {
  if (segment.kind === 'line') return 'Line'
  return segment.kind === 'quad' ? 'QCurve' : 'Curve'
}

/**
 * One stored point of a rebuilt contour.
 *
 * `tt_index` is always `Unnumbered`: a cut, an extend or a fitted stroke
 * produces points that did not exist in the source, so there is no TrueType
 * point number to carry over, and inventing one would break the hinting that
 * numbers it. The contour's own `name` and `id` *are* carried, by
 * `pathToContour`'s `identity`.
 */
function storedPoint(point: Point2, kind: PointKind, smooth = false): Point {
  return {
    x: point.x,
    y: point.y,
    kind,
    smooth,
    name: null,
    tt_index: 'Unnumbered',
  }
}

/** The contour-level identity a rebuilt contour inherits from its source. */
export interface ContourIdentity {
  name: string | null
  id: string | null
}

/** Nothing inherited: a contour that did not exist before. */
export const NO_IDENTITY: ContourIdentity = { name: null, id: null }

/**
 * Turns explicit segments back into the UFO point list a command takes.
 *
 * `smooth` marks the on-curve points that should keep their handles tangent;
 * it is indexed by segment, naming the point each segment ends at. `identity`
 * is the source contour's name and persistent id, which a cut or an extend
 * carries onto the piece that stands for the original.
 */
export function pathToContour(
  path: GeometricPath,
  smooth: (index: number) => boolean = () => false,
  identity: ContourIdentity = NO_IDENTITY,
): Contour {
  const points: Point[] = []
  const count = path.segments.length
  if (count === 0)
    return {
      closed: path.closed,
      points: [storedPoint(path.start, path.closed ? 'Line' : 'Move')],
      name: identity.name,
      id: identity.id,
    }
  if (path.closed) {
    const last = path.segments[count - 1] as PathSegment
    points.push(storedPoint(path.start, kindOf(last), smooth(count - 1)))
  } else {
    points.push(storedPoint(path.start, 'Move', smooth(-1)))
  }
  path.segments.forEach((segment, index) => {
    for (const control of segment.controls)
      points.push(storedPoint(control, 'OffCurve'))
    if (path.closed && index === count - 1) return
    points.push(storedPoint(segment.to, kindOf(segment), smooth(index)))
  })
  return { closed: path.closed, points, name: identity.name, id: identity.id }
}

/** The point of a segment at parameter `t`. */
export function segmentPointAt(segment: PathSegment, t: number): Point2 {
  const { from, to, controls } = segment
  if (segment.kind === 'line') return lerp(from, to, t)
  if (segment.kind === 'quad') {
    const c = controls[0] ?? from
    return lerp(lerp(from, c, t), lerp(c, to, t), t)
  }
  const c1 = controls[0] ?? from
  const c2 = controls[1] ?? to
  const a = lerp(from, c1, t)
  const b = lerp(c1, c2, t)
  const c = lerp(c2, to, t)
  return lerp(lerp(a, b, t), lerp(b, c, t), t)
}

/** Splits a segment at `t` into the part before it and the part after. */
export function splitSegment(
  segment: PathSegment,
  t: number,
): [PathSegment, PathSegment] {
  const { from, to, controls } = segment
  if (segment.kind === 'line') {
    const mid = lerp(from, to, t)
    return [
      { kind: 'line', from, to: mid, controls: [] },
      { kind: 'line', from: mid, to, controls: [] },
    ]
  }
  if (segment.kind === 'quad') {
    const c = controls[0] ?? from
    const a = lerp(from, c, t)
    const b = lerp(c, to, t)
    const mid = lerp(a, b, t)
    return [
      { kind: 'quad', from, to: mid, controls: [a] },
      { kind: 'quad', from: mid, to, controls: [b] },
    ]
  }
  const c1 = controls[0] ?? from
  const c2 = controls[1] ?? to
  const a = lerp(from, c1, t)
  const b = lerp(c1, c2, t)
  const c = lerp(c2, to, t)
  const d = lerp(a, b, t)
  const e = lerp(b, c, t)
  const mid = lerp(d, e, t)
  return [
    { kind: 'cubic', from, to: mid, controls: [a, d] },
    { kind: 'cubic', from: mid, to, controls: [e, c] },
  ]
}

/** The portion of one segment between two parameters. */
function segmentPart(
  segment: PathSegment,
  t0: number,
  t1: number,
): PathSegment {
  let part = segment
  if (t1 < 1 - 1e-9) part = splitSegment(part, t1)[0]
  if (t0 > 1e-9) {
    const scaled = t1 < 1 - 1e-9 ? t0 / t1 : t0
    part = splitSegment(part, Math.min(1 - 1e-9, scaled))[1]
  }
  return part
}

/**
 * The segments between two path parameters, walking forward and wrapping
 * around a closed path when the end lies before the start.
 */
export function extractSegments(
  path: GeometricPath,
  from: number,
  to: number,
): PathSegment[] {
  const count = path.segments.length
  if (count === 0) return []
  const wraps = to <= from + 1e-9
  // `to === count` names the very end of an open path, which is the last
  // segment taken whole rather than a zero-length slice of a segment past it.
  const beyond = to >= count
  const endIndex = beyond
    ? count - 1
    : Math.min(count - 1, Math.max(0, Math.floor(to)))
  const endT = beyond ? 1 : to - Math.floor(to)
  let index = Math.min(count - 1, Math.max(0, Math.floor(from)))
  let start = from - Math.floor(from)
  const out: PathSegment[] = []
  for (let step = 0; step <= count; step += 1) {
    const segment = path.segments[index]
    if (!segment) break
    const last = index === endIndex && (!wraps || step > 0)
    const stop = last ? endT : 1
    if (stop > start + 1e-9) out.push(segmentPart(segment, start, stop))
    if (last) break
    index = (index + 1) % count
    start = 0
  }
  return out
}

/** Builds an open path from a run of segments. */
export function pathOfSegments(
  segments: readonly PathSegment[],
  closed: boolean,
): GeometricPath | null {
  const first = segments[0]
  if (!first) return null
  return { closed, start: at(first.from), segments: [...segments] }
}

/** Signed cross product of `b - a` with `c - a`. */
function side(a: Point2, direction: Point2, point: Point2): number {
  return (point.x - a.x) * direction.y - (point.y - a.y) * direction.x
}

/**
 * Where a straight line crosses a path.
 *
 * Curved segments are sampled `LINE_HIT_SAMPLES` times to bracket each sign
 * change and the crossing is then refined on the exact Bézier by bisection,
 * so the reported point lies on the curve rather than on its polygon.
 */
export function lineHits(
  path: GeometricPath,
  a: Point2,
  b: Point2,
): PathLineHit[] {
  const direction = { x: b.x - a.x, y: b.y - a.y }
  const length2 = direction.x * direction.x + direction.y * direction.y
  if (length2 < 1e-12) return []
  const hits: PathLineHit[] = []
  path.segments.forEach((segment, index) => {
    const samples = segment.kind === 'line' ? 1 : LINE_HIT_SAMPLES
    let previousT = 0
    let previous = side(a, direction, segment.from)
    for (let step = 1; step <= samples; step += 1) {
      const t = step / samples
      const point = segmentPointAt(segment, t)
      const current = side(a, direction, point)
      if (previous === 0 || previous * current < 0) {
        const root =
          previous === 0
            ? previousT
            : refine(segment, a, direction, previousT, t)
        const found = segmentPointAt(segment, root)
        const s =
          ((found.x - a.x) * direction.x + (found.y - a.y) * direction.y) /
          length2
        // The parameter is capped just short of 1 so a crossing at a node is
        // reported on the segment it starts, never twice.
        if (s >= 0 && s <= 1 && root < 1 - 1e-9)
          hits.push({ u: index + root, s, point: found })
      }
      previousT = t
      previous = current
    }
  })
  return hits.sort((x, y) => x.u - y.u)
}

/** Bisects a bracketed sign change on the exact curve. */
function refine(
  segment: PathSegment,
  a: Point2,
  direction: Point2,
  low: number,
  high: number,
): number {
  let lo = low
  let hi = high
  let loValue = side(a, direction, segmentPointAt(segment, lo))
  for (let pass = 0; pass < 40; pass += 1) {
    const mid = (lo + hi) / 2
    const value = side(a, direction, segmentPointAt(segment, mid))
    if (value === 0) return mid
    if (loValue * value < 0) hi = mid
    else {
      lo = mid
      loValue = value
    }
  }
  return (lo + hi) / 2
}

/**
 * Twice the signed area of a path, positive when it winds counter-clockwise
 * in font space (y-up). Curves are approximated by their control polygon,
 * which has the same sign as the exact integral for any sane outline.
 */
export function signedArea(path: GeometricPath): number {
  const points: Point2[] = [path.start]
  for (const segment of path.segments) {
    points.push(...segment.controls, segment.to)
  }
  let total = 0
  for (let index = 0; index < points.length; index += 1) {
    const a = points[index] as Point2
    const b = points[(index + 1) % points.length] as Point2
    total += a.x * b.y - b.x * a.y
  }
  return total
}

/** The same path traced the other way round. */
export function reversePath(path: GeometricPath): GeometricPath {
  const segments = [...path.segments].reverse().map<PathSegment>((segment) => ({
    kind: segment.kind,
    from: segment.to,
    to: segment.from,
    controls: [...segment.controls].reverse(),
  }))
  const first = segments[0]
  return {
    closed: path.closed,
    start: first ? first.from : path.start,
    segments,
  }
}

/** The path wound the way `curves` asks for: cubic anti-clockwise, quadratic clockwise. */
export function withWinding(
  path: GeometricPath,
  curves: LayerCurves,
): GeometricPath {
  const area = signedArea(path)
  const wantPositive = curves === 'cubic'
  if (area === 0 || area > 0 === wantPositive) return path
  return reversePath(path)
}

/** Bounding box of a box drag, normalised so the extents are non-negative. */
export function normaliseBox(a: Point2, b: Point2): FontBox {
  return {
    x0: Math.min(a.x, b.x),
    y0: Math.min(a.y, b.y),
    x1: Math.max(a.x, b.x),
    y1: Math.max(a.y, b.y),
  }
}

/**
 * The curve flavour a layer is drawn in.
 *
 * `LayerOutline` does not carry the font's declared curve kind, so it is read
 * off the outlines themselves: a quadratic on-curve point means TrueType, an
 * explicit cubic means PostScript. A layer with no curves at all — an empty
 * glyph, a rectangle — says nothing, and then the document's own format
 * decides, defaulting to quadratic for a TrueType source.
 */
export function inferCurves(
  contours: readonly Contour[],
  format: string,
): LayerCurves {
  for (const contour of contours)
    for (const point of contour.points) {
      if (point.kind === 'QCurve') return 'quadratic'
      if (point.kind === 'Curve') return 'cubic'
    }
  return /truetype|ttf|ttc|woff/i.test(format) ? 'quadratic' : 'cubic'
}
