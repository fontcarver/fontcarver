import { describe, expect, it } from 'vitest'

import type { Contour, Point, PointKind } from '../../api'
import { Viewport } from '../../canvas/Viewport'
import { createRecordingContext } from '../../test/recordingContext'

import {
  controlPoints,
  directionTarget,
  handleLines,
  onCurvePoints,
  resolveContour,
} from './contourGeometry'
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

/** A viewport that keeps positive font y on screen: screen y = 400 − font y. */
function shiftedViewport(): Viewport {
  const viewport = viewportOf()
  viewport.offset = { x: 0, y: 400 }
  return viewport
}

/** How many strokes were drawn while `strokeStyle` held `colour`. */
function strokesWith(
  recording: ReturnType<typeof createRecordingContext>,
  colour: string,
): number {
  let count = 0
  for (let index = 0; index < recording.calls('stroke').length; index += 1)
    if (recording.styleAt('stroke', 'strokeStyle', index) === colour) count += 1
  return count
}

/** The `arc` call whose centre matches, if any. */
function arcAt(
  recording: ReturnType<typeof createRecordingContext>,
  x: number,
  y: number,
): unknown[] | undefined {
  return recording
    .calls('arc')
    .map((op) => op.args)
    .find((args) => args[0] === x && args[1] === y)
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

describe('resolveContour edge cases', () => {
  /** A closed quadratic contour whose point list begins on an off-curve. */
  const offCurveStart = contourOf([
    point(0, 100, 'OffCurve'),
    point(100, 0, 'QCurve'),
    point(200, 100, 'OffCurve'),
    point(100, 200, 'QCurve'),
  ])

  /** A closed TrueType contour with no on-curve point at all. */
  const allOffCurve = contourOf([
    point(0, 0, 'OffCurve'),
    point(100, 0, 'OffCurve'),
    point(100, 100, 'OffCurve'),
    point(0, 100, 'OffCurve'),
  ])

  it('starts an off-curve-start contour at its first explicit on-curve point', () => {
    const resolved = resolveContour(offCurveStart)
    expect(resolved.start).toMatchObject({ x: 100, y: 0 })
    expect(resolved.start?.source?.kind).toBe('QCurve')
    // The path heads towards the first control, not towards points[0].
    expect(directionTarget(resolved)).toMatchObject({ x: 200, y: 100 })
    // Two distinct on-curve points; the closing segment reuses the start.
    expect(onCurvePoints(resolved)).toHaveLength(2)
    expect(handleLines(resolved)).toHaveLength(4)
  })

  it('marks the start of an off-curve-start contour, not its first point', () => {
    const recording = createRecordingContext()
    drawOutline(
      recording.ctx,
      shiftedViewport(),
      { contours: [offCurveStart], filled: false, showPoints: true },
      FALLBACK_GLYPH_THEME,
    )

    // The ring sits on the on-curve point at (100, 0), i.e. screen (100, 400).
    expect(arcAt(recording, 100, 400)).toEqual([100, 400, 6, 0, Math.PI * 2])
    // Nothing is marked at points[0] = (0, 100) → screen (0, 300).
    expect(arcAt(recording, 0, 300)?.[2]).not.toBe(6)
    expect(strokesWith(recording, FALLBACK_GLYPH_THEME.handle)).toBe(1)
  })

  it('gives an all-off-curve contour four implied on-curve points', () => {
    const resolved = resolveContour(allOffCurve)
    expect(resolved.start).toEqual({ x: 0, y: 50, source: null })
    const nodes = onCurvePoints(resolved)
    expect(nodes).toHaveLength(4)
    expect(nodes.every((node) => node.source === null)).toBe(true)
    expect(nodes.map((node) => [node.x, node.y])).toEqual([
      [0, 50],
      [50, 0],
      [100, 50],
      [50, 100],
    ])
    expect(controlPoints(resolved)).toHaveLength(4)
    // Every control is joined to the two implied points it governs.
    expect(handleLines(resolved)).toHaveLength(8)
  })

  it('draws handles, implied nodes and the start marker for an all-off-curve contour', () => {
    const recording = createRecordingContext()
    drawOutline(
      recording.ctx,
      shiftedViewport(),
      { contours: [allOffCurve], filled: false, showPoints: true },
      FALLBACK_GLYPH_THEME,
    )

    // One stroked pass for the eight handle lines.
    expect(strokesWith(recording, FALLBACK_GLYPH_THEME.handle)).toBe(1)
    const handleMoves = recording.ops
      .slice(
        recording.ops.findIndex(
          (op) =>
            op.name === 'set:strokeStyle' &&
            op.args[0] === FALLBACK_GLYPH_THEME.handle,
        ),
      )
      .filter((op) => op.name === 'moveTo')
    expect(handleMoves.length).toBeGreaterThanOrEqual(8)

    // Four hollow diamonds, one per implied on-curve point.
    expect(strokesWith(recording, FALLBACK_GLYPH_THEME.nodeImplied)).toBe(4)
    expect(recording.calls('fillRect')).toHaveLength(0)

    // The start ring sits on the first implied point, (0, 50) → (0, 350).
    expect(arcAt(recording, 0, 350)).toEqual([0, 350, 6, 0, Math.PI * 2])
  })
})
