import { describe, expect, it } from 'vitest'

import { Viewport } from '../../canvas/Viewport'
import { IDENTITY } from '../../editor/affine'
import { createRecordingContext } from '../../test/recordingContext'
import {
  makeCell,
  makeGlyph,
  makeGlyphDetail,
  makeLayer,
  makeMaster,
} from '../../test/fontFixtures'

import { drawAnchors } from './drawAnchors'
import { drawComponents } from './drawComponents'
import { drawGuides } from './drawGuides'
import { DEFAULT_METRIC_LABELS, drawMetrics } from './drawMetrics'
import { drawRulers, RULER_SIZE, tickStep } from './drawRulers'
import { buildScene } from './glyphScene'
import { FALLBACK_GLYPH_THEME } from './glyphTheme'

/** A viewport with one CSS pixel per font unit and the origin at (100, 500). */
function viewportOf(): Viewport {
  const viewport = new Viewport()
  viewport.setSize(800, 600, 1)
  viewport.scale = 1
  viewport.offset = { x: 100, y: 500 }
  return viewport
}

const scene = buildScene({
  detail: makeGlyphDetail(),
  cell: makeCell(),
  master: makeMaster(),
  masterId: 'm1',
  unitsPerEm: 1000,
})

describe('drawMetrics', () => {
  it('draws each vertical metric at its font position', () => {
    const recording = createRecordingContext()
    // Half a pixel per unit with the baseline at y = 450 shows every metric.
    const viewport = new Viewport()
    viewport.setSize(800, 600, 1)
    viewport.scale = 0.5
    viewport.offset = { x: 100, y: 450 }

    drawMetrics(
      recording.ctx,
      viewport,
      {
        metrics: scene.metrics,
        labels: DEFAULT_METRIC_LABELS,
        showLabels: false,
      },
      FALLBACK_GLYPH_THEME,
    )

    const horizontal = recording
      .calls('moveTo')
      .filter((op) => op.args[0] === 0)
      .map((op) => op.args[1])
    // descender −200, baseline 0, x-height 500, cap height 700, ascender 800.
    expect(horizontal).toEqual([550.5, 450.5, 200.5, 100.5, 50.5])
  })

  it('culls the metrics that fall outside the canvas', () => {
    const recording = createRecordingContext()
    drawMetrics(
      recording.ctx,
      viewportOf(),
      {
        metrics: scene.metrics,
        labels: DEFAULT_METRIC_LABELS,
        showLabels: false,
      },
      FALLBACK_GLYPH_THEME,
    )
    const horizontal = recording
      .calls('moveTo')
      .filter((op) => op.args[0] === 0)
      .map((op) => op.args[1])
    // Only the baseline and the x-height are on screen at this zoom.
    expect(horizontal).toEqual([500.5, 0.5])
  })

  it('marks the baseline more strongly than the other metrics', () => {
    const recording = createRecordingContext()
    drawMetrics(
      recording.ctx,
      viewportOf(),
      {
        metrics: scene.metrics,
        labels: DEFAULT_METRIC_LABELS,
        showLabels: false,
      },
      FALLBACK_GLYPH_THEME,
    )
    const strokes = recording.ops
      .filter((op) => op.name === 'set:strokeStyle')
      .map((op) => op.args[0])
    expect(strokes).toContain(FALLBACK_GLYPH_THEME.metricStrong)
    expect(strokes).toContain(FALLBACK_GLYPH_THEME.metric)
  })

  it('dashes the sidebearing lines and leaves the advance solid', () => {
    const recording = createRecordingContext()
    drawMetrics(
      recording.ctx,
      viewportOf(),
      {
        metrics: scene.metrics,
        labels: DEFAULT_METRIC_LABELS,
        showLabels: false,
      },
      FALLBACK_GLYPH_THEME,
    )
    const dashes = recording.calls('setLineDash').map((op) => op.args[0])
    expect(dashes).toContainEqual([3, 3])
    const verticals = recording
      .calls('moveTo')
      .filter((op) => op.args[1] === 0)
      .map((op) => op.args[0])
    // origin at x=0 → 100, advance at x=600 → 700, LSB 40 → 140, RSB 560 → 660.
    expect(verticals).toEqual([100.5, 700.5, 140.5, 660.5])
  })

  it('drops a metric the font leaves at zero instead of doubling the baseline', () => {
    const recording = createRecordingContext()
    const viewport = new Viewport()
    viewport.setSize(800, 600, 1)
    viewport.scale = 0.5
    viewport.offset = { x: 100, y: 450 }

    drawMetrics(
      recording.ctx,
      viewport,
      {
        // DejaVu reports no x-height or cap height, which arrive as zero.
        metrics: { ...scene.metrics, xHeight: 0, capHeight: 0 },
        labels: DEFAULT_METRIC_LABELS,
        showLabels: true,
      },
      FALLBACK_GLYPH_THEME,
    )

    const horizontal = recording
      .calls('moveTo')
      .filter((op) => op.args[0] === 0)
      .map((op) => op.args[1])
    expect(horizontal).toEqual([550.5, 450.5, 50.5])
    expect(recording.calls('fillText').map((op) => op.args[0])).not.toContain(
      'x-height 0',
    )
  })

  it('keeps the metric labels clear of the vertical ruler', () => {
    const recording = createRecordingContext()
    drawMetrics(
      recording.ctx,
      viewportOf(),
      {
        metrics: scene.metrics,
        labels: DEFAULT_METRIC_LABELS,
        showLabels: true,
      },
      FALLBACK_GLYPH_THEME,
    )
    const labelled = recording
      .calls('fillText')
      .filter((op) => String(op.args[0]).startsWith('baseline'))
    expect(labelled[0]?.args[1]).toBeGreaterThanOrEqual(RULER_SIZE)
  })

  it('draws the vertical origin and advance only when the layer has them', () => {
    const plain = createRecordingContext()
    drawMetrics(
      plain.ctx,
      viewportOf(),
      {
        metrics: scene.metrics,
        labels: DEFAULT_METRIC_LABELS,
        showLabels: false,
      },
      FALLBACK_GLYPH_THEME,
    )
    const before = plain.calls('stroke').length

    const vertical = buildScene({
      detail: makeGlyphDetail(
        makeGlyph({
          layers: [makeLayer({ vertical_origin: 200, vertical_width: 300 })],
        }),
      ),
      cell: makeCell(),
      master: makeMaster(),
      masterId: 'm1',
      unitsPerEm: 1000,
    })
    const recording = createRecordingContext()
    drawMetrics(
      recording.ctx,
      viewportOf(),
      {
        metrics: vertical.metrics,
        labels: DEFAULT_METRIC_LABELS,
        showLabels: false,
      },
      FALLBACK_GLYPH_THEME,
    )
    expect(recording.calls('stroke').length).toBe(before + 2)
  })
})

