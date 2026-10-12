import { describe, expect, it } from 'vitest'

import { makeContour, makePoint } from '../../test/fontFixtures'

import { auditOutline, outlineAuditFix } from './outlineAuditModel'

const target = { doc: 1, glyph: 'a', layer: 'layer-1' }

describe('live outline audit', () => {
  it('locates open contours, fractional points and coincident stored points', () => {
    const contours = [
      makeContour([makePoint(10.4, 20.6, 'Move'), makePoint(10, 20)], false),
      makeContour([makePoint(10, 20), makePoint(30, 40)]),
    ]
    const report = auditOutline(contours)
    expect(report.issues.map((issue) => issue.test)).toEqual([
      'open',
      'fractional',
      'coincident',
    ])
    expect(report.issues[2]?.points).toEqual([
      { contour: 0, point: 1 },
      { contour: 1, point: 0 },
    ])
    expect(report.truncated).toBe(false)
  })

  it('honors per-test toggles and never emits non-finite marker positions', () => {
    const contours = [
      makeContour(
        [makePoint(Number.NaN, 10), makePoint(1.2, 10), makePoint(1.2, 10)],
        false,
      ),
    ]
    expect(
      auditOutline(contours, {
        open: false,
        fractional: false,
        coincident: true,
        extrema: false,
      }).issues,
    ).toMatchObject([{ test: 'coincident', x: 1.2, y: 10 }])
    expect(
      auditOutline(contours).issues.every((issue) => Number.isFinite(issue.x)),
    ).toBe(true)
  })

  it('bounds markers without pretending that every issue is shown', () => {
    const contours = [
      makeContour(
        Array.from({ length: 150 }, (_, index) => makePoint(index + 0.2, 0)),
      ),
    ]
    const report = auditOutline(contours)
    expect(report.issues).toHaveLength(100)
    expect(report.truncated).toBe(true)
  })

  it('rounds only explicitly requested fractional points and batches one undo entry', () => {
    const contours = [
      makeContour([makePoint(10.4, -20.6), makePoint(11.7, 30.2)]),
    ]
    const issues = auditOutline(contours).issues
    expect(outlineAuditFix(target, contours, issues.slice(0, 1))).toEqual({
      SetPointPosition: {
        glyph: 'a',
        layer: 'layer-1',
        point: { contour: 0, point: 0 },
        x: 10,
        y: -21,
      },
    })
    const command = outlineAuditFix(target, contours, issues)
    expect(command && 'Batch' in command && command.Batch).toHaveLength(2)
    expect(contours[0]?.points[0]?.x).toBe(10.4)
  })

  it('refuses to repair anything the kernel no longer reports', () => {
    const contours = [
      makeContour([makePoint(1.2, 0), makePoint(1.2, 0)], false),
    ]
    const issues = auditOutline(contours).issues
    expect(
      outlineAuditFix(target, [makeContour([makePoint(9.8, 0)])], issues),
    ).toBeNull()
  })

  it('closes an open contour whose two ends are on-curve nodes', () => {
    const contours = [
      makeContour(
        [makePoint(0, 0, 'Move'), makePoint(100, 0), makePoint(100, 100)],
        false,
      ),
    ]
    const open = auditOutline(contours).issues.filter(
      (issue) => issue.test === 'open',
    )
    expect(open).toHaveLength(1)
    expect(outlineAuditFix(target, contours, open)).toEqual({
      JoinContours: {
        glyph: 'a',
        layer: 'layer-1',
        a: { contour: 0, point: 2 },
        b: { contour: 0, point: 0 },
      },
    })
  })

  it('leaves an open contour that ends in a handle to manual review', () => {
    const contours = [
      makeContour(
        [
          makePoint(0, 0, 'Move'),
          makePoint(50, 50, 'OffCurve'),
          makePoint(80, 50, 'OffCurve'),
        ],
        false,
      ),
    ]
    const open = auditOutline(contours).issues.filter(
      (issue) => issue.test === 'open',
    )
    expect(open).toHaveLength(1)
    expect(open[0]?.fix).toBeUndefined()
    expect(outlineAuditFix(target, contours, open)).toBeNull()
  })

  it('deletes adjacent duplicate nodes but not a coincidence across contours', () => {
    const duplicate = [
      makeContour([
        makePoint(0, 0),
        makePoint(100, 0),
        makePoint(100, 0),
        makePoint(100, 80),
      ]),
    ]
    const issue = auditOutline(duplicate).issues.find(
      (entry) => entry.test === 'coincident',
    )
    expect(issue?.fix).toMatchObject({ kind: 'dedupe' })
    expect(outlineAuditFix(target, duplicate, issue ? [issue] : [])).toEqual({
      DeletePoints: {
        glyph: 'a',
        layer: 'layer-1',
        points: [{ contour: 0, point: 2 }],
        mode: 'KeepShape',
      },
    })

    const shared = [
      makeContour([makePoint(0, 0), makePoint(10, 0), makePoint(0, 10)]),
      makeContour([makePoint(0, 0), makePoint(-10, 0), makePoint(0, -10)]),
    ]
    const across = auditOutline(shared).issues.find(
      (entry) => entry.test === 'coincident',
    )
    expect(across?.points).toHaveLength(2)
    expect(across?.fix).toBeUndefined()
  })

  it('reports a curve extreme with no node on it and inserts one', () => {
    // A half circle from (0,0) to (200,0) bulging to y ≈ 100: its vertical
    // extreme sits in the middle of the segment, with no node there.
    const contours = [
      makeContour([
        makePoint(0, 0),
        makePoint(0, 133, 'OffCurve'),
        makePoint(200, 133, 'OffCurve'),
        makePoint(200, 0, 'Curve'),
      ]),
    ]
    const extrema = auditOutline(contours).issues.filter(
      (issue) => issue.test === 'extrema',
    )
    expect(extrema).toHaveLength(1)
    expect(extrema[0]?.y).toBeCloseTo(99.75, 1)
    expect(outlineAuditFix(target, contours, extrema)).toEqual({
      InsertPointOnSegment: {
        glyph: 'a',
        layer: 'layer-1',
        contour: 0,
        segment: 0,
        t: 0.5,
      },
    })
    // A segment whose extreme is its own endpoint is already correct.
    const clean = [
      makeContour([
        makePoint(0, 0),
        makePoint(110, 0, 'OffCurve'),
        makePoint(200, 90, 'OffCurve'),
        makePoint(200, 200, 'Curve'),
      ]),
    ]
    expect(
      auditOutline(clean).issues.filter((issue) => issue.test === 'extrema'),
    ).toHaveLength(0)
  })

  it('applies one structural repair per contour and keeps the rest listed', () => {
    // One contour with both a duplicate node and a missing extreme: the two
    // repairs renumber the same contour, so only the higher-priority one runs.
    const contours = [
      makeContour([
        makePoint(0, 0),
        makePoint(0, 133, 'OffCurve'),
        makePoint(200, 133, 'OffCurve'),
        makePoint(200, 0, 'Curve'),
        makePoint(200, 0),
      ]),
    ]
    const issues = auditOutline(contours).issues
    expect(issues.some((issue) => issue.test === 'coincident')).toBe(true)
    expect(issues.some((issue) => issue.test === 'extrema')).toBe(true)
    const command = outlineAuditFix(target, contours, issues)
    expect(command && 'DeletePoints' in command).toBe(true)
  })

  it('remembers nothing about disabled checks in a repair', () => {
    const contours = [
      makeContour([makePoint(10.4, 0), makePoint(100, 0), makePoint(100, 80)]),
    ]
    const fractional = auditOutline(contours, {
      open: true,
      fractional: true,
      coincident: false,
      extrema: false,
    }).issues
    expect(fractional).toHaveLength(1)
    expect(outlineAuditFix(target, contours, fractional)).toMatchObject({
      SetPointPosition: { x: 10, y: 0 },
    })
  })
})

