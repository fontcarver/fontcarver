import { afterEach, describe, expect, it } from 'vitest'

import { setTransport, type EditApplyParams } from '../../api'
import { Viewport } from '../../canvas/Viewport'
import { immediateScheduler } from '../../editor/editSession'
import {
  createEditingTransport,
  type MockTransport,
} from '../../test/mockTransport'

import type { SceneMetrics } from './glyphScene'
import {
  METRIC_HIT,
  MetricsDrag,
  metricLineX,
  metricValue,
  metricsHit,
} from './metricsHandles'
import { RulerGuideDrag, rulerAt, shouldStartRulerGuide } from './rulerGuides'

const metrics: SceneMetrics = {
  upem: 1000,
  ascender: 800,
  descender: -200,
  xHeight: 500,
  capHeight: 700,
  italicAngle: 0,
  width: 600,
  lsb: 40,
  rsb: 60,
  verticalOrigin: null,
  verticalWidth: null,
}

/** One CSS pixel per font unit, origin at canvas x = 100. */
function viewportOf(): Viewport {
  const viewport = new Viewport()
  viewport.setSize(800, 600, 1)
  viewport.scale = 1
  viewport.offset = { x: 100, y: 500 }
  return viewport
}

afterEach(() => setTransport(undefined))

describe('metric handles', () => {
  it('puts each handle on the line the canvas draws', () => {
    expect(metricLineX(metrics, 'lsb')).toBe(40)
    // The RSB line sits inside the advance, at width − rsb.
    expect(metricLineX(metrics, 'rsb')).toBe(540)
    expect(metricLineX(metrics, 'width')).toBe(600)
  })

  it('finds a handle within the grab radius and nothing outside it', () => {
    const viewport = viewportOf()
    expect(metricsHit(metrics, viewport, { x: 140, y: 300 })).toBe('lsb')
    expect(metricsHit(metrics, viewport, { x: 140 + METRIC_HIT, y: 300 })).toBe(
      'lsb',
    )
    expect(
      metricsHit(metrics, viewport, { x: 140 + METRIC_HIT + 1, y: 300 }),
    ).toBeNull()
    expect(metricsHit(metrics, viewport, { x: 700, y: 300 })).toBe('width')
  })

  it('prefers the sidebearing when it coincides with the advance', () => {
    const flush = { ...metrics, rsb: 0 }
    expect(metricsHit(flush, viewportOf(), { x: 700, y: 300 })).toBe('rsb')
  })

  it('reads the right sidebearing in the opposite sense', () => {
    const start = { width: 600, lsb: 40, rsb: 60 }
    expect(metricValue('lsb', start, 25, 1)).toBe(65)
    expect(metricValue('width', start, 25, 1)).toBe(625)
    // Dragging the RSB line right *reduces* the right sidebearing.
    expect(metricValue('rsb', start, 25, 1)).toBe(35)
  })

  it('rounds to whole units, and to ten with the coarse step', () => {
    const start = { width: 600, lsb: 40, rsb: 60 }
    expect(metricValue('lsb', start, 12.4, 1)).toBe(52)
    expect(metricValue('lsb', start, 12.4, 10)).toBe(50)
  })
})

