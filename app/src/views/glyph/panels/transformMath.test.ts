import { describe, expect, it } from 'vitest'

import {
  alignMoves,
  isNeutral,
  multiply,
  NEUTRAL_TRANSFORM,
  originPoint,
  transformAffine,
  type AlignItem,
} from './transformMath'

const box = { x0: 0, y0: 0, x1: 100, y1: 200 }

/** Applies a kurbo affine the way the kernel does. */
function apply(
  affine: readonly number[],
  point: { x: number; y: number },
): { x: number; y: number } {
  const [a = 1, b = 0, c = 0, d = 1, e = 0, f = 0] = affine
  return { x: a * point.x + c * point.y + e, y: b * point.x + d * point.y + f }
}

function near(value: number): number {
  return Math.round(value * 1e6) / 1e6
}

describe('transformAffine', () => {
  it('is the identity for an untouched panel', () => {
    expect(isNeutral(NEUTRAL_TRANSFORM)).toBe(true)
    expect(transformAffine(NEUTRAL_TRANSFORM, { x: 50, y: 100 })).toEqual([
      1, 0, 0, 1, 0, 0,
    ])
  })

  it('scales about the chosen origin', () => {
    // Bottom-left handle: that corner stays put and the box doubles.
    const origin = originPoint(box, 6)
    expect(origin).toEqual({ x: 0, y: 0 })
    const affine = transformAffine(
      { ...NEUTRAL_TRANSFORM, scaleX: 200, scaleY: 200 },
      origin,
    )
    expect(apply(affine, { x: 0, y: 0 })).toEqual({ x: 0, y: 0 })
    expect(apply(affine, { x: 100, y: 200 })).toEqual({ x: 200, y: 400 })
  })

  it('keeps the centre fixed when the origin is the centre', () => {
    const origin = originPoint(box, 4)
    expect(origin).toEqual({ x: 50, y: 100 })
    const affine = transformAffine(
      { ...NEUTRAL_TRANSFORM, scaleX: 50, scaleY: 50 },
      origin,
    )
    expect(apply(affine, origin)).toEqual(origin)
  })

  it('rotates counter-clockwise in font space', () => {
    const affine = transformAffine(
      { ...NEUTRAL_TRANSFORM, rotate: 90 },
      { x: 0, y: 0 },
    )
    const rotated = apply(affine, { x: 100, y: 0 })
    expect(near(rotated.x)).toBe(0)
    expect(near(rotated.y)).toBe(100)
  })

  it('flips horizontally about the origin', () => {
    const affine = transformAffine(
      { ...NEUTRAL_TRANSFORM, flipH: true },
      originPoint(box, 4),
    )
    expect(apply(affine, { x: 0, y: 0 })).toEqual({ x: 100, y: 0 })
    expect(apply(affine, { x: 100, y: 0 })).toEqual({ x: 0, y: 0 })
  })

  it('shears the vertical by the skew angle', () => {
    const affine = transformAffine(
      { ...NEUTRAL_TRANSFORM, skewX: 45 },
      { x: 0, y: 0 },
    )
    expect(near(apply(affine, { x: 0, y: 100 }).x)).toBe(100)
  })

  it('translates after the linear part, whatever the origin', () => {
    const affine = transformAffine(
      { ...NEUTRAL_TRANSFORM, dx: 10, dy: -5, scaleX: 200 },
      { x: 50, y: 100 },
    )
    expect(apply(affine, { x: 50, y: 100 })).toEqual({ x: 60, y: 95 })
  })

  it('composes so the right-hand transform runs first', () => {
    const scaleThenMove = multiply([1, 0, 0, 1, 10, 0], [2, 0, 0, 2, 0, 0])
    expect(apply(scaleThenMove, { x: 5, y: 0 })).toEqual({ x: 20, y: 0 })
  })
})

describe('alignMoves', () => {
  const item = (x0: number, x1: number, y0 = 0, y1 = 10): AlignItem => ({
    box: { x0, y0, x1, y1 },
    points: [{ contour: 0, point: 0 }],
  })

  it('does nothing with fewer than two items', () => {
    expect(alignMoves([item(0, 10)], 'left')).toEqual([])
  })

  it('aligns to the outermost edge and leaves that item alone', () => {
    const moves = alignMoves([item(0, 10), item(40, 60)], 'left')
    expect(moves).toHaveLength(1)
    expect(moves[0]?.dx).toBe(-40)
  })

  it('centres on the span, not on the origin', () => {
    const moves = alignMoves([item(0, 10), item(40, 60)], 'centerH')
    // Span 0..60, centre 30; the boxes' centres are 5 and 50.
    expect(moves.map((move) => move.dx)).toEqual([25, -20])
  })

  it('aligns vertically to the top of the span', () => {
    const moves = alignMoves([item(0, 10, 0, 10), item(0, 10, 50, 90)], 'top')
    expect(moves).toHaveLength(1)
    expect(moves[0]?.dy).toBe(80)
  })

  it('needs three items to distribute and keeps the outer two still', () => {
    expect(alignMoves([item(0, 10), item(40, 50)], 'distributeH')).toEqual([])
    const moves = alignMoves(
      [item(0, 10), item(15, 25), item(100, 110)],
      'distributeH',
    )
    // Extent 110, occupied 30, two gaps of 40: the middle box starts at 50.
    expect(moves).toHaveLength(1)
    expect(moves[0]?.dx).toBe(35)
  })

  it('orders by position rather than by selection order', () => {
    const moves = alignMoves(
      [item(100, 110), item(15, 25), item(0, 10)],
      'distributeH',
    )
    expect(moves).toHaveLength(1)
    expect(moves[0]?.item.box.x0).toBe(15)
  })
})
