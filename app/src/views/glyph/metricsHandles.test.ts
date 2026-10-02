import { afterEach, describe, expect, it } from 'vitest'

import { setTransport, type EditApplyParams } from '../../api'
import { Viewport } from '../../canvas/Viewport'
import { immediateScheduler } from '../../editor/editSession'
import {
  createEditingTransport,
  type MockTransport,
} from '../../test/mockTransport'
import {
  makeContour,
  makeLayer,
  makeLayerOutline,
  makePoint,
} from '../../test/fontFixtures'

import type { SceneMetrics } from './glyphScene'
import {
  METRIC_HIT,
  MetricsDrag,
  metricCursor,
  metricHandleAxis,
  metricLineX,
  metricLineY,
  metricValue,
  metricsHit,
  withPreview,
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
        outline: makeLayerOutline(makeLayer()),
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

  it('previews the kernel-equivalent LSB translation for all geometry', () => {
    setTransport(createEditingTransport())
    const sourceLayer = makeLayer({
      contours: [makeContour([makePoint(40, 0), makePoint(140, 0)])],
      anchors: [{ name: 'top', x: 90, y: 200 }],
    })
    const outline = makeLayerOutline(sourceLayer, {
      lsb: 40,
      components: [
        {
          component: {
            base: 'acute',
            transform: [0, 2, -3, 0, 70, 80],
            anchor: null,
            auto_align: false,
            use_my_metrics: false,
            round_to_grid: false,
            smart: null,
            point_match: null,
          },
          path: 'M0 0L10 0L0 10Z',
          bbox: [50, 60, 90, 120],
        },
      ],
    })
    const gesture = new MetricsDrag({
      doc: 1,
      glyph: 'A',
      layer: sourceLayer.id,
      handle: 'lsb',
      origin: 40,
      start: { width: 600, lsb: 40, rsb: 460 },
      outline,
      label: 'Set metrics',
      scheduler: immediateScheduler,
    })
    gesture.move(65, false)

    expect(gesture.geometryPreview.contours[0]?.points[0]?.x).toBe(65)
    expect(gesture.geometryPreview.anchors[0]?.x).toBe(115)
    expect(gesture.geometryPreview.components[0]?.component.transform).toEqual([
      0, 2, -3, 0, 95, 80,
    ])
    expect(gesture.geometryPreview.components[0]?.bbox).toEqual([
      75, 60, 115, 120,
    ])
  })

  it('carries the advance with a right sidebearing too', () => {
    const { gesture } = drag('rsb')
    gesture.move(90, false)
    // The line moved 50 units right, so the RSB is 10 and the advance 550.
    expect(gesture.preview).toEqual({ width: 550, lsb: 40, rsb: 10 })
  })

  it.each([
    { pointer: 680, coarse: false, width: 680, rsb: 140 },
    { pointer: 360, coarse: false, width: 360, rsb: -180 },
    { pointer: -40, coarse: false, width: -40, rsb: -580 },
    { pointer: 612.4, coarse: true, width: 610, rsb: 70 },
  ])(
    'keeps the ink fixed while width changes to $width units',
    async ({ pointer, coarse, width, rsb }) => {
      const mock = createEditingTransport()
      setTransport(mock)
      const layer = makeLayer({
        width: 600,
        contours: [
          makeContour([
            makePoint(40, 0),
            makePoint(540, 0),
            makePoint(540, 700),
            makePoint(40, 700),
          ]),
        ],
        anchors: [{ name: 'top', x: 290, y: 700 }],
      })
      const outline = makeLayerOutline(layer)
      const gesture = new MetricsDrag({
        doc: 1,
        glyph: 'A',
        layer: layer.id,
        handle: 'width',
        origin: 600,
        start: { width: 600, lsb: 40, rsb: 60 },
        outline,
        label: 'Set advance',
        scheduler: immediateScheduler,
      })
      gesture.move(pointer, coarse)

      const preview = withPreview(metrics, gesture.preview)
      expect(await gesture.finish()).toBe(true)
      expect(preview).toMatchObject({ width, lsb: 40, rsb })
      expect(metricLineX(preview, 'rsb')).toBe(540)
      expect(gesture.geometryPreview.contours).toEqual(outline.contours)
      expect(gesture.geometryPreview.anchors).toEqual(outline.anchors)
      expect(applies(mock).map((entry) => entry.command)).toEqual([
        {
          SetMetrics: {
            glyph: 'A',
            layer: layer.id,
            width,
            lsb: null,
            rsb: null,
          },
        },
      ])
    },
  )

  it('applies nothing for a gesture that never moved', async () => {
    const { gesture, mock } = drag('width')
    expect(await gesture.finish()).toBe(false)
    expect(applies(mock)).toHaveLength(0)
  })

  it('cancels only its edit token after applying a metrics change', async () => {
    const { gesture, mock } = drag('width')
    gesture.move(120, false)
    expect(gesture.preview).toEqual({ width: 680, lsb: 40, rsb: 140 })
    await gesture.cancel()

    expect(mock.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'doc.edit_apply',
      'doc.edit_cancel',
    ])
    expect(applies(mock)).toEqual([
      {
        doc: 1,
        token: 101,
        command: {
          SetMetrics: {
            glyph: 'A',
            layer: 'layer-1',
            width: 680,
            lsb: null,
            rsb: null,
          },
        },
      },
    ])
    expect(mock.calls[2]?.params).toEqual({ doc: 1, token: 101 })
  })
})

