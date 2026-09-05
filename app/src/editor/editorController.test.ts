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
  moveHandles: (count) => `Move ${count} handles`,
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

/** The transport the running test installed, drained when it ends. */
let installed: MockTransport | null = null

/** What the view would have been told. */
interface Recorded {
  selections: string[][]
  editing: boolean[]
  reloads: number
  errors: unknown[]
  previews: (number | null)[]
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
  installed = transport
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
    previews: [],
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
    onPreview: (preview) => {
      recorded.previews.push(preview === null ? null : preview.contours.length)
    },
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

// A gesture's promise chain outlives the test that started it, and `call()`
// reads whichever transport is installed *when it runs* — so drain the chains
// against their own transport, until it stops recording, before the next test
// installs another one.
afterEach(async () => {
  for (let pass = 0; pass < 12; pass += 1) {
    const before = installed?.calls.length ?? 0
    await settle()
    if ((installed?.calls.length ?? 0) === before) break
  }
  installed = null
  setTransport(undefined)
})

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
    installed = mock
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
    // One failure, one toast — however many calls the gesture had queued.
    expect(recorded.errors).toHaveLength(1)
    expect(String(recorded.errors[0])).toContain('kernel says no')
  })

  it('reports a failed single-shot command and still reloads', async () => {
    const { controller, recorded } = makeController()
    failing('doc.apply')

    controller.selectAll()
    await controller.toggleSmooth(false)
    await settle()

    expect(recorded.errors).toHaveLength(1)
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

describe('EditorController insert-on-segment', () => {
  it('inserts at the nearest extremum rather than under the pointer', async () => {
    const { controller, transport } = makeController()
    controller.setSource({
      ...source(),
      outline: makeLayerOutline(curvedLayer),
    })
    // Click on the arch at about a quarter along it — font (48, 67.5), which
    // is canvas (48, 232.5). Its only extremum is the apex, t = 0.5.
    controller.pointerDown(at(48, 233, { ...NO_MODIFIERS, shift: true }))
    await settle()

    const params = transport.calls.find((entry) => entry.method === 'doc.apply')
      ?.params as ApplyParams
    expect(params.command).toEqual({
      InsertPointOnSegment: {
        glyph: 'a',
        layer: 'layer-1',
        contour: 0,
        segment: 0,
        t: 0.5,
      },
    })
  })

  it('inserts where the pointer is on a straight segment', async () => {
    const { controller, transport } = makeController()
    // The square's bottom edge from (0, 0) to (200, 0), clicked at a quarter.
    controller.pointerDown(at(50, 300, { ...NO_MODIFIERS, shift: true }))
    await settle()

    const params = transport.calls.find((entry) => entry.method === 'doc.apply')
      ?.params as ApplyParams
    const command = params.command
    expect(
      command && 'InsertPointOnSegment' in command
        ? command.InsertPointOnSegment.t
        : 0,
    ).toBeCloseTo(0.25, 2)
  })
})

describe('EditorController preview publishing', () => {
  it('publishes the live geometry so the panels can read it', async () => {
    const { controller, recorded } = makeController()
    controller.pointerDown(at(200, 300))
    controller.pointerMove(at(230, 280))
    // The Info box reads this instead of the last fetched outline, which is
    // deferred for the length of the gesture.
    expect(recorded.previews.at(-1)).toBe(1)

    controller.pointerUp(at(230, 280))
    await settle()
    expect(recorded.previews.at(-1)).toBeNull()
  })
})

describe('EditorController snapping', () => {
  it('stops snapping when the preference is off', () => {
    const { controller } = makeController()
    // Drag node 0 to within snapping distance of node 1 at font (200, 0).
    const drop = at(197, 300)
    controller.pointerDown(at(0, 300))
    controller.pointerMove(drop)
    expect(controller.frame().contours?.[0]?.points[0]).toMatchObject({
      x: 200,
      y: 0,
    })

    const off = makeController()
    off.controller.setSnapping(false)
    off.controller.pointerDown(at(0, 300))
    off.controller.pointerMove(drop)
    expect(off.controller.frame().contours?.[0]?.points[0]).toMatchObject({
      x: 197,
      y: 0,
    })
  })
})

describe('EditorController view gestures', () => {
  it('pans with the middle button whatever the tool', () => {
    const { controller } = makeController()
    controller.pointerDown({ ...at(100, 100), button: 1 })
    controller.pointerMove({ ...at(140, 130), button: 1 })
    controller.pointerUp({ ...at(140, 130), button: 1 })

    expect(controller.viewportForTest.offset).toEqual({ x: 40, y: 330 })
  })

  it('pans with Space held, and stops when it is released', () => {
    const { controller } = makeController()
    controller.setSpaceHeld(true)
    controller.pointerDown(at(100, 100))
    controller.pointerMove(at(90, 120))
    controller.pointerUp(at(90, 120))
    expect(controller.viewportForTest.offset).toEqual({ x: -10, y: 320 })

    controller.setSpaceHeld(false)
    // The pan moved the glyph with it: node (200, 0) now sits at (190, 320).
    controller.pointerDown(at(190, 320))
    expect([...controller.currentSelection.keys()]).toEqual(['node:0:1'])
  })

  it('zooms in on a click and out with Alt', () => {
    const { controller } = makeController()
    controller.setTool('zoom')
    const before = controller.viewportForTest.scale

    controller.pointerDown(at(200, 200))
    controller.pointerUp(at(200, 200))
    expect(controller.viewportForTest.scale).toBeCloseTo(before * 1.15, 6)

    controller.pointerDown(at(200, 200, { ...NO_MODIFIERS, alt: true }))
    controller.pointerUp(at(200, 200, { ...NO_MODIFIERS, alt: true }))
    expect(controller.viewportForTest.scale).toBeCloseTo(before, 6)
  })

  it('zooms to a dragged rectangle', () => {
    const { controller } = makeController()
    controller.setTool('zoom')

    controller.pointerDown(at(100, 100))
    controller.pointerMove(at(300, 260))
    // The band is drawn while it is dragged.
    expect(controller.frame().overlay.marquee).toEqual({
      x0: 100,
      y0: 100,
      x1: 300,
      y1: 260,
    })
    controller.pointerUp(at(300, 260))

    // 200 × 160 canvas pixels framed in a 400 × 400 canvas: about twice.
    expect(controller.viewportForTest.scale).toBeGreaterThan(1.9)
    expect(controller.frame().overlay.marquee).toBeNull()
  })

  it('drops a band that never saw its pointer-up', () => {
    const { controller } = makeController()
    controller.pointerDown(at(380, 20))
    controller.pointerMove(at(300, 100))
    // A lost capture leaves the band open; the next press must not inherit it.
    controller.pointerDown(at(200, 300))
    controller.pointerUp(at(200, 300))

    expect([...controller.currentSelection.keys()]).toEqual(['node:0:1'])
  })
})

describe('EditorController gesture handover', () => {
  it('closes an open nudge run before the next gesture opens its own', async () => {
    const { controller, transport, recorded } = makeController()
    controller.setSelection(
      new Map([['node:0:1', { kind: 'node', contour: 0, point: 1 }]]),
    )
    controller.nudgeSelection({ x: 1, y: 0 }, NO_MODIFIERS)

    // A pointer-down inside the same animation frame: the run's apply is still
    // queued. `edit_begin` finalises the active edit in the kernel, so it must
    // not overtake that apply — the arrow press would come back
    // InvalidEditToken and raise a toast.
    controller.pointerDown(at(200, 300))
    controller.pointerMove(at(230, 300))
    controller.pointerUp(at(230, 300))
    await settle()

    expect(transport.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'doc.edit_apply',
      'doc.edit_end',
      'doc.edit_begin',
      'doc.edit_apply',
      'doc.edit_end',
    ])
    expect(recorded.errors).toEqual([])
  })

  it('closes it before a single-shot command too', async () => {
    const { controller, transport, recorded } = makeController()
    controller.setSelection(
      new Map([['node:0:1', { kind: 'node', contour: 0, point: 1 }]]),
    )
    controller.nudgeSelection({ x: 0, y: 1 }, NO_MODIFIERS)
    await controller.toggleSmooth(false)
    await settle()

    expect(transport.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'doc.edit_apply',
      'doc.edit_end',
      'doc.apply',
    ])
    expect(recorded.errors).toEqual([])
  })
})

