import { describe, expect, it } from 'vitest'

import { makeContour, makePoint } from '../test/fontFixtures'

import {
  buildEditOutline,
  nearestSplitParameter,
  splitCandidates,
} from './outlineGeometry'

/** An arch: no horizontal extremum inside the span, one vertical at t = 0.5. */
const arch = buildEditOutline([
  makeContour([
    makePoint(0, 0),
    makePoint(60, 120, 'OffCurve'),
    makePoint(140, 120, 'OffCurve'),
    makePoint(200, 0, 'Curve'),
  ]),
])

/** A quadratic whose apex is likewise its only extremum. */
const quad = buildEditOutline([
  makeContour([
    makePoint(0, 0),
    makePoint(100, 200, 'OffCurve'),
    makePoint(200, 0, 'QCurve'),
  ]),
])

/** An S-curve, which has an inflection but no extremum in x or y. */
const ess = buildEditOutline([
  makeContour([
    makePoint(0, 0),
    makePoint(100, 0, 'OffCurve'),
    makePoint(0, 100, 'OffCurve'),
    makePoint(100, 100, 'Curve'),
  ]),
])

describe('splitCandidates', () => {
  it('finds the extremum of a cubic arch', () => {
    const segment = arch.segments[0]
    expect(segment?.kind).toBe('cubic')
    expect(segment && splitCandidates(segment)).toEqual([0.5])
  })

  it('finds the apex of a quadratic', () => {
    const segment = quad.segments[0]
    expect(segment?.kind).toBe('quad')
    expect(segment && splitCandidates(segment)).toEqual([0.5])
  })

  it('finds the inflection of an S-curve', () => {
    const segment = ess.segments[0]
    const found = segment ? splitCandidates(segment) : []
    expect(found).toHaveLength(1)
    expect(found[0]).toBeCloseTo(0.5, 6)
  })

  it('has nothing to offer on a straight line', () => {
    const line = buildEditOutline([
      makeContour([makePoint(0, 0), makePoint(100, 100, 'Line')], false),
    ]).segments[0]
    expect(line && splitCandidates(line)).toEqual([])
  })
})

describe('nearestSplitParameter', () => {
  it('snaps a click to the extremum of the segment', () => {
    const segment = arch.segments[0]
    expect(segment && nearestSplitParameter(segment, 0.22)).toBe(0.5)
    expect(segment && nearestSplitParameter(segment, 0.9)).toBe(0.5)
  })

  it('keeps the clicked position when the segment has no candidate', () => {
    const line = buildEditOutline([
      makeContour([makePoint(0, 0), makePoint(100, 100, 'Line')], false),
    ]).segments[0]
    expect(line && nearestSplitParameter(line, 0.31)).toBe(0.31)
  })
})
