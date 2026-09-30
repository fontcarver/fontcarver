import { describe, expect, it } from 'vitest'
import { parseSourcePoints, samePointStructure } from './variationCompatibility'

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
