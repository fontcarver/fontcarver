import type { VariationShape, VariationShapeStatus } from '../../api'

/** An ordered SVG point used only for source-overlay inspection. */
export interface SourcePoint {
  x: number
  y: number
  kind: 'on' | 'control'
}

/** Parsed SVG topology, including off-curve points and contour closure. */
export interface SourcePoints {
  contours: { points: SourcePoint[]; closed: boolean }[]
  pointCount: number
}

/**
 * Read the absolute M/L/Q/C/Z subset emitted by kurbo, with explicit work limits.
 * Unsupported or malformed input keeps its outline but receives no point links.
 */
export function parseSourcePoints(path: string): SourcePoints | null {
  if (path.length > 100_000) return null
  const token = /[A-Za-z]|[-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?/y
  const separator = /[\s,]+/y
  const tokens: (string | number)[] = []
  let position = 0
  while (position < path.length) {
    separator.lastIndex = position
    const space = separator.exec(path)
    if (space) position = separator.lastIndex
    if (position === path.length) break
    token.lastIndex = position
    const match = token.exec(path)
    if (!match) return null
    const value = match[0]
    if (/^[A-Za-z]$/.test(value)) tokens.push(value)
    else {
      const number = Number(value)
      if (!Number.isFinite(number)) return null
      tokens.push(number)
    }
    position = token.lastIndex
  }
  const result: SourcePoints = { contours: [], pointCount: 0 }
  let index = 0
  let command = ''
  let contour: SourcePoints['contours'][number] | undefined
  while (index < tokens.length) {
    if (typeof tokens[index] === 'string') command = tokens[index++] as string
    if (command === 'Z') {
      if (!contour || contour.closed) return null
      contour.closed = true
      command = ''
      continue
    }
    const pairs =
      command === 'M' || command === 'L'
        ? 1
        : command === 'Q'
          ? 2
          : command === 'C'
            ? 3
            : 0
    if (!pairs || index + pairs * 2 > tokens.length) return null
    if (command === 'M') {
      contour = { points: [], closed: false }
      result.contours.push(contour)
    }
    if (!contour || contour.closed) return null
    for (let pair = 0; pair < pairs; pair++) {
      const x = tokens[index++]
      const y = tokens[index++]
      if (typeof x !== 'number' || typeof y !== 'number') return null
      if (++result.pointCount > 2048) return null
      contour.points.push({ x, y, kind: pair === pairs - 1 ? 'on' : 'control' })
    }
    if (command === 'M') command = 'L'
  }
  return result
}

/** Point structure is visual correspondence, not a replacement for API diagnostics. */
export function samePointStructure(
  left: SourcePoints,
  right: SourcePoints,
): boolean {
  return (
    left.contours.length === right.contours.length &&
    left.contours.every((contour, i) => {
      const other = right.contours[i]!
      return (
        contour.closed === other.closed &&
        contour.points.length === other.points.length &&
        contour.points.every((point, j) => point.kind === other.points[j]!.kind)
      )
    })
  )
}

/** Number of distinct shape colours the view cycles through. */
export const SHAPE_COLOURS = 6

/** Colour class index of one shape position, consistent across every source. */
export function shapeColour(position: number): number {
  return position % SHAPE_COLOURS
}

/** The status the kernel reported for one shape position. */
export function statusOf(
  status: readonly VariationShapeStatus[],
  position: number,
): VariationShapeStatus['status'] {
  return status[position]?.status ?? 'compatible'
}

/** One segment of a shape, in stored-point order. */
export interface ShapeSegment {
  from: { x: number; y: number }
  to: { x: number; y: number }
  status: string
}

/**
 * Straight chords between consecutive stored points of one shape.
 *
 * The chords are what carries the green/yellow/red status: they follow the
 * point order the statuses are indexed by, which a rendered curve does not.
 */
export function shapeSegments(
  shape: VariationShape,
  status: VariationShapeStatus | undefined,
): ShapeSegment[] {
  const points = shape.points
  if (points.length < 2) return []
  const limit = shape.closed ? points.length : points.length - 1
  const segments: ShapeSegment[] = []
  for (let index = 0; index < limit; index += 1) {
    const from = points[index]!
    const to = points[(index + 1) % points.length]!
    segments.push({
      from: { x: from[0], y: from[1] },
      to: { x: to[0], y: to[1] },
      status: status?.segments[index] ?? 'compatible',
    })
  }
  return segments
}

/** Move the shape at `from` to `to`, as a permutation the kernel accepts. */
export function swapOrder(count: number, from: number, to: number): number[] {
  const order = Array.from({ length: count }, (_, index) => index)
  if (from === to || from < 0 || to < 0 || from >= count || to >= count)
    return order
  order[from] = to
  order[to] = from
  return order
}

/** Move the shape at `from` one position earlier or later. */
export function nudgeOrder(
  count: number,
  from: number,
  delta: number,
): number[] {
  return swapOrder(count, from, from + delta)
}
