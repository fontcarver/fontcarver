import { afterEach, describe, expect, it, vi } from 'vitest'

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
import { NUDGE_TIMEOUT } from './gestures/nudge'

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
  snap: {
    node: 'node',
    handle: 'handle',
    guide: 'guide',
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

/** A layer whose bottom span is a cubic, for the segment cases. */
const curvedLayer = makeLayer({
  width: 600,
  contours: [
    makeContour([
      makePoint(0, 0),
      makePoint(60, 120, 'OffCurve'),
      makePoint(140, 120, 'OffCurve'),
      makePoint(200, 0, 'Curve'),
      makePoint(200, 200, 'Line'),
      makePoint(0, 200, 'Line'),
    ]),
  ],
})

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

/** What the view would have been told. */
interface Recorded {
  selections: string[][]
  editing: boolean[]
  reloads: number
  errors: unknown[]
}

/** 1 CSS pixel per font unit, font origin at canvas (0, 300). */
function makeController(): {
  controller: EditorController
  transport: MockTransport
  recorded: Recorded
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
  const recorded: Recorded = {
    selections: [],
    editing: [],
    reloads: 0,
    errors: [],
  }
  controller.setCallbacks({
    repaint: () => undefined,
    onSelectionChange: (selection) =>
      recorded.selections.push([...selection.keys()]),
    onEditingChange: (editing) => recorded.editing.push(editing),
    onZoomChange: () => undefined,
    onReload: () => {
      recorded.reloads += 1
    },
    onCursorChange: () => undefined,
    onError: (error) => recorded.errors.push(error),
  })
  controller.setSource(source())
  return { controller, transport, recorded }
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

describe('EditorController modifier grammar', () => {
  it('does not toggle the selection when Ctrl+Alt starts a mirror drag', () => {
    const { controller } = makeController()
    controller.pointerDown(at(200, 300))
    controller.pointerUp(at(200, 300))
    expect([...controller.currentSelection.keys()]).toEqual(['node:0:1'])

    const mirror = { ...NO_MODIFIERS, mod: true, alt: true }
    controller.pointerDown(at(200, 300, mirror))
    // Ctrl alone would have removed it; Ctrl+Alt is a drag modifier.
    expect([...controller.currentSelection.keys()]).toEqual(['node:0:1'])
  })

  it('keeps Alt from hiding handles under the pointer', () => {
    const { controller } = makeController()
    const alt = { ...NO_MODIFIERS, alt: true }
    // An Alt-drag of a node still grabs the node it is pointing at.
    controller.pointerDown(at(200, 300, alt))
    expect([...controller.currentSelection.keys()]).toEqual(['node:0:1'])
  })
})

describe('EditorController nudging', () => {
  it('shows the move on the first key press, before the kernel answers', () => {
    const { controller } = makeController()
    controller.setSelection(
      new Map([['node:0:1', { kind: 'node', contour: 0, point: 1 }]]),
    )

    controller.nudgeSelection({ x: 1, y: 0 }, NO_MODIFIERS)
    // The optimistic preview carries the nudge exactly as a drag does.
    expect(controller.frame().contours?.[0]?.points[1]).toMatchObject({
      x: 201,
      y: 0,
    })
    controller.nudgeSelection({ x: 1, y: 0 }, { ...NO_MODIFIERS, shift: true })
    expect(controller.frame().contours?.[0]?.points[1]).toMatchObject({
      x: 211,
      y: 0,
    })
  })

  it('ends the run on its own timer and lets the view catch up', async () => {
    vi.useFakeTimers()
    try {
      const { controller, transport, recorded } = makeController()
      controller.selectAll()
      controller.nudgeSelection({ x: 0, y: 1 }, NO_MODIFIERS)
      await vi.advanceTimersByTimeAsync(0)
      expect(recorded.editing.at(-1)).toBe(true)
      const reloadsDuring = recorded.reloads

      await vi.advanceTimersByTimeAsync(NUDGE_TIMEOUT + 50)

      expect(
        transport.calls.filter((entry) => entry.method === 'doc.edit_end'),
      ).toHaveLength(1)
      expect(recorded.editing.at(-1)).toBe(false)
      expect(recorded.reloads).toBeGreaterThan(reloadsDuring)
      // The preview is handed back to the refetched outline.
      expect(controller.frame().contours).toBeNull()
    } finally {
      vi.useRealTimers()
    }
  })

  it('closes a nudge run before the next gesture starts', async () => {
    vi.useFakeTimers()
    try {
      const { controller, transport } = makeController()
      controller.selectAll()
      controller.nudgeSelection({ x: 0, y: -1 }, NO_MODIFIERS)
      await vi.advanceTimersByTimeAsync(0)
      // Any other action closes the run rather than leaving it to the timer.
      controller.pointerDown(at(380, 20))
      await vi.advanceTimersByTimeAsync(0)

      expect(transport.calls.map((entry) => entry.method)).toEqual([
        'doc.edit_begin',
        'doc.edit_apply',
        'doc.edit_end',
      ])
    } finally {
      vi.useRealTimers()
    }
  })
})

describe('EditorController error handling', () => {
  function failing(method: 'doc.edit_apply' | 'doc.apply'): MockTransport {
    const mock = createMockTransport({
      'doc.edit_begin': () => 5,
      'doc.edit_apply': () => {
        if (method === 'doc.edit_apply') throw new Error('kernel says no')
        return change()
      },
      'doc.edit_end': () => null,
      'doc.apply': () => {
        if (method === 'doc.apply') throw new Error('kernel says no')
        return change()
      },
      'doc.undo': () => null,
    })
    setTransport(mock)
    return mock
  }

  it('recovers the view when an apply fails mid-drag', async () => {
    const { controller, recorded } = makeController()
    failing('doc.edit_apply')

    controller.pointerDown(at(200, 300))
    controller.pointerMove(at(230, 300))
    controller.pointerUp(at(230, 300))
    await settle()

    expect(recorded.editing.at(-1)).toBe(false)
    expect(controller.frame().contours).toBeNull()
    expect(recorded.reloads).toBeGreaterThan(0)
    expect(String(recorded.errors[0])).toContain('kernel says no')
  })

  it('reports a failed single-shot command and still reloads', async () => {
    const { controller, recorded } = makeController()
    failing('doc.apply')

    controller.selectAll()
    await controller.toggleSmooth(false)
    await settle()

    expect(String(recorded.errors[0])).toContain('kernel says no')
    expect(recorded.reloads).toBeGreaterThan(0)
  })
})

describe('EditorController glyph switching', () => {
  it('publishes the empty selection when another glyph arrives', () => {
    const { controller, recorded } = makeController()
    controller.pointerDown(at(200, 300))
    controller.pointerUp(at(200, 300))
    expect(recorded.selections.at(-1)).toEqual(['node:0:1'])

    // A freshly mounted controller starts empty, so "the selection did not
    // change" is true locally while the store still holds the old tab's.
    controller.setSource({ ...source(), gid: 43, glyph: 'b' })

    expect(recorded.selections.at(-1)).toEqual([])
    expect(controller.currentSelection.size).toBe(0)
  })

  it('publishes it even when this controller never had a selection', () => {
    const { controller, recorded } = makeController()
    controller.setSource({ ...source(), gid: 44, glyph: 'c' })
    expect(recorded.selections.at(-1)).toEqual([])
  })
})

describe('EditorController segment grammar', () => {
  function curved() {
    const made = makeController()
    made.controller.setSource({
      ...source(),
      outline: makeLayerOutline(curvedLayer),
    })
    return made
  }

  it('converts a line segment to a curve on Alt-click', async () => {
    const { controller, transport } = curved()
    // The straight right edge runs from font (200, 0) to (200, 200).
    controller.pointerDown(at(200, 200, { ...NO_MODIFIERS, alt: true }))
    await settle()

    const params = transport.calls.find((entry) => entry.method === 'doc.apply')
      ?.params as ApplyParams
    expect(params.command).toMatchObject({ ConvertSegment: { to: 'Curve' } })
  })

  it('leaves a curve alone on Alt and lets the drag through', async () => {
    const { controller, transport } = curved()
    // The cubic bulges up to y ≈ 90 between (0, 0) and (200, 0).
    controller.pointerDown(at(100, 210, { ...NO_MODIFIERS, alt: true }))
    await settle()

    // No command: converting a curve to a curve is a no-op that would still
    // record an undo entry and dirty the document.
    expect(
      transport.calls.filter((entry) => entry.method === 'doc.apply'),
    ).toHaveLength(0)
    expect(controller.dragging).toBe(true)
  })

  it('reshapes a curve segment on a plain drag', () => {
    const { controller } = curved()
    controller.pointerDown(at(100, 210))
    controller.pointerMove(at(100, 190))
    const contour = controller.frame().contours?.[0]
    // Both controls of the segment move with the pointer.
    expect(contour?.points[1]).toMatchObject({ x: 60, y: 140 })
    expect(contour?.points[2]).toMatchObject({ x: 140, y: 140 })
  })
})