describe('drawComponents', () => {
  const component = {
    base: 'a',
    transform: [1, 0, 0, 1, 120, 40] as [
      number,
      number,
      number,
      number,
      number,
      number,
    ],
    anchor: null,
    auto_align: false,
    use_my_metrics: false,
    round_to_grid: false,
    smart: null,
    point_match: null,
  }

  it('draws nothing for a layer with no components', () => {
    const recording = createRecordingContext()
    drawComponents(
      recording.ctx,
      viewportOf(),
      { path: null, components: [], filled: false, showLabels: true },
      FALLBACK_GLYPH_THEME,
    )
    expect(recording.ops).toHaveLength(0)
  })

  it('strokes the flattened path in the component colour', () => {
    const recording = createRecordingContext()
    const path = {} as Path2D
    drawComponents(
      recording.ctx,
      viewportOf(),
      {
        path,
        components: [{ component, path: null, transform: IDENTITY }],
        filled: false,
        showLabels: false,
      },
      FALLBACK_GLYPH_THEME,
    )
    expect(recording.calls('stroke')[0]?.args).toEqual([path])
    expect(recording.styleAt('stroke', 'strokeStyle')).toBe(
      FALLBACK_GLYPH_THEME.component,
    )
  })

  it('draws each component from its own path, offset by the preview', () => {
    const recording = createRecordingContext()
    const own = {} as Path2D
    drawComponents(
      recording.ctx,
      viewportOf(),
      {
        path: {} as Path2D,
        components: [
          { component, path: own, transform: [1, 0, 0, 1, 40, -10] },
        ],
        filled: false,
        showLabels: false,
      },
      FALLBACK_GLYPH_THEME,
    )
    // The component's own path wins over the whole-layer fallback, and the
    // preview transform is applied inside the font-space transform.
    expect(recording.calls('stroke')[0]?.args).toEqual([own])
    expect(recording.calls('translate').map((entry) => entry.args)).toEqual([
      [100, 500],
    ])
    expect(recording.calls('transform')[0]?.args).toEqual([1, 0, 0, 1, 40, -10])
  })

  it('draws a previewed rotation as a rotation, not as a slide', () => {
    const recording = createRecordingContext()
    const own = {} as Path2D
    // A quarter turn about the origin: the linear part is not the identity,
    // which a `dx`/`dy` offset could never express.
    const quarter: [number, number, number, number, number, number] = [
      0, 1, -1, 0, 0, 0,
    ]
    drawComponents(
      recording.ctx,
      viewportOf(),
      {
        path: null,
        components: [{ component, path: own, transform: quarter }],
        filled: false,
        showLabels: false,
      },
      FALLBACK_GLYPH_THEME,
    )
    expect(recording.calls('transform')[0]?.args).toEqual(quarter)
  })

  it('fills in preview mode and labels each component origin', () => {
    const recording = createRecordingContext()
    const path = {} as Path2D
    drawComponents(
      recording.ctx,
      viewportOf(),
      {
        path,
        components: [{ component, path: null, transform: IDENTITY }],
        filled: true,
        showLabels: true,
      },
      FALLBACK_GLYPH_THEME,
    )
    expect(recording.calls('fill')[0]?.args).toEqual([path, 'nonzero'])
    expect(recording.calls('fillText')[0]?.args[0]).toBe('a')
  })
})

