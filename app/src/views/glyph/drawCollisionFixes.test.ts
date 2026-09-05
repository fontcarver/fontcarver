import { describe, expect, it } from 'vitest'

import { Viewport } from '../../canvas/Viewport'
import { IDENTITY } from '../../editor/affine'
import { makeCell, makeGlyphDetail, makeMaster } from '../../test/fontFixtures'
import { createRecordingContext } from '../../test/recordingContext'

import { drawComponents } from './drawComponents'
import { DEFAULT_METRIC_LABELS, drawMetrics } from './drawMetrics'
import { buildScene } from './glyphScene'
import { FALLBACK_GLYPH_THEME } from './glyphTheme'

const scene = buildScene({
  detail: makeGlyphDetail(),
  cell: makeCell(),
  master: makeMaster(),
  masterId: 'm1',
  unitsPerEm: 1000,
})

function viewportOf(): Viewport {
  const viewport = new Viewport()
  viewport.setSize(800, 600, 1)
  viewport.scale = 1
  viewport.offset = { x: 100, y: 500 }
  return viewport
}

describe('drawing collision fixes', () => {
  it('merges coincident CJK labels and distinguishes vertical metrics', () => {
    const recording = createRecordingContext()
    const viewport = new Viewport()
    viewport.setSize(800, 600, 1)
    viewport.scale = 0.5
    viewport.offset = { x: 100, y: 450 }

    drawMetrics(
      recording.ctx,
      viewport,
      {
        metrics: {
          ...scene.metrics,
          ascender: 880,
          descender: -120,
          verticalOrigin: 880,
          verticalWidth: 1000,
        },
        labels: DEFAULT_METRIC_LABELS,
        showLabels: true,
      },
      FALLBACK_GLYPH_THEME,
    )

    const labels = recording.calls('fillText').map((op) => op.args[0])
    expect(labels).toContain('ascender 880 · vert. origin 880')
    expect(labels).toContain('descender -120 · vert. advance -120')
    expect(
      recording.calls('setLineDash').map((op) => op.args[0]),
    ).toContainEqual([6, 3])
  })

  it('offsets a component label when its bounds intersect an earlier label', () => {
    const recording = createRecordingContext()
    const component = {
      base: 'base-a',
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
    drawComponents(
      recording.ctx,
      viewportOf(),
      {
        path: null,
        components: [
          { component, path: null, transform: IDENTITY },
          {
            component: { ...component, base: 'acute' },
            path: null,
            transform: IDENTITY,
          },
        ],
        filled: false,
        showLabels: true,
      },
      FALLBACK_GLYPH_THEME,
    )

    const labels = recording.calls('fillText')
    expect(labels).toHaveLength(2)
    expect(
      Math.abs(Number(labels[1]?.args[2]) - Number(labels[0]?.args[2])),
    ).toBeGreaterThanOrEqual(10)
  })
})