describe('EditorController nudge handover', () => {
  it('keeps the preview until the kernel has closed the run', async () => {
    let release = () => undefined as void
    const held = new Promise<void>((resolve) => {
      release = () => resolve()
    })
    const mock = createMockTransport({
      'doc.edit_begin': () => 5,
      'doc.edit_apply': () => change(),
      'doc.edit_end': async () => {
        await held
        return null
      },
    })
    const { controller, recorded } = makeController()
    setTransport(mock)
    installed = mock
    controller.setSelection(
      new Map([['node:0:1', { kind: 'node', contour: 0, point: 1 }]]),
    )
    controller.nudgeSelection({ x: 5, y: 0 }, NO_MODIFIERS)
    const moved = controller.frame().contours?.[0]?.points[1]
    expect(moved).toMatchObject({ x: 205, y: 0 })

    const ending = controller.endNudge()
    await settle()
    // The drag path holds its preview until `edit_end` resolves; the nudge
    // path must not blink back to the pre-nudge geometry before then.
    expect(controller.frame().contours?.[0]?.points[1]).toMatchObject({
      x: 205,
      y: 0,
    })
    const reloadsBefore = recorded.reloads

    release()
    await ending
    await settle()
    expect(controller.frame().contours).toBeNull()
    expect(recorded.reloads).toBeGreaterThan(reloadsBefore)
  })

  it('carries a closed run forward when the selection changes mid-run', async () => {
    const { controller } = makeController()
    controller.setSelection(
      new Map([['node:0:1', { kind: 'node', contour: 0, point: 1 }]]),
    )
    controller.nudgeSelection({ x: 10, y: 0 }, NO_MODIFIERS)

    // Tab to another node inside the 500 ms window: the first run closes, but
    // its reload cannot have landed yet, so the second run has to accumulate
    // from what the first one produced rather than from the stale outline.
    controller.setSelection(
      new Map([['node:0:2', { kind: 'node', contour: 0, point: 2 }]]),
    )
    controller.nudgeSelection({ x: 0, y: 10 }, NO_MODIFIERS)

    const points = controller.frame().contours?.[0]?.points
    expect(points?.[1]).toMatchObject({ x: 210, y: 0 })
    expect(points?.[2]).toMatchObject({ x: 200, y: 210 })
    await controller.endNudge()
    await settle()
  })

  it('still closes the second run when the first one interrupted it', async () => {
    const { controller, transport, recorded } = makeController()
    controller.setSelection(
      new Map([['node:0:1', { kind: 'node', contour: 0, point: 1 }]]),
    )
    controller.nudgeSelection({ x: 10, y: 0 }, NO_MODIFIERS)
    controller.setSelection(
      new Map([['node:0:2', { kind: 'node', contour: 0, point: 2 }]]),
    )
    controller.nudgeSelection({ x: 0, y: 10 }, NO_MODIFIERS)
    await controller.endNudge()
    await settle()

    // Two runs, two undo entries, and the preview handed back at the end of
    // the second — the first run's late `onEnd` must not have taken it.
    expect(
      transport.calls.filter((entry) => entry.method === 'doc.edit_end'),
    ).toHaveLength(2)
    expect(controller.frame().contours).toBeNull()
    expect(recorded.editing.at(-1)).toBe(false)
  })
})
