import { afterEach, describe, expect, it } from 'vitest'

import { setTransport, type ApplyParams } from '../api'
import { Viewport } from '../canvas/Viewport'
import {
  makeContour,
  makeLayer,
  makeLayerOutline,
  makePoint,
} from '../test/fontFixtures'
import { createMockTransport, type MockTransport } from '../test/mockTransport'

import {
  EditorController,
  type EditorLabels,
  type EditorSource,
  type PointerInput,
} from './editorController'
import { NO_MODIFIERS, type Modifiers } from './gestures/drag'

const layer = makeLayer({
  width: 600,
  contours: [
    makeContour([
      makePoint(0, 0),
      makePoint(200, 0, 'Line'),
      makePoint(200, 200, 'Line'),
      makePoint(0, 200, 'Line'),
    ]),
  ],
  anchors: [{ name: 'top', x: 100, y: 260 }],
})

const labels: EditorLabels = {
  movePoints: (count) => `Move ${count} points`,
  moveHandle: 'Move handle',
  reshapeSegment: 'Reshape segment',
  moveAnchor: 'Move anchor',
  moveComponent: 'Move component',
  nudgePoints: (count) => `Nudge ${count} points`,
  toggleSmooth: 'Toggle smooth',
  deletePoints: 'Delete points',
  insertPoint: 'Insert point',
  convertSegment: 'Convert segment',
  metrics: {
    baseline: 'baseline',
    xHeight: 'x-height',
    capHeight: 'cap height',
    ascender: 'ascender',
    descender: 'descender',
  },
  hud: {
    deltaX: 'Δx',
    deltaY: 'Δy',
    angle: '∠',
    length: 'ℓ',
    width: 'w',
    height: 'h',
    stem: '⟂',
  },
}

function source(): EditorSource {
  return {
    doc: 1,
    gid: 42,
    glyph: 'a',
    outline: makeLayerOutline(layer),
    guides: [],
    metrics: {
      upem: 1000,
      ascender: 800,
      descender: -200,
      xHeight: 500,
      capHeight: 700,
      italicAngle: 0,
      width: 600,
      lsb: 0,
      rsb: 400,
      verticalOrigin: null,
      verticalWidth: null,
    },
    master: 'm1',
  }
}

/** 1 CSS pixel per font unit, font origin at canvas (0, 300). */
function makeController(): {
  controller: EditorController
  transport: MockTransport
} {
  const transport = createMockTransport({
    'doc.apply': () => change(),
    'doc.edit_begin': () => 5,
    'doc.edit_apply': () => change(),
    'doc.edit_end': () => null,
    'doc.undo': () => null,
  })
  setTransport(transport)
  const viewport = new Viewport()
  viewport.setSize(400, 400, 1)
  viewport.scale = 1
  viewport.offset = { x: 0, y: 300 }
  const controller = new EditorController(viewport, labels)
  controller.setSource(source())
  return { controller, transport }
}

function change() {
  return {
    glyphs: [42],
    font_info: false,
    masters: false,
    glyph_order: false,
    metrics: false,
    layers: false,
    guides: false,
  }
}

function at(
  x: number,
  y: number,
  modifiers: Modifiers = NO_MODIFIERS,
): PointerInput {
  return { x, y, button: 0, modifiers }
}

/** Waits for the promise chain the controller kicked off in a handler. */
async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0))
  await new Promise((resolve) => setTimeout(resolve, 0))
}

afterEach(() => setTransport(undefined))

describe('EditorController selection', () => {
  it('selects the node under the pointer and clears on empty canvas', () => {
    const { controller } = makeController()
    // Font (200, 0) → canvas (200, 300).
    controller.pointerDown(at(200, 300))
    controller.pointerUp(at(200, 300))
    expect([...controller.currentSelection.keys()]).toEqual(['node:0:1'])

    controller.pointerDown(at(380, 20))
    controller.pointerUp(at(380, 20))
    expect(controller.currentSelection.size).toBe(0)
  })

  it('extends with Shift and toggles with Ctrl', () => {
    const { controller } = makeController()
    controller.pointerDown(at(200, 300))
    controller.pointerUp(at(200, 300))
    controller.pointerDown(at(200, 100, { ...NO_MODIFIERS, shift: true }))
    controller.pointerUp(at(200, 100, { ...NO_MODIFIERS, shift: true }))
    expect([...controller.currentSelection.keys()].sort()).toEqual([
      'node:0:1',
      'node:0:2',
    ])

    controller.pointerDown(at(200, 300, { ...NO_MODIFIERS, mod: true }))
    controller.pointerUp(at(200, 300, { ...NO_MODIFIERS, mod: true }))
    expect([...controller.currentSelection.keys()]).toEqual(['node:0:2'])
  })

  it('selects with a marquee and selects every point with select-all', () => {
    const { controller } = makeController()
    controller.pointerDown(at(-10, 290))
    controller.pointerMove(at(210, 310))
    controller.pointerUp(at(210, 310))
    expect([...controller.currentSelection.keys()].sort()).toEqual([
      'node:0:0',
      'node:0:1',
    ])

    controller.selectAll()
    expect(controller.currentSelection.size).toBe(4)
  })

  it('walks the nodes with Tab', () => {
    const { controller } = makeController()
    controller.cycleNodes(1)
    expect([...controller.currentSelection.keys()]).toEqual(['node:0:0'])
    controller.cycleNodes(1)
    expect([...controller.currentSelection.keys()]).toEqual(['node:0:1'])
    controller.cycleNodes(-1)
    expect([...controller.currentSelection.keys()]).toEqual(['node:0:0'])
  })
})