/** The same font with the vertical metrics a CJK face carries. */
const cjkMetrics: SceneMetrics = {
  ...metrics,
  verticalOrigin: 880,
  verticalWidth: 1000,
}

describe('vertical metric handles', () => {
  it('puts each horizontal handle on the line the canvas draws', () => {
    // `drawMetrics` draws the origin at its own y and the advance one advance
    // height below it; the handles must agree exactly.
    expect(metricLineY(cjkMetrics, 'vorigin')).toBe(880)
    expect(metricLineY(cjkMetrics, 'vadvance')).toBe(-120)
    // A font with no vertical metrics draws neither line and offers neither.
    expect(metricLineY(metrics, 'vorigin')).toBeNull()
    expect(metricLineY(metrics, 'vadvance')).toBeNull()
    expect(
      metricLineY({ ...cjkMetrics, verticalWidth: null }, 'vadvance'),
    ).toBeNull()
    expect(metricLineX(cjkMetrics, 'vorigin')).toBeNull()
  })

  it('names the axis and the cursor of every handle', () => {
    expect(metricHandleAxis('lsb')).toBe('x')
    expect(metricHandleAxis('width')).toBe('x')
    expect(metricHandleAxis('vorigin')).toBe('y')
    expect(metricHandleAxis('vadvance')).toBe('y')
    expect(metricCursor('rsb')).toBe('ew-resize')
    expect(metricCursor('vadvance')).toBe('ns-resize')
  })

  it('hits a horizontal line only where no vertical line is grabbed', () => {
    const viewport = viewportOf()
    // y = 880 font units maps to canvas y 500 − 880 = −380, off screen; use the
    // advance line at −120, which maps to 620… also off screen, so move the
    // viewport to frame them.
    viewport.offset = { x: 100, y: 1000 }
    expect(metricsHit(cjkMetrics, viewport, { x: 400, y: 120 }, true)).toBe(
      'vorigin',
    )
    expect(
      metricsHit(cjkMetrics, viewport, { x: 400, y: 120 + METRIC_HIT }, true),
    ).toBe('vorigin')
    expect(
      metricsHit(
        cjkMetrics,
        viewport,
        { x: 400, y: 120 + METRIC_HIT + 1 },
        true,
      ),
    ).toBeNull()
    expect(metricsHit(cjkMetrics, viewport, { x: 400, y: 1120 }, true)).toBe(
      'vadvance',
    )
    // Where the origin line crosses the advance vertical, the vertical wins, so
    // adding these handles cannot take a press away from the existing gesture.
    expect(metricsHit(cjkMetrics, viewport, { x: 700, y: 120 }, true)).toBe(
      'width',
    )
    // And a font without vertical metrics is completely unaffected.
    expect(metricsHit(metrics, viewport, { x: 400, y: 120 }, true)).toBeNull()
  })

  it('offers the horizontal lines only in vertical writing mode', () => {
    const viewport = viewportOf()
    viewport.offset = { x: 100, y: 1000 }
    // A CJK font gives every glyph these two lines through the master's default
    // vertical origin. Out of vertical mode they must not take a press, because
    // they are full-width bands that outrank segments and components.
    for (const point of [
      { x: 400, y: 120 },
      { x: 400, y: 1120 },
    ]) {
      expect(metricsHit(cjkMetrics, viewport, point, true)).not.toBeNull()
      expect(metricsHit(cjkMetrics, viewport, point, false)).toBeNull()
      // The default is "off", so a caller that forgets the flag is safe.
      expect(metricsHit(cjkMetrics, viewport, point)).toBeNull()
    }
    // The sidebearing and advance handles are unchanged by the mode.
    for (const mode of [true, false]) {
      expect(metricsHit(cjkMetrics, viewport, { x: 140, y: 300 }, mode)).toBe(
        'lsb',
      )
      expect(metricsHit(cjkMetrics, viewport, { x: 700, y: 300 }, mode)).toBe(
        'width',
      )
    }
  })

  it('reads the advance in the opposite sense and clamps both to OpenType', () => {
    const start = {
      width: 600,
      lsb: 40,
      rsb: 60,
      verticalOrigin: 880,
      verticalWidth: 1000,
    }
    expect(metricValue('vorigin', start, 25, 1)).toBe(905)
    // Pulling the advance line *down* by 25 units lengthens the advance.
    expect(metricValue('vadvance', start, -25, 1)).toBe(1025)
    expect(metricValue('vadvance', start, 12.4, 10)).toBe(990)
    // The kernel refuses a negative advance or an origin past 16 bits, so a
    // long drag stops at the limit instead of failing.
    expect(metricValue('vadvance', start, 5000, 1)).toBe(0)
    expect(metricValue('vorigin', start, 99999, 1)).toBe(32767)
    expect(metricValue('vorigin', start, -99999, 1)).toBe(-32768)
  })

  it('replaces only the fields a preview changed', () => {
    const horizontal = withPreview(cjkMetrics, {
      width: 650,
      lsb: 90,
      rsb: 60,
    })
    expect(horizontal.width).toBe(650)
    expect(horizontal.verticalOrigin).toBe(880)
    expect(horizontal.verticalWidth).toBe(1000)
    const vertical = withPreview(cjkMetrics, {
      width: 600,
      lsb: 40,
      rsb: 60,
      verticalOrigin: 905,
    })
    expect(vertical.verticalOrigin).toBe(905)
    expect(vertical.verticalWidth).toBe(1000)
  })
})

