import { describe, expect, it } from 'vitest'

import type { Component } from '../../api'
import { Viewport } from '../../canvas/Viewport'
import { createRecordingContext } from '../../test/recordingContext'
import { makeContour, makePoint } from '../../test/fontFixtures'

import { drawComponents } from './drawComponents'
import { drawGuides } from './drawGuides'
import { drawOutline } from './drawOutline'
import { FALLBACK_GLYPH_THEME } from './glyphTheme'
import { guideCrossings, guideMeasurements } from './guideMeasure'

/** One CSS pixel per font unit, origin at canvas (100, 500). */
function viewportOf(): Viewport {
  const viewport = new Viewport()
  viewport.setSize(800, 600, 1)
  viewport.scale = 1
  viewport.offset = { x: 100, y: 500 }
  return viewport
}

function component(overrides: Partial<Component> = {}): Component {
  return {
    base: 'acute',
    transform: [1, 0, 0, 1, 0, 0],
    anchor: null,
    auto_align: false,
    use_my_metrics: false,
    round_to_grid: false,
    smart: null,
    point_match: null,
    ...overrides,
  }
}

describe('the vertical-metric diamond', () => {
  const box = makeContour([
    makePoint(0, 0),
    makePoint(100, 0, 'Line'),
    makePoint(100, 500, 'Line'),
    makePoint(0, 500, 'Line'),
  ])

  function diamonds(metrics: number[]): number {
    const recording = createRecordingContext()
    drawOutline(
      recording.ctx,
      viewportOf(),
      {
        contours: [box],
        filled: false,
        showPoints: true,
        verticalMetrics: metrics,
      },
      FALLBACK_GLYPH_THEME,
    )
    // Each diamond is a four-sided stroked path; the only other stroked paths
    // here are the outline itself and the open-end ticks of an open contour.
    return recording
      .calls('stroke')
      .filter(
        (_, index) =>
          recording.styleAt('stroke', 'strokeStyle', index) ===
          FALLBACK_GLYPH_THEME.zone,
      ).length
  }

  it('marks the two nodes on the baseline and the two on the x-height', () => {
    expect(diamonds([0, 500])).toBe(4)
  })

  it('marks nothing when no node sits on a metric', () => {
    expect(diamonds([700, 800])).toBe(0)
  })
})

describe('the locked-component hatch', () => {
  function hatched(hovered: number | null, autoAlign: boolean): boolean {
    const recording = createRecordingContext()
    drawComponents(
      recording.ctx,
      viewportOf(),
      {
        path: null,
        components: [component({ auto_align: autoAlign })],
        bounds: [[0, 0, 100, 200]],
        hovered,
        filled: false,
        showLabels: false,
      },
      FALLBACK_GLYPH_THEME,
    )
    return recording.calls('clip').length > 0
  }

  it('hatches an auto-aligned component the pointer is over', () => {
    expect(hatched(0, true)).toBe(true)
  })

  it('leaves a movable component and an unhovered one alone', () => {
    expect(hatched(0, false)).toBe(false)
    expect(hatched(null, true)).toBe(false)
  })
})

describe('guide overlays', () => {
  const guide = {
    guide: { x: 0, y: 100, angle: 0, name: null, locked: false, color: null },
    scope: 'layer' as const,
  }

  const stem = makeContour([
    makePoint(0, 0),
    makePoint(60, 0, 'Line'),
    makePoint(60, 400, 'Line'),
    makePoint(0, 400, 'Line'),
  ])

  it('draws an angle knob only for a selected guide', () => {
    const withKnob = createRecordingContext()
    drawGuides(
      withKnob.ctx,
      viewportOf(),
      {
        guides: [guide],
        showLabels: false,
        selected: new Set(['guide:layer:0']),
      },
      FALLBACK_GLYPH_THEME,
    )
    expect(withKnob.calls('arc')).toHaveLength(1)

    const plain = createRecordingContext()
    drawGuides(
      plain.ctx,
      viewportOf(),
      { guides: [guide], showLabels: false },
      FALLBACK_GLYPH_THEME,
    )
    expect(plain.calls('arc')).toHaveLength(0)
  })

  it('labels the intersection distances of a measurement guide', () => {
    const recording = createRecordingContext()
    drawGuides(
      recording.ctx,
      viewportOf(),
      {
        guides: [guide],
        showLabels: false,
        measurement: new Set(['layer:0']),
        contours: [stem],
      },
      FALLBACK_GLYPH_THEME,
    )

    // The horizontal guide at y = 100 crosses the 60-unit stem twice.
    expect(recording.calls('fillText').map((op) => op.args[0])).toEqual(['60'])
  })
})

describe('guide measurements', () => {
  const stem = makeContour([
    makePoint(0, 0),
    makePoint(60, 0, 'Line'),
    makePoint(60, 400, 'Line'),
    makePoint(0, 400, 'Line'),
  ])
  const counter = makeContour([
    makePoint(200, 0),
    makePoint(260, 0, 'Line'),
    makePoint(260, 400, 'Line'),
    makePoint(200, 400, 'Line'),
  ])

  it('orders the crossings along the guide', () => {
    const crossings = guideCrossings([stem, counter], {
      x: 0,
      y: 100,
      angle: 0,
    })
    expect(crossings.map((crossing) => Math.round(crossing.t))).toEqual([
      0, 60, 200, 260,
    ])
  })

  it('reports the stem width and the gap between the stems', () => {
    const spans = guideMeasurements([stem, counter], { x: 0, y: 100, angle: 0 })
    expect(spans.map((span) => Math.round(span.length))).toEqual([60, 140, 60])
  })

  it('measures along a slanted guide too', () => {
    // A 45° line through the origin enters the 60-wide stem at (0, 0) and
    // leaves it at (60, 60): the span is the diagonal, not the stem width.
    const spans = guideMeasurements([stem], { x: 0, y: 0, angle: 45 })
    expect(spans.map((span) => Math.round(span.length))).toEqual([85])
  })

  it('measures from wherever the guide origin sits on its own line', () => {
    const across = guideMeasurements([stem], { x: -100, y: 100, angle: 0 })
    expect(across.map((span) => Math.round(span.length))).toEqual([60])
  })

  it('reports nothing for a guide that misses the outline', () => {
    expect(guideMeasurements([stem], { x: 0, y: 900, angle: 0 })).toEqual([])
  })

  it('counts a vertex the guide merely touches once, not twice', () => {
    // A diamond whose apex sits exactly on the guide: the two segments that
    // meet there share the point, and it must not be reported for both.
    const diamond = makeContour([
      makePoint(0, 0),
      makePoint(30, 100, 'Line'),
      makePoint(60, 0, 'Line'),
      makePoint(30, -100, 'Line'),
    ])
    expect(guideCrossings([diamond], { x: 0, y: 100, angle: 0 })).toHaveLength(
      1,
    )
  })

  it('reports the far side of a guide lying along an edge', () => {
    // The baseline runs *along* the bottom edge of the stem, which crosses
    // nothing: only the opposite side of the contour is a real crossing, so
    // a guide laid on an edge measures nothing rather than a phantom span.
    expect(guideCrossings([stem], { x: 0, y: 0, angle: 0 })).toHaveLength(1)
    expect(guideMeasurements([stem], { x: 0, y: 0, angle: 0 })).toEqual([])
  })
})
