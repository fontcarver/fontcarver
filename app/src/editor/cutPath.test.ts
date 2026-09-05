import { describe, expect, it } from 'vitest'

import { makeContour, makePoint } from '../test/fontFixtures'

import { cutContours, cutPoints } from './cutPath'

const square = makeContour([
  makePoint(0, 0),
  makePoint(200, 0, 'Line'),
  makePoint(200, 200, 'Line'),
  makePoint(0, 200, 'Line'),
])

describe('cutContours', () => {
  it('cuts a rectangle into two closed four-point contours', () => {
    const result = cutContours([square], { x: 100, y: -50 }, { x: 100, y: 250 })
    expect(result.removed).toEqual([0])
    expect(result.added).toHaveLength(2)
    for (const piece of result.added) {
      expect(piece.closed).toBe(true)
      expect(piece.points).toHaveLength(4)
    }
    // Between them the pieces hold both original corners of each side.
    const xs = result.added.flatMap((piece) =>
      piece.points.map((point) => point.x),
    )
    expect(xs.filter((x) => x === 0)).toHaveLength(2)
    expect(xs.filter((x) => x === 200)).toHaveLength(2)
    expect(xs.filter((x) => x === 100)).toHaveLength(4)
  })

  it('splits an open path cut once into two open paths', () => {
    const open = makeContour(
      [
        makePoint(0, 0, 'Move'),
        makePoint(100, 0, 'Line'),
        makePoint(200, 0, 'Line'),
      ],
      false,
    )
    const result = cutContours([open], { x: 50, y: -50 }, { x: 50, y: 50 })
    expect(result.removed).toEqual([0])
    expect(result.added).toHaveLength(2)
    expect(result.added.every((piece) => !piece.closed)).toBe(true)
    expect(result.added[0]?.points.map((point) => point.x)).toEqual([0, 50])
    expect(result.added[1]?.points.map((point) => point.x)).toEqual([
      50, 100, 200,
    ])
  })

  it('leaves a contour the stroke never reaches alone', () => {
    const result = cutContours([square], { x: 400, y: 0 }, { x: 400, y: 200 })
    expect(result).toEqual({ removed: [], added: [] })
  })

  it('leaves a closed contour it only grazes alone', () => {
    // A stroke that ends inside the square crosses it once, which cannot be
    // paired into chords.
    const result = cutContours([square], { x: 100, y: -50 }, { x: 100, y: 100 })
    expect(result.added).toHaveLength(0)
  })

  it('cuts a curved contour on the curve, not on its polygon', () => {
    const bowl = makeContour([
      makePoint(0, 0),
      makePoint(0, 200, 'OffCurve'),
      makePoint(200, 200, 'OffCurve'),
      makePoint(200, 0, 'Curve'),
    ])
    const result = cutContours([bowl], { x: 100, y: -10 }, { x: 100, y: 300 })
    expect(result.added).toHaveLength(2)
    const cutY = result.added
      .flatMap((piece) => piece.points)
      .filter((point) => Math.abs(point.x - 100) < 1e-6)
      .map((point) => Math.round(point.y * 1000) / 1000)
    // The cubic's apex is at three quarters of the control height.
    expect(cutY).toContain(150)
  })

  it('reports the points a stroke would cut at', () => {
    const points = cutPoints([square], { x: 100, y: -50 }, { x: 100, y: 250 })
    expect(points.map((point) => Math.round(point.y))).toEqual([0, 200])
  })
})
