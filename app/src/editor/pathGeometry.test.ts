import { describe, expect, it } from 'vitest'

import { makeContour, makePoint } from '../test/fontFixtures'

import {
  contourToPath,
  extractSegments,
  lineHits,
  normaliseBox,
  pathOfSegments,
  pathToContour,
  reversePath,
  segmentPointAt,
  signedArea,
  splitSegment,
  withWinding,
} from './pathGeometry'

/** A 200 × 200 square, counter-clockwise from the origin. */
const square = makeContour([
  makePoint(0, 0),
  makePoint(200, 0, 'Line'),
  makePoint(200, 200, 'Line'),
  makePoint(0, 200, 'Line'),
])

describe('contourToPath', () => {
  it('resolves a closed square into four line segments', () => {
    const path = contourToPath(square)
    expect(path.closed).toBe(true)
    expect(path.start).toEqual({ x: 0, y: 0 })
    expect(path.segments.map((segment) => segment.kind)).toEqual([
      'line',
      'line',
      'line',
      'line',
    ])
    expect(path.segments[3]?.to).toEqual({ x: 0, y: 0 })
  })

  it('round-trips a contour through the point list', () => {
    const contour = pathToContour(contourToPath(square))
    expect(contour.closed).toBe(true)
    expect(
      contour.points.map((point) => [point.x, point.y, point.kind]),
    ).toEqual([
      [0, 0, 'Line'],
      [200, 0, 'Line'],
      [200, 200, 'Line'],
      [0, 200, 'Line'],
    ])
  })

  it('round-trips an open cubic path', () => {
    const open = makeContour(
      [
        makePoint(0, 0, 'Move'),
        makePoint(20, 100, 'OffCurve'),
        makePoint(80, 100, 'OffCurve'),
        makePoint(100, 0, 'Curve'),
      ],
      false,
    )
    const contour = pathToContour(contourToPath(open))
    expect(contour.closed).toBe(false)
    expect(contour.points.map((point) => point.kind)).toEqual([
      'Move',
      'OffCurve',
      'OffCurve',
      'Curve',
    ])
  })
})

describe('splitSegment', () => {
  it('splits a cubic so both halves trace the original curve', () => {
    const segment = {
      kind: 'cubic' as const,
      from: { x: 0, y: 0 },
      to: { x: 300, y: 0 },
      controls: [
        { x: 100, y: 200 },
        { x: 200, y: 200 },
      ],
    }
    const [first, second] = splitSegment(segment, 0.25)
    expect(first.to).toEqual(second.from)
    // A point three quarters along the second half is at t = 0.8125 overall.
    const direct = segmentPointAt(segment, 0.8125)
    const halved = segmentPointAt(second, 0.75)
    expect(halved.x).toBeCloseTo(direct.x, 8)
    expect(halved.y).toBeCloseTo(direct.y, 8)
  })
})

describe('lineHits', () => {
  it('finds the two crossings of a vertical cut through a square', () => {
    const hits = lineHits(
      contourToPath(square),
      { x: 100, y: -50 },
      { x: 100, y: 250 },
    )
    expect(hits).toHaveLength(2)
    expect(hits.map((hit) => hit.point.x)).toEqual([100, 100])
    expect(hits.map((hit) => Math.round(hit.point.y))).toEqual([0, 200])
  })

  it('lands on the curve itself, not on its sampled polygon', () => {
    const arch = makeContour(
      [
        makePoint(0, 0, 'Move'),
        makePoint(0, 200, 'OffCurve'),
        makePoint(200, 200, 'OffCurve'),
        makePoint(200, 0, 'Curve'),
      ],
      false,
    )
    const hits = lineHits(
      contourToPath(arch),
      { x: 100, y: 0 },
      { x: 100, y: 300 },
    )
    expect(hits).toHaveLength(1)
    // The apex of that cubic sits at 3/4 of the control height.
    expect(hits[0]?.point.y).toBeCloseTo(150, 6)
  })

  it('ignores crossings beyond the ends of the drawn line', () => {
    const hits = lineHits(
      contourToPath(square),
      { x: 100, y: -50 },
      { x: 100, y: 100 },
    )
    expect(hits).toHaveLength(1)
    expect(Math.round(hits[0]?.point.y ?? -1)).toBe(0)
  })
})

describe('extractSegments', () => {
  it('takes the arc between two parameters', () => {
    const path = contourToPath(square)
    const part = extractSegments(path, 0.5, 2.5)
    expect(part).toHaveLength(3)
    expect(part[0]?.from).toEqual({ x: 100, y: 0 })
    expect(part[2]?.to).toEqual({ x: 100, y: 200 })
  })

  it('wraps around a closed path when the end lies before the start', () => {
    const path = contourToPath(square)
    const part = extractSegments(path, 2.5, 0.5)
    expect(part[0]?.from).toEqual({ x: 100, y: 200 })
    expect(part[part.length - 1]?.to).toEqual({ x: 100, y: 0 })
  })
})

describe('winding', () => {
  it('reports the counter-clockwise square as positive', () => {
    expect(signedArea(contourToPath(square))).toBeGreaterThan(0)
  })

  it('reverses a path so a quadratic layer gets a clockwise outer contour', () => {
    const path = contourToPath(square)
    expect(signedArea(withWinding(path, 'cubic'))).toBeGreaterThan(0)
    expect(signedArea(withWinding(path, 'quadratic'))).toBeLessThan(0)
    expect(signedArea(reversePath(path))).toBeLessThan(0)
  })
})

describe('helpers', () => {
  it('normalises a box drag', () => {
    expect(normaliseBox({ x: 10, y: 40 }, { x: -5, y: 0 })).toEqual({
      x0: -5,
      y0: 0,
      x1: 10,
      y1: 40,
    })
  })

  it('rejects an empty segment run', () => {
    expect(pathOfSegments([], true)).toBeNull()
  })
})
