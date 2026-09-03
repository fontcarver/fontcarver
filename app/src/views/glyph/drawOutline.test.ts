import { describe, expect, it } from 'vitest'

import type { Contour, Point, PointKind } from '../../api'
import { Viewport } from '../../canvas/Viewport'
import { createRecordingContext } from '../../test/recordingContext'

import { drawOutline, traceContour } from './drawOutline'
import { FALLBACK_GLYPH_THEME } from './glyphTheme'

function point(x: number, y: number, kind: PointKind, smooth = false): Point {
  return { x, y, kind, smooth, name: null, tt_index: 'Unnumbered' }
}

function contourOf(points: Point[], closed = true): Contour {
  return { closed, points, name: null, id: null }
}

/** An identity map so the assertions can use font coordinates directly. */
const identity = (p: { x: number; y: number }) => p

function viewportOf(): Viewport {
  const viewport = new Viewport()
  viewport.setSize(400, 400, 1)
  viewport.scale = 1
  viewport.offset = { x: 0, y: 0 }
  return viewport
}

describe('traceContour', () => {
  it('draws a cubic contour with bezierCurveTo', () => {
    const recording = createRecordingContext()
    traceContour(
      recording.ctx,
      contourOf([
        point(0, 0, 'Curve'),
        point(50, 0, 'OffCurve'),
        point(100, 50, 'OffCurve'),
        point(100, 100, 'Curve'),
        point(100, 150, 'OffCurve'),
        point(50, 200, 'OffCurve'),
        point(0, 200, 'Curve'),
      ]),
      identity,
    )

    expect(recording.calls('bezierCurveTo')).toHaveLength(2)
    expect(recording.calls('quadraticCurveTo')).toHaveLength(0)
    expect(recording.calls('moveTo')[0]?.args).toEqual([0, 0])
    expect(recording.calls('bezierCurveTo')[1]?.args.slice(4)).toEqual([0, 200])
    // The start point carries no preceding control, so the contour closes with
    // a straight segment, exactly as the UFO point list describes it.
    expect(recording.calls('lineTo').map((op) => op.args)).toEqual([[0, 0]])
    expect(recording.calls('closePath')).toHaveLength(1)
  })

  it('draws a quadratic contour with quadraticCurveTo', () => {
    const recording = createRecordingContext()
    traceContour(
      recording.ctx,
      contourOf([
        point(0, 0, 'QCurve'),
        point(0, 100, 'OffCurve'),
        point(100, 100, 'QCurve'),
        point(200, 100, 'OffCurve'),
        point(200, 0, 'QCurve'),
      ]),
      identity,
    )

    expect(recording.calls('quadraticCurveTo')).toHaveLength(2)
    expect(recording.calls('bezierCurveTo')).toHaveLength(0)
    expect(recording.calls('lineTo').map((op) => op.args)).toEqual([[0, 0]])
  })

  it('inserts the implied on-curve point between consecutive controls', () => {
    const recording = createRecordingContext()
    traceContour(
      recording.ctx,
      contourOf([
        point(0, 0, 'QCurve'),
        point(0, 100, 'OffCurve'),
        point(100, 100, 'OffCurve'),
        point(100, 0, 'QCurve'),
      ]),
      identity,
    )

    const quads = recording.calls('quadraticCurveTo')
    // Two controls before one on-curve point: one segment to the implied
    // midpoint, then the segment that actually reaches the on-curve point.
    expect(quads).toHaveLength(2)
    expect(quads[0]?.args).toEqual([0, 100, 50, 100])
    expect(quads[1]?.args).toEqual([100, 100, 100, 0])
  })

  it('starts an all-off-curve contour at an implied midpoint', () => {
    const recording = createRecordingContext()
    traceContour(
      recording.ctx,
      contourOf([
        point(0, 0, 'OffCurve'),
        point(100, 0, 'OffCurve'),
        point(100, 100, 'OffCurve'),
        point(0, 100, 'OffCurve'),
      ]),
      identity,
    )

    expect(recording.calls('moveTo')[0]?.args).toEqual([0, 50])
    expect(recording.calls('quadraticCurveTo')).toHaveLength(4)
    expect(recording.calls('closePath')).toHaveLength(1)
  })

  it('draws straight segments and open contours', () => {
    const recording = createRecordingContext()
    traceContour(
      recording.ctx,
      contourOf(
        [point(0, 0, 'Move'), point(100, 0, 'Line'), point(100, 100, 'Line')],
        false,
      ),
      identity,
    )

    expect(recording.calls('moveTo')[0]?.args).toEqual([0, 0])
    expect(recording.calls('lineTo').map((op) => op.args)).toEqual([
      [100, 0],
      [100, 100],
    ])
    expect(recording.calls('closePath')).toHaveLength(0)
  })

  it('ignores an empty contour', () => {
    const recording = createRecordingContext()
    traceContour(recording.ctx, contourOf([]), identity)
    expect(recording.ops).toHaveLength(0)
  })
})

