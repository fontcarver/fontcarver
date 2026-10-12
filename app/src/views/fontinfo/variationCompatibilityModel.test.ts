import { describe, expect, it } from 'vitest'
import {
  nudgeOrder,
  parseSourcePoints,
  samePointStructure,
  shapeColour,
  shapeSegments,
  swapOrder,
} from './variationCompatibilityModel'

describe('variation SVG point inspection', () => {
  it('preserves contour, point and control-point ordering', () => {
    const parsed = parseSourcePoints(
      'M0 0L10 0Q20 0 20 10C20 20 10 20 0 10ZM30 40L50 60',
    )!
    expect(parsed.contours).toHaveLength(2)
    expect(parsed.contours[0]!.points.map((point) => point.kind)).toEqual([
      'on',
      'on',
      'control',
      'on',
      'control',
      'control',
      'on',
    ])
    expect(parsed.contours[0]!.closed).toBe(true)
    expect(parsed.contours[1]!.closed).toBe(false)
    expect(parsed.contours[1]!.points[0]).toEqual({ x: 30, y: 40, kind: 'on' })
  })

  it('reads compact SVG number syntax and repeated command coordinates', () => {
    const parsed = parseSourcePoints('M1e2,-2.5 3.5.5L-4-5 6 7Z')!
    expect(parsed.contours[0]!.points).toEqual([
      { x: 100, y: -2.5, kind: 'on' },
      { x: 3.5, y: 0.5, kind: 'on' },
      { x: -4, y: -5, kind: 'on' },
      { x: 6, y: 7, kind: 'on' },
    ])
  })

  it('compares topology independently of point coordinates', () => {
    const left = parseSourcePoints('M0 0L10 10Z')!
    expect(samePointStructure(left, parseSourcePoints('M20 30L40 80Z')!)).toBe(
      true,
    )
    expect(samePointStructure(left, parseSourcePoints('M0 0L10 10')!)).toBe(
      false,
    )
    expect(
      samePointStructure(left, parseSourcePoints('M0 0Q5 5 10 10Z')!),
    ).toBe(false)
    expect(samePointStructure(left, parseSourcePoints('M0 0Z M10 10Z')!)).toBe(
      false,
    )
  })

  it.each([
    'M0',
    'M0 0L2',
    'M0 0A1 1 0 0 0 2 2',
    'm0 0l1 1',
    'L1 1',
    'M0 0Z1 2',
    'M1e999 0',
    'M0 0<script>',
  ])('declines malformed or unsupported paths: %s', (path) => {
    expect(parseSourcePoints(path)).toBeNull()
  })

  it('bounds inspection work and accepts an empty outline', () => {
    expect(parseSourcePoints('M0 0' + 'L1 1'.repeat(2048))).toBeNull()
    expect(parseSourcePoints(' '.repeat(100001))).toBeNull()
    expect(parseSourcePoints('')).toEqual({ contours: [], pointCount: 0 })
  })
})

describe('shape order helpers', () => {
  it('gives the same colour to the same shape position in every source', () => {
    expect(shapeColour(0)).toBe(0)
    expect(shapeColour(6)).toBe(0)
    expect(shapeColour(7)).toBe(1)
  })

  it('chords one closed shape in stored-point order with its statuses', () => {
    const shape = {
      index: 0,
      closed: true,
      path: '',
      start: [0, 0] as [number, number],
      center: [5, 5] as [number, number],
      points: [
        [0, 0],
        [0, 10],
        [10, 10],
      ] as [number, number][],
      on_curve: [true, true, true],
    }
    const segments = shapeSegments(shape, {
      index: 0,
      status: 'angle',
      segments: ['compatible', 'angle', 'compatible'],
      max_angle: 30,
    })
    expect(segments).toHaveLength(3)
    expect(segments[1]).toEqual({
      from: { x: 0, y: 10 },
      to: { x: 10, y: 10 },
      status: 'angle',
    })
    // The closing chord returns to the first point.
    expect(segments[2]?.to).toEqual({ x: 0, y: 0 })
    // An open shape has one chord fewer.
    expect(shapeSegments({ ...shape, closed: false }, undefined)).toHaveLength(
      2,
    )
    // A missing status reads as compatible rather than throwing.
    expect(shapeSegments(shape, undefined)[0]?.status).toBe('compatible')
  })

  it('builds a permutation that swaps exactly two positions', () => {
    expect(swapOrder(4, 0, 2)).toEqual([2, 1, 0, 3])
    expect(swapOrder(4, 1, 1)).toEqual([0, 1, 2, 3])
    for (const [from, to] of [
      [-1, 0],
      [0, 9],
      [9, 0],
    ])
      expect(swapOrder(3, from!, to!)).toEqual([0, 1, 2])
  })

  it('nudges one shape by a single position and stops at the ends', () => {
    expect(nudgeOrder(3, 1, -1)).toEqual([1, 0, 2])
    expect(nudgeOrder(3, 1, 1)).toEqual([0, 2, 1])
    expect(nudgeOrder(3, 0, -1)).toEqual([0, 1, 2])
    expect(nudgeOrder(3, 2, 1)).toEqual([0, 1, 2])
  })
})