describe('two extremes on one segment', () => {
  // The reviewer's cubic: an S that leaves and re-enters its own x range, so
  // one segment carries two horizontal extremes at t ≈ 0.2113 and 0.7887.
  const doubled = [
    makeContour([
      makePoint(0, 0),
      makePoint(300, 400, 'OffCurve'),
      makePoint(-300, 400, 'OffCurve'),
      makePoint(0, 800, 'Curve'),
    ]),
  ]

  it('repairs both with parameters relative to the remaining left segment', () => {
    const extrema = auditOutline(doubled).issues.filter(
      (issue) => issue.test === 'extrema',
    )
    expect(extrema).toHaveLength(2)
    expect(
      extrema.map((issue) =>
        issue.fix?.kind === 'insertExtremum'
          ? Math.round(issue.fix.t * 1e4) / 1e4
          : null,
      ),
    ).toEqual([0.2113, 0.7887])

    const command = outlineAuditFix(target, doubled, extrema)
    const batch = command && 'Batch' in command ? command.Batch : []
    expect(batch).toHaveLength(2)
    const parameters = batch.map((entry) =>
      'InsertPointOnSegment' in entry ? entry.InsertPointOnSegment.t : 0,
    )
    const originalParameters = extrema.map((issue) =>
      issue.fix?.kind === 'insertExtremum' ? issue.fix.t : 0,
    )
    expect(parameters[0]).toBe(originalParameters[1])
    expect(parameters[1]).toBeCloseTo(
      originalParameters[0]! / originalParameters[1]!,
      12,
    )
  })

  it('still repairs one extreme per segment across several segments', () => {
    // Two curved segments, each with one missing extreme: both are safe in one
    // batch as long as the higher segment is split first.
    const two = [
      makeContour([
        makePoint(0, 0),
        makePoint(0, 133, 'OffCurve'),
        makePoint(200, 133, 'OffCurve'),
        makePoint(200, 0, 'Curve'),
        makePoint(200, -133, 'OffCurve'),
        makePoint(0, -133, 'OffCurve'),
        makePoint(0, 0, 'Curve'),
      ]),
    ]
    const extrema = auditOutline(two).issues.filter(
      (issue) => issue.test === 'extrema',
    )
    expect(extrema).toHaveLength(2)
    const command = outlineAuditFix(target, two, extrema)
    const batch = command && 'Batch' in command ? command.Batch : []
    expect(batch).toHaveLength(2)
    const segments = batch.map((entry) =>
      'InsertPointOnSegment' in entry
        ? entry.InsertPointOnSegment.segment
        : null,
    )
    expect(segments).toEqual([1, 0])
  })
})

it('revalidates enabled extrema when disabled fractional markers would fill the report', () => {
  const contours = [
    makeContour(Array.from({ length: 110 }, (_, i) => makePoint(i + 0.2, 0))),
    makeContour([
      makePoint(0, 0),
      makePoint(300, 400, 'OffCurve'),
      makePoint(-300, 400, 'OffCurve'),
      makePoint(0, 800, 'Curve'),
    ]),
  ]
  const issues = auditOutline(contours, {
    open: false,
    fractional: false,
    coincident: false,
    extrema: true,
  }).issues
  expect(issues).toHaveLength(2)
  const command = outlineAuditFix(target, contours, issues)
  expect(command && 'Batch' in command && command.Batch).toHaveLength(2)
})