describe('drawOutline', () => {
  const square = contourOf([
    point(0, 0, 'Line'),
    point(100, 0, 'Line'),
    point(100, 100, 'Line', true),
    point(0, 100, 'Line'),
  ])

  it('strokes the outline by default and fills in preview mode', () => {
    const outline = createRecordingContext()
    drawOutline(
      outline.ctx,
      viewportOf(),
      { contours: [square], filled: false, showPoints: false },
      FALLBACK_GLYPH_THEME,
    )
    expect(outline.calls('stroke').length).toBeGreaterThan(0)
    expect(outline.calls('fill')).toHaveLength(0)

    const filled = createRecordingContext()
    drawOutline(
      filled.ctx,
      viewportOf(),
      { contours: [square], filled: true, showPoints: false },
      FALLBACK_GLYPH_THEME,
    )
    expect(filled.calls('fill')[0]?.args).toEqual(['nonzero'])
    expect(filled.styleAt('fill', 'fillStyle')).toBe(FALLBACK_GLYPH_THEME.fill)
  })

  it('draws corner nodes as squares and smooth nodes as arcs', () => {
    const recording = createRecordingContext()
    drawOutline(
      recording.ctx,
      viewportOf(),
      { contours: [square], filled: false, showPoints: true },
      FALLBACK_GLYPH_THEME,
    )

    // Three corner nodes are filled rectangles, the smooth one is an arc.
    expect(recording.calls('fillRect')).toHaveLength(3)
    expect(recording.styleAt('fillRect', 'fillStyle')).toBe(
      FALLBACK_GLYPH_THEME.nodeCorner,
    )
    const arcs = recording.calls('arc')
    // One smooth node plus the start-point ring.
    expect(arcs).toHaveLength(2)
  })

  it('draws off-curve points with a handle line to their node', () => {
    const recording = createRecordingContext()
    drawOutline(
      recording.ctx,
      viewportOf(),
      {
        contours: [
          contourOf([
            point(0, 0, 'Curve'),
            point(50, 0, 'OffCurve'),
            point(100, 50, 'OffCurve'),
            point(100, 100, 'Curve'),
          ]),
        ],
        filled: false,
        showPoints: true,
      },
      FALLBACK_GLYPH_THEME,
    )

    // Each off-curve point is joined to the single on-curve neighbour it has.
    const handleStrokes = recording.ops.filter(
      (op) => op.name === 'set:strokeStyle',
    )
    expect(
      handleStrokes.some((op) => op.args[0] === FALLBACK_GLYPH_THEME.handle),
    ).toBe(true)
    expect(recording.calls('arc').length).toBeGreaterThanOrEqual(2)
  })

  it('draws nothing for a layer with no contours', () => {
    const recording = createRecordingContext()
    drawOutline(
      recording.ctx,
      viewportOf(),
      { contours: [], filled: false, showPoints: true },
      FALLBACK_GLYPH_THEME,
    )
    expect(recording.ops).toHaveLength(0)
  })
})