describe('MetricsDrag', () => {
  function drag(handle: 'lsb' | 'rsb' | 'width'): {
    gesture: MetricsDrag
    mock: MockTransport
  } {
    const mock = createEditingTransport()
    setTransport(mock)
    return {
      mock,
      gesture: new MetricsDrag({
        doc: 1,
        glyph: 'A',
        layer: 'layer-1',
        handle,
        origin: 40,
        start: { width: 600, lsb: 40, rsb: 60 },
        label: 'Set metrics',
        scheduler: immediateScheduler,
      }),
    }
  }

  function applies(mock: MockTransport): EditApplyParams[] {
    return mock.calls
      .filter((entry) => entry.method === 'doc.edit_apply')
      .map((entry) => entry.params as EditApplyParams)
  }

  it('sends the absolute value, once per change, as one undo entry', async () => {
    const { gesture, mock } = drag('lsb')
    gesture.move(90, false)
    gesture.move(90, false)
    gesture.move(120, false)
    expect(await gesture.finish()).toBe(true)

    expect(mock.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'doc.edit_apply',
      'doc.edit_apply',
      'doc.edit_end',
    ])
    expect(applies(mock).map((entry) => entry.command)).toEqual([
      {
        SetMetrics: {
          glyph: 'A',
          layer: 'layer-1',
          width: null,
          lsb: 90,
          rsb: null,
        },
      },
      {
        SetMetrics: {
          glyph: 'A',
          layer: 'layer-1',
          width: null,
          lsb: 120,
          rsb: null,
        },
      },
    ])
  })

  it('carries the advance with a left sidebearing in the preview', () => {
    const { gesture } = drag('lsb')
    gesture.move(90, false)
    expect(gesture.preview).toEqual({ width: 650, lsb: 90, rsb: 60 })
  })

  it('carries the advance with a right sidebearing too', () => {
    const { gesture } = drag('rsb')
    gesture.move(90, false)
    // The line moved 50 units right, so the RSB is 10 and the advance 550.
    expect(gesture.preview).toEqual({ width: 550, lsb: 40, rsb: 10 })
  })

  it('applies nothing for a gesture that never moved', async () => {
    const { gesture, mock } = drag('width')
    expect(await gesture.finish()).toBe(false)
    expect(applies(mock)).toHaveLength(0)
  })

  it('undoes what it applied when it is cancelled', async () => {
    const { gesture, mock } = drag('width')
    gesture.move(120, false)
    await gesture.cancel()

    expect(mock.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'doc.edit_apply',
      'doc.edit_end',
      'doc.undo',
    ])
  })
})

describe('ruler guides', () => {
  it('names the ruler a point is inside, and the corner as neither', () => {
    expect(rulerAt({ x: 200, y: 5 })).toBe('horizontal')
    expect(rulerAt({ x: 5, y: 200 })).toBe('vertical')
    expect(rulerAt({ x: 5, y: 5 })).toBeNull()
    expect(rulerAt({ x: 200, y: 200 })).toBeNull()
  })

  it('starts only with Alt held and the rulers on screen', () => {
    expect(shouldStartRulerGuide({ x: 200, y: 5 }, { alt: true }, true)).toBe(
      'horizontal',
    )
    expect(
      shouldStartRulerGuide({ x: 200, y: 5 }, { alt: false }, true),
    ).toBeNull()
    expect(
      shouldStartRulerGuide({ x: 200, y: 5 }, { alt: true }, false),
    ).toBeNull()
  })

  it('lays a horizontal guide in the layer and a vertical one in the master', () => {
    const local = new RulerGuideDrag(
      {
        doc: 1,
        glyph: 'A',
        layer: 'layer-1',
        master: 'm1',
        scope: 'layer',
        axis: 'horizontal',
        label: 'Add guide',
      },
      { x: 0, y: 0 },
    )
    local.move({ x: 123.4, y: 456.7 })
    expect(local.command()).toEqual({
      AddGuide: {
        glyph: 'A',
        layer: 'layer-1',
        master: null,
        guide: {
          x: 0,
          y: 457,
          angle: 0,
          name: null,
          locked: false,
          color: null,
        },
      },
    })
    expect(local.previewY).toBe(456.7)
    expect(local.previewX).toBeNull()

    const global = new RulerGuideDrag(
      {
        doc: 1,
        glyph: 'A',
        layer: 'layer-1',
        master: 'm1',
        scope: 'master',
        axis: 'vertical',
        label: 'Add guide',
      },
      { x: 10, y: 0 },
    )
    expect(global.command()).toEqual({
      AddGuide: {
        glyph: null,
        layer: null,
        master: 'm1',
        guide: {
          x: 10,
          y: 0,
          angle: 90,
          name: null,
          locked: false,
          color: null,
        },
      },
    })
  })
})