describe('EditorController commands', () => {
  it('toggles smooth on the selected nodes with one SetSmooth', async () => {
    const { controller, transport } = makeController()
    controller.pointerDown(at(200, 300))
    controller.pointerUp(at(200, 300))
    await controller.toggleSmooth(false)

    const applies = transport.calls.filter(
      (entry) => entry.method === 'doc.apply',
    )
    expect(applies).toHaveLength(1)
    const params = applies[0]?.params as ApplyParams
    expect(params.label).toBe('Toggle smooth')
    expect(params.command).toEqual({
      SetSmooth: {
        glyph: 'a',
        layer: 'layer-1',
        points: [{ contour: 0, point: 1 }],
        smooth: true,
        level_handles: true,
      },
    })
  })

  it('double-click on a node toggles it too', async () => {
    const { controller, transport } = makeController()
    controller.doubleClick(at(200, 300))
    await settle()
    expect(
      transport.calls.filter((entry) => entry.method === 'doc.apply'),
    ).toHaveLength(1)
  })

  it('deletes the selection, keeping the shape unless Shift breaks it', async () => {
    const { controller, transport } = makeController()
    controller.selectAll()
    await controller.deleteSelection(false)
    const params = transport.calls.find((entry) => entry.method === 'doc.apply')
      ?.params as ApplyParams
    expect(params.command).toMatchObject({
      DeletePoints: { mode: 'KeepShape' },
    })
    expect(controller.currentSelection.size).toBe(0)
  })

  it('Alt-clicking a line segment converts it to a curve', async () => {
    const { controller, transport } = makeController()
    // The bottom edge runs along font y = 0 → canvas y = 300.
    controller.pointerDown(at(100, 300, { ...NO_MODIFIERS, alt: true }))
    await settle()
    const params = transport.calls.find((entry) => entry.method === 'doc.apply')
      ?.params as ApplyParams
    expect(params.command).toEqual({
      ConvertSegment: {
        glyph: 'a',
        layer: 'layer-1',
        contour: 0,
        segment: 0,
        to: 'Curve',
      },
    })
    expect(params.label).toBe('Convert segment')
  })

  it('Shift-clicking a segment inserts a node on it', async () => {
    const { controller, transport } = makeController()
    controller.pointerDown(at(100, 300, { ...NO_MODIFIERS, shift: true }))
    await settle()
    const params = transport.calls.find((entry) => entry.method === 'doc.apply')
      ?.params as ApplyParams
    expect(params.command).toMatchObject({
      InsertPointOnSegment: { contour: 0, segment: 0 },
    })
  })
})

describe('EditorController dragging', () => {
  it('runs the gesture protocol and reports one undo label', async () => {
    const { controller, transport } = makeController()
    controller.pointerDown(at(200, 300))
    controller.pointerMove(at(230, 300))
    controller.pointerUp(at(230, 300))
    await settle()

    expect(
      transport.calls
        .map((entry) => entry.method)
        .filter((method) => method !== 'doc.apply'),
    ).toEqual(['doc.edit_begin', 'doc.edit_apply', 'doc.edit_end'])
    expect(transport.calls[0]?.params).toMatchObject({
      label: 'Move 1 points',
    })
  })

  it('previews the move before the kernel answers', () => {
    const { controller } = makeController()
    controller.pointerDown(at(200, 300))
    controller.pointerMove(at(230, 280))
    const frame = controller.frame()
    expect(frame.contours?.[0]?.points[1]).toMatchObject({ x: 230, y: 20 })
    expect(frame.hud).toContain('Δx 30')
  })
})
