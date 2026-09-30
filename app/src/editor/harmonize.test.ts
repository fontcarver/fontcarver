import { describe, expect, it } from 'vitest'

import type { Contour } from '../api'
import { makeContour, makePoint } from '../test/fontFixtures'

import { harmonizeNodes, maintainSuperSmoothContours } from './harmonize'

function joined(): Contour {
  return makeContour(
    [
      makePoint(-100, 100, 'Move'),
      makePoint(-80, 40, 'OffCurve'),
      makePoint(-30, 0, 'OffCurve'),
      makePoint(0, 0, 'Curve'),
      makePoint(50, 0, 'OffCurve'),
      makePoint(70, 10, 'OffCurve'),
      makePoint(100, 100, 'Curve'),
    ],
    false,
  )
}

/** Signed endpoint curvatures, independently from the projection formula. */
function curvatures(contour: Contour, node: number): [number, number] {
  const at = (offset: number) =>
    contour.points[
      (node + offset + contour.points.length) % contour.points.length
    ]
  const p = at(0)
  const incoming = { x: p.x - at(-1).x, y: p.y - at(-1).y }
  const outgoing = { x: at(1).x - p.x, y: at(1).y - p.y }
  const curvature = (
    tangent: { x: number; y: number },
    outer: { x: number; y: number },
  ) =>
    (2 * (tangent.x * (outer.y - p.y) - tangent.y * (outer.x - p.x))) /
    (3 * Math.hypot(tangent.x, tangent.y) ** 3)
  return [curvature(incoming, at(-2)), curvature(outgoing, at(2))]
}

describe('harmonizeNodes', () => {
  it('matches the cubic endpoint curvature while preserving nodes and handle budget', () => {
    const source = joined()
    source.points[3] = { ...source.points[3], name: 'join', super_smooth: true }
    const before = structuredClone(source)
    const result = harmonizeNodes(source, [3, 3])!

    expect(source).toEqual(before)
    expect(result.points[2].x).toBeCloseTo(-160 / 3, 12)
    expect(result.points[4].x).toBeCloseTo(80 / 3, 12)
    expect(curvatures(result, 3)[0]).toBeCloseTo(curvatures(result, 3)[1], 12)
    expect(result.points[3]).toEqual({ ...source.points[3], smooth: true })
    for (const index of [0, 1, 5, 6])
      expect(result.points[index]).toEqual(source.points[index])
  })

  it('aligns disagreeing tangents before balancing the handles', () => {
    const source = joined()
    Object.assign(source.points[2], { x: -30, y: -40 })
    Object.assign(source.points[4], { x: 40, y: 30 })
    Object.assign(source.points[5], { x: 70, y: 100 })
    const result = harmonizeNodes(source, [3])!

    expect(result.points[2].x).toBeCloseTo(result.points[2].y, 12)
    expect(result.points[4].x).toBeCloseTo(result.points[4].y, 12)
    expect(
      Math.hypot(result.points[2].x, result.points[2].y) +
        Math.hypot(result.points[4].x, result.points[4].y),
    ).toBeCloseTo(100, 12)
    expect(curvatures(result, 3)[0]).toBeCloseTo(curvatures(result, 3)[1], 12)
  })

  it('resolves a cubic join across the closed contour seam', () => {
    const source = joined()
    source.closed = true
    source.points = [...source.points.slice(3), ...source.points.slice(1, 3)]
    const result = harmonizeNodes(source, [0])!

    expect(result.points[5].x).toBeCloseTo(-160 / 3, 12)
    expect(result.points[1].x).toBeCloseTo(80 / 3, 12)
    expect(curvatures(result, 0)[0]).toBeCloseTo(curvatures(result, 0)[1], 12)
  })

  it('solves adjacent selected joins together regardless of selection order', () => {
    const source = joined()
    Object.assign(source.points[5], { x: 100, y: 50 })
    source.points.push(
      makePoint(100, 130, 'OffCurve'),
      makePoint(60, 180, 'OffCurve'),
      makePoint(0, 200, 'Curve'),
    )
    const result = harmonizeNodes(source, [6, 3])!

    expect(result).toEqual(harmonizeNodes(source, [3, 6]))
    for (const node of [3, 6]) {
      const [incoming, outgoing] = curvatures(result, node)
      expect(incoming).toBeCloseTo(outgoing, 12)
      expect(result.points[node]).toEqual({
        ...source.points[node],
        smooth: true,
      })
    }
  })

  it('preserves straight handle lengths and treats empty selections as a no-op', () => {
    const source = joined()
    source.points[1].y = 0
    source.points[5].y = 0
    const result = harmonizeNodes(source, [3])!
    expect(result.points[2]).toEqual(source.points[2])
    expect(result.points[4]).toEqual(source.points[4])
    expect(harmonizeNodes(source, [])).toBe(source)
  })

  it('keeps accepted coordinates exactly stable on repeated projections', () => {
    const source = joined()
    Object.assign(source.points[2], { x: -30, y: -40 })
    Object.assign(source.points[4], { x: 40, y: 30 })
    Object.assign(source.points[5], { x: 70, y: 100 })
    const first = harmonizeNodes(source, [3])!
    let repeated = first
    for (let pass = 0; pass < 20; pass += 1)
      repeated = harmonizeNodes(repeated, [3])!
    expect(repeated).toEqual(first)
  })

  it.each([
    'invalid',
    'off-curve',
    'open-end',
    'quadratic',
    'zero-handle',
    'cusp',
    'inflection',
    'one-straight',
    'non-finite',
  ])('rejects %s atomically', (scenario) => {
    const source = joined()
    let nodes = [3]
    if (scenario === 'invalid') nodes = [3, 99]
    if (scenario === 'off-curve') nodes = [2]
    if (scenario === 'open-end') nodes = [6]
    if (scenario === 'quadratic') source.points[6].kind = 'QCurve'
    if (scenario === 'zero-handle') source.points[2].x = 0
    if (scenario === 'cusp') source.points[4].x = -50
    if (scenario === 'inflection') source.points[5].y = -10
    if (scenario === 'one-straight') source.points[5].y = 0
    if (scenario === 'non-finite') source.points[1].x = Infinity
    const before = structuredClone(source)

    expect(harmonizeNodes(source, nodes)).toBeNull()
    expect(source).toEqual(before)
  })
})

describe('maintainSuperSmoothContours', () => {
  it('only projects flagged nodes and retains ordinary contours by identity', () => {
    const retained = joined()
    retained.points[3].super_smooth = true
    const ordinary = joined()
    const result = maintainSuperSmoothContours([retained, ordinary])!

    expect(result[0].points[3].super_smooth).toBe(true)
    expect(curvatures(result[0], 3)[0]).toBeCloseTo(
      curvatures(result[0], 3)[1],
      12,
    )
    expect(result[1]).toBe(ordinary)
  })

  it('fails the whole projection when any persistent join is incompatible', () => {
    const first = joined()
    first.points[3].super_smooth = true
    const second = structuredClone(first)
    second.points[5].y = -10
    const source = [first, second]
    const before = structuredClone(source)

    expect(maintainSuperSmoothContours(source)).toBeNull()
    expect(source).toEqual(before)
  })
})
