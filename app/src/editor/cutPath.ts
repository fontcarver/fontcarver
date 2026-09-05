/**
 * Cutting contours with a straight line.
 *
 * The whole computation is local: the intersections are solved on the exact
 * Béziers, the contours are split there and the pieces are closed again by
 * joining consecutive cut points with a straight segment — the behaviour
 * Glyphs has, and the reason a knife stroke across a bowl leaves two closed
 * shapes rather than two open ones.
 *
 * Open contours simply split at every crossing. A closed contour is cut only
 * when it is crossed an even number of times, because an odd count means the
 * line grazed it and there is no honest pairing of the cut points.
 */

import type { Contour } from '../api'
import type { Point2 } from '../canvas/Viewport'

import {
  contourToPath,
  extractSegments,
  lineHits,
  pathToContour,
  type GeometricPath,
  type PathLineHit,
  type PathSegment,
} from './pathGeometry'

/** What one knife stroke does to a layer. */
export interface CutResult {
  /** Contour indexes the cut replaces, ascending. */
  removed: number[]
  /** The pieces that take their place, in cut order. */
  added: Contour[]
}

/** A straight segment between two points. */
function chord(from: Point2, to: Point2): PathSegment {
  return { kind: 'line', from, to, controls: [] }
}

/** The open pieces an open contour falls into. */
function cutOpen(path: GeometricPath, hits: PathLineHit[]): Contour[] {
  const bounds = [0, ...hits.map((hit) => hit.u), path.segments.length]
  const pieces: Contour[] = []
  for (let index = 0; index + 1 < bounds.length; index += 1) {
    const segments = extractSegments(
      path,
      bounds[index] as number,
      bounds[index + 1] as number,
    )
    const first = segments[0]
    if (!first) continue
    pieces.push(pathToContour({ closed: false, start: first.from, segments }))
  }
  return pieces
}

/**
 * The closed pieces a closed contour falls into.
 *
 * Sorting the crossings along the *cut line* pairs them the way the Jordan
 * curve theorem does — the line enters the shape at one and leaves at the next
 * — so each pair is a chord that lies inside the contour. Following the
 * contour from a cut point to the next, then jumping across that point's
 * chord, walks exactly one closed piece.
 */
function cutClosed(path: GeometricPath, hits: PathLineHit[]): Contour[] {
  const count = hits.length
  if (count < 2 || count % 2 !== 0) return []
  const byLine = hits
    .map((hit, index) => ({ hit, index }))
    .sort((a, b) => a.hit.s - b.hit.s)
  const partner = new Array<number>(count).fill(0)
  for (let pair = 0; pair + 1 < count; pair += 2) {
    const a = byLine[pair]?.index ?? 0
    const b = byLine[pair + 1]?.index ?? 0
    partner[a] = b
    partner[b] = a
  }

  const visited = new Set<number>()
  const pieces: Contour[] = []
  for (let start = 0; start < count; start += 1) {
    if (visited.has(start)) continue
    const segments: PathSegment[] = []
    let arc = start
    for (let guard = 0; guard <= count; guard += 1) {
      visited.add(arc)
      const from = hits[arc] as PathLineHit
      const to = hits[(arc + 1) % count] as PathLineHit
      segments.push(...extractSegments(path, from.u, to.u))
      const jump = partner[(arc + 1) % count] as number
      const landing = hits[jump] as PathLineHit
      segments.push(chord(to.point, landing.point))
      if (jump === start) break
      arc = jump
      if (visited.has(arc)) break
    }
    const first = segments[0]
    if (!first) continue
    pieces.push(pathToContour({ closed: true, start: first.from, segments }))
  }
  return pieces
}

/** What a knife stroke from `a` to `b` does to the layer's contours. */
export function cutContours(
  contours: readonly Contour[],
  a: Point2,
  b: Point2,
): CutResult {
  const removed: number[] = []
  const added: Contour[] = []
  contours.forEach((contour, index) => {
    const path = contourToPath(contour)
    const hits = lineHits(path, a, b)
    if (hits.length === 0) return
    const pieces = path.closed ? cutClosed(path, hits) : cutOpen(path, hits)
    if (pieces.length < 2) return
    removed.push(index)
    added.push(...pieces)
  })
  return { removed, added }
}

/** The points a knife stroke would cut at, for the overlay. */
export function cutPoints(
  contours: readonly Contour[],
  a: Point2,
  b: Point2,
): Point2[] {
  const points: Point2[] = []
  for (const contour of contours)
    for (const hit of lineHits(contourToPath(contour), a, b))
      points.push(hit.point)
  return points
}