describe('vertical MetricsDrag', () => {
  function drag(
    handle: 'vorigin' | 'vadvance',
    stored: { origin: number | null; advance: number | null },
  ): { gesture: MetricsDrag; mock: MockTransport } {
    const mock = createEditingTransport()
    setTransport(mock)
    return {
      mock,
      gesture: new MetricsDrag({
        doc: 1,
        glyph: 'ideograph',
        layer: 'layer-1',
        handle,
        origin: handle === 'vorigin' ? 880 : -120,
        start: {
          width: 1000,
          lsb: 50,
          rsb: 50,
          verticalOrigin: 880,
          verticalWidth: 1000,
        },
        stored,
        outline: makeLayerOutline(makeLayer()),
        label: 'Vertical metrics',
        scheduler: immediateScheduler,
      }),
    }
  }

  function commands(mock: MockTransport): unknown[] {
    return mock.calls
      .filter((entry) => entry.method === 'doc.edit_apply')
      .map((entry) => (entry.params as EditApplyParams).command)
  }

  it('writes the origin and keeps an inherited advance inherited', async () => {
    const { gesture, mock } = drag('vorigin', { origin: null, advance: null })
    gesture.move(0, false, 905)
    expect(await gesture.finish()).toBe(true)
    expect(commands(mock)).toEqual([
      {
        SetVerticalMetrics: {
          glyph: 'ideograph',
          layer: 'layer-1',
          advance: null,
          origin: 905,
        },
      },
    ])
    expect(gesture.preview.verticalOrigin).toBe(905)
    expect(gesture.preview.verticalWidth).toBe(1000)
    // The outline does not move with a vertical metric edit.
    expect(gesture.geometryPreview.contours).toEqual(
      makeLayerOutline(makeLayer()).contours,
    )
  })

  it('writes the advance and carries the stored origin through', async () => {
    const { gesture, mock } = drag('vadvance', { origin: 880, advance: null })
    gesture.move(0, false, -220)
    gesture.move(0, false, -220)
    gesture.move(0, false, -320)
    expect(await gesture.finish()).toBe(true)
    expect(commands(mock)).toEqual([
      {
        SetVerticalMetrics: {
          glyph: 'ideograph',
          layer: 'layer-1',
          advance: 1100,
          origin: 880,
        },
      },
      {
        SetVerticalMetrics: {
          glyph: 'ideograph',
          layer: 'layer-1',
          advance: 1200,
          origin: 880,
        },
      },
    ])
    expect(mock.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'doc.edit_apply',
      'doc.edit_apply',
      'doc.edit_end',
    ])
  })

  it('ignores a move with no y, so a horizontal axis cannot drive it', () => {
    const { gesture, mock } = drag('vorigin', { origin: 880, advance: 1000 })
    gesture.move(905, false)
    expect(commands(mock)).toHaveLength(0)
    expect(gesture.value).toBe(880)
  })

  it('cancels only its edit token after applying a metrics change', async () => {
    const { gesture, mock } = drag('vorigin', { origin: 880, advance: 1000 })
    gesture.move(0, false, 905)
    expect(gesture.preview.verticalOrigin).toBe(905)
    expect(gesture.geometryPreview.contours).toEqual(
      makeLayerOutline(makeLayer()).contours,
    )
    await gesture.cancel()
    expect(mock.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'doc.edit_apply',
      'doc.edit_cancel',
    ])
    expect(commands(mock)).toEqual([
      {
        SetVerticalMetrics: {
          glyph: 'ideograph',
          layer: 'layer-1',
          advance: 1000,
          origin: 905,
        },
      },
    ])
    expect(mock.calls[2]?.params).toEqual({ doc: 1, token: 101 })
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
          anchor: 'Origin',
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
          anchor: 'Origin',
        },
      },
    })
  })
})
