import { describe, expect, it } from 'vitest'

import { makeContour, makePoint } from '../../../test/fontFixtures'
import { detectStraightStems, servantNeighbors } from './constructionGeometry'

function rectangle(
  x: number,
  y: number,
  width: number,
  height: number,
  reverse = false,
) {
  const points = [
    [x, y],
    [x + width, y],
    [x + width, y + height],
    [x, y + height],
  ]
  return makeContour(
    (reverse ? points.reverse() : points).map(([px, py]) =>
      makePoint(px!, py!, 'Line'),
    ),
  )
}

describe('straight stem detection', () => {
  it('detects a narrow vertical stem and keeps the source edge nodes', () => {
    const stems = detectStraightStems([rectangle(10, 0, 70, 700)], 200)
    expect(stems).toHaveLength(1)
    expect(stems[0]).toMatchObject({
      axis: 'X',
      width: 70,
      low: { position: 10 },
      high: { position: 80 },
    })
    expect(stems[0]?.low.points).toEqual([
      { contour: 0, point: 3 },
      { contour: 0, point: 0 },
    ])
  })

  it('detects a horizontal bar', () => {
    expect(
      detectStraightStems([rectangle(0, 120, 500, 50)], 200),
    ).toMatchObject([{ axis: 'Y', width: 50 }])
  })

  it('detects ink around a counter without treating the hole as a stem', () => {
    const stems = detectStraightStems(
      [rectangle(0, 0, 200, 500), rectangle(40, 50, 120, 400, true)],
      180,
    )
    expect(
      stems.filter((stem) => stem.axis === 'X').map((stem) => stem.width),
    ).toEqual([40, 40])
    expect(
      stems.filter((stem) => stem.axis === 'Y').map((stem) => stem.width),
    ).toEqual([50, 50])
  })

  it('rejects open, degenerate and zero-width candidates', () => {
    const open = { ...rectangle(0, 0, 40, 500), closed: false }
    expect(detectStraightStems([open], 200)).toEqual([])
    expect(detectStraightStems([rectangle(0, 0, 0, 500)], 200)).toEqual([])
    expect(detectStraightStems([rectangle(0, 0, 40, 500)], NaN)).toEqual([])
  })

  it('does not bridge independent rectangles across whitespace', () => {
    const stems = detectStraightStems(
      [rectangle(0, 0, 20, 500), rectangle(50, 0, 20, 500)],
      100,
    )
    expect(stems.map((stem) => stem.width)).toEqual([20, 20])
  })
})

describe('servant neighbors', () => {
  it('skips off-curve handles and rejects open endpoints', () => {
    const contour = makeContour(
      [
        makePoint(0, 0, 'Move'),
        makePoint(25, 10, 'OffCurve'),
        makePoint(50, 0, 'QCurve'),
        makePoint(100, 0, 'Line'),
      ],
      false,
    )
    expect(
      servantNeighbors([contour], { contour: 0, point: 2 }, new Set()),
    ).toEqual([
      { contour: 0, point: 0 },
      { contour: 0, point: 3 },
    ])
    expect(
      servantNeighbors([contour], { contour: 0, point: 0 }, new Set()),
    ).toBeNull()
  })

  it('skips an existing servant and wraps a closed path', () => {
    const contour = rectangle(0, 0, 100, 500)
    expect(
      servantNeighbors([contour], { contour: 0, point: 0 }, new Set(['0:1'])),
    ).toEqual([
      { contour: 0, point: 3 },
      { contour: 0, point: 2 },
    ])
  })
})