describe('drawAnchors', () => {
  it('draws a crosshair, a ring and the name for each anchor', () => {
    const recording = createRecordingContext()
    drawAnchors(
      recording.ctx,
      viewportOf(),
      { anchors: [{ name: 'top', x: 300, y: 700 }], showLabels: true },
      FALLBACK_GLYPH_THEME,
    )
    expect(recording.calls('arc')).toHaveLength(1)
    expect(recording.calls('fillText')[0]?.args[0]).toBe('top')
    expect(recording.styleAt('arc', 'strokeStyle')).toBe(
      FALLBACK_GLYPH_THEME.anchor,
    )
  })

  it('draws nothing when the layer has no anchors', () => {
    const recording = createRecordingContext()
    drawAnchors(
      recording.ctx,
      viewportOf(),
      { anchors: [], showLabels: true },
      FALLBACK_GLYPH_THEME,
    )
    expect(recording.ops).toHaveLength(0)
  })
})

describe('drawGuides', () => {
  const guide = (angle: number, name: string | null = null) => ({
    guide: { x: 100, y: 200, angle, name, locked: false, color: null },
    scope: 'layer' as const,
  })

  it('draws a horizontal guide as a horizontal screen line', () => {
    const recording = createRecordingContext()
    drawGuides(
      recording.ctx,
      viewportOf(),
      { guides: [guide(0)], showLabels: false },
      FALLBACK_GLYPH_THEME,
    )
    const from = recording.calls('moveTo')[0]?.args
    const to = recording.calls('lineTo')[0]?.args
    expect(from?.[1]).toBe(to?.[1])
    expect(recording.styleAt('stroke', 'strokeStyle')).toBe(
      FALLBACK_GLYPH_THEME.guideLocal,
    )
  })

  it('uses the global colour for a master guide and prints its name', () => {
    const recording = createRecordingContext()
    drawGuides(
      recording.ctx,
      viewportOf(),
      {
        guides: [{ ...guide(90, 'stem'), scope: 'master' }],
        showLabels: true,
      },
      FALLBACK_GLYPH_THEME,
    )
    expect(recording.styleAt('stroke', 'strokeStyle')).toBe(
      FALLBACK_GLYPH_THEME.guideGlobal,
    )
    expect(recording.calls('fillText')[0]?.args[0]).toBe('stem')
    // A 90° guide is vertical on screen.
    const from = recording.calls('moveTo')[0]?.args
    const to = recording.calls('lineTo')[0]?.args
    expect(from?.[0]).toBeCloseTo(Number(to?.[0]), 6)
  })
})

describe('drawRulers', () => {
  it('chooses a 1 / 2 / 5 step that keeps labels apart', () => {
    expect(tickStep(1, 50)).toBe(50)
    expect(tickStep(0.5, 50)).toBe(100)
    expect(tickStep(0.06, 60)).toBe(1000)
    expect(tickStep(10, 50)).toBe(5)
    expect(tickStep(0, 50)).toBe(1)
  })

  it('paints both strips, ticks them and marks the pointer', () => {
    const recording = createRecordingContext()
    drawRulers(
      recording.ctx,
      viewportOf(),
      { pointer: { x: 300, y: 400 } },
      FALLBACK_GLYPH_THEME,
    )
    const fills = recording.calls('fillRect').map((op) => op.args)
    expect(fills[0]).toEqual([0, 0, 800, RULER_SIZE])
    expect(fills[1]).toEqual([0, 0, RULER_SIZE, 600])
    expect(recording.calls('fillText').length).toBeGreaterThan(0)
    // Two extra ticks for the pointer crosshair.
    expect(recording.calls('moveTo').some((op) => op.args[0] === 400.5)).toBe(
      true,
    )
  })

  it('draws nothing for a zero-sized canvas', () => {
    const empty = new Viewport()
    empty.setSize(0, 0, 1)
    const recording = createRecordingContext()
    drawRulers(recording.ctx, empty, { pointer: null }, FALLBACK_GLYPH_THEME)
    expect(recording.ops).toHaveLength(0)
  })
})
