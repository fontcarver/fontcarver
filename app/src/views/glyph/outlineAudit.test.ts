import { describe, expect, it } from 'vitest'

import { makeContour, makePoint } from '../../test/fontFixtures'

import { auditOutline, outlineAuditFix } from './outlineAudit'

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

  it('does not invent fixes for open paths, coincident points or stale point positions', () => {
    const contours = [
      makeContour([makePoint(1.2, 0), makePoint(1.2, 0)], false),
    ]
    const issues = auditOutline(contours).issues
    expect(
      outlineAuditFix(
        target,
        contours,
        issues.filter((issue) => issue.test !== 'fractional'),
      ),
    ).toBeNull()
    expect(
      outlineAuditFix(target, [makeContour([makePoint(9.8, 0)])], issues),
    ).toBeNull()
  })
})
