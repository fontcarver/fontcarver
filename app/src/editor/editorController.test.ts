import { afterEach, describe, expect, it, vi } from 'vitest'

import { setTransport, type ApplyParams } from '../api'
import { Viewport } from '../canvas/Viewport'
import {
  makeContour,
  makeLayer,
  makeLayerOutline,
  makePoint,
} from '../test/fontFixtures'
import { TOOL_LABELS } from '../test/toolContext'
import {
  createEditingTransport,
  createMockTransport,
  type MockTransport,
} from '../test/mockTransport'

import {
  EditorController,
  type EditorLabels,
  type EditorSource,
  type PointerInput,
} from './editorController'
import { registerEditor } from './activeEditor'
import { commitField } from '../views/glyph/commitField'
import { NO_MODIFIERS, type Modifiers } from './gestures/drag'
import { NUDGE_TIMEOUT } from './gestures/nudge'
import { suspendDocumentEditParticipants } from './documentEditParticipants'

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
  setMetrics: 'Set metrics',
  addGuide: 'Add guide',
  metricHud: (handle, value) => `${handle} ${value}`,
  snap: {
    node: 'node',
    handle: 'handle',
    guide: 'guide',
    anchor: 'anchor',
    grid: 'grid',
    component: 'component',
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
  tools: TOOL_LABELS,
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
    curves: 'cubic',
    componentPoints: [],
  }
}

/** The transport the running test installed, drained when it ends. */
let installed: MockTransport | null = null
/** Controllers the running test built, stopped when it ends. */
let built: EditorController[] = []

/** What the view would have been told. */
interface Recorded {
  selections: string[][]
  editing: boolean[]
  reloads: number
  errors: unknown[]
  previews: (number | null)[]
}

/** 1 CSS pixel per font unit, font origin at canvas (0, 300). */
function makeController(supplied?: MockTransport): {
  controller: EditorController
  transport: MockTransport
  recorded: Recorded
} {
  const transport =
    supplied ??
    createMockTransport({
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
    onShapeDialog: () => undefined,
    onError: (error) => recorded.errors.push(error),
  })
  controller.setSource(source())
  built.push(controller)
  return { controller, transport, recorded }
}

function change() {
  return {
    glyphs: [42],
    font_info: false,
    masters: false,
    layout: false,
    bitmaps: false,
    math: false,
    color: false,
    feature_source: false,
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

/** Lets the production animation-frame scheduler put an apply on the wire. */
async function settleFrame(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 20))
  await settle()
}

describe('dragging an anchor on its own', () => {
  it('commits the TransformSelection that carries it', async () => {
    const { controller, transport } = makeController(createEditingTransport())

    // The layer's anchor sits at (100, 260), which is canvas (100, 40).
    controller.pointerDown(at(100, 40))
    expect(controller.currentSelection.has('anchor:0')).toBe(true)
    controller.pointerMove(at(130, 40))
    controller.pointerUp(at(130, 40))
    await settle()

    const applied = transport.calls.find(
      (entry) => entry.method === 'doc.edit_apply',
    )?.params as { command: Record<string, unknown> } | undefined
    expect(applied?.command).toEqual({
      TransformSelection: {
        glyph: 'a',
        layer: 'layer-1',
        points: [],
        components: [],
        anchors: [0],
        affine: [1, 0, 0, 1, 30, 0],
      },
    })
    expect((transport.calls[0]?.params as { label: string }).label).toBe(
      'Move anchor',
    )
  })
})

describe('metric handles and ruler guides', () => {
  it('drags the right sidebearing as one coalesced SetMetrics run', async () => {
    const { controller, transport, recorded } = makeController(
      createEditingTransport(),
    )

    // The RSB line sits at x = 200 (width 600 − rsb 400); a vertical segment
    // of the outline runs along it, and §4.2 puts the metric handle first.
    controller.pointerDown(at(200, 250))
    controller.pointerMove(at(240, 250))
    controller.pointerUp(at(240, 250))
    await settle()

    expect(transport.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'doc.edit_apply',
      'doc.edit_end',
    ])
    expect(
      (transport.calls[1]?.params as { command: unknown }).command,
    ).toEqual({
      SetMetrics: {
        glyph: 'a',
        layer: 'layer-1',
        width: null,
        lsb: null,
        rsb: 360,
      },
    })
    expect((transport.calls[0]?.params as { label: string }).label).toBe(
      'Set metrics',
    )
    expect(recorded.editing).toEqual([true, false])
    expect(recorded.errors).toEqual([])
  })

  it('leaves a node alone even when it sits on a metric line', async () => {
    const { controller, transport } = makeController(createEditingTransport())

    // The node at (200, 200) is on the RSB line; a node outranks the handle.
    controller.pointerDown(at(200, 100))
    controller.pointerMove(at(210, 100))
    controller.pointerUp(at(210, 100))
    await settle()

    const applied = transport.calls.find(
      (entry) => entry.method === 'doc.edit_apply',
    )?.params as { command: Record<string, unknown> } | undefined
    expect(applied && Object.keys(applied.command)).toEqual(['MovePoints'])
  })

  it('adds a layer guide dragged out of the top ruler', async () => {
    const { controller, transport } = makeController()

    controller.pointerDown(at(200, 5, { ...NO_MODIFIERS, alt: true }))
    controller.pointerMove(at(200, 260, { ...NO_MODIFIERS, alt: true }))
    controller.pointerUp(at(200, 260, { ...NO_MODIFIERS, alt: true }))
    await settle()

    expect(transport.calls.map((entry) => entry.method)).toEqual(['doc.apply'])
    expect(transport.calls[0]?.params).toEqual({
      doc: 1,
      label: 'Add guide',
      command: {
        AddGuide: {
          glyph: 'a',
          layer: 'layer-1',
          master: null,
          guide: {
            x: 0,
            y: 40,
            angle: 0,
            name: null,
            locked: false,
            color: null,
          },
        },
      },
    })
  })

  it('puts a shifted ruler drag in the master scope', async () => {
    const { controller, transport } = makeController()

    const held = { ...NO_MODIFIERS, alt: true, shift: true }
    controller.pointerDown(at(5, 200, held))
    controller.pointerUp(at(120, 200, held))
    await settle()

    expect(transport.calls[0]?.params).toMatchObject({
      command: {
        AddGuide: { glyph: null, layer: null, master: 'm1' },
      },
    })
  })

  it('draws no guide when the rulers are hidden', async () => {
    const { controller, transport } = makeController()
    controller.setRulersVisible(false)

    controller.pointerDown(at(200, 5, { ...NO_MODIFIERS, alt: true }))
    controller.pointerUp(at(200, 5, { ...NO_MODIFIERS, alt: true }))
    await settle()

    expect(transport.calls.some((entry) => entry.method === 'doc.apply')).toBe(
      false,
    )
  })
})

// A gesture's promise chain outlives the test that started it, and `call()`
// reads whichever transport is installed *when it runs* — so drain the chains
// against their own transport, until it stops recording, before the next test
// installs another one.
afterEach(async () => {
  // Stop every gesture this test started, then drain the chains they left
  // against their own transport, before the next test installs another one.
  for (const controller of built) controller.dispose()
  built = []
  for (let pass = 0; pass < 24; pass += 1) {
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
  it('does not accept outline input while the document is closing', async () => {
    const { controller, transport } = makeController()
    controller.setSelection(
      new Map([['node:0:1', { kind: 'node', contour: 0, point: 1 }]]),
    )
    const resume = suspendDocumentEditParticipants(1)
    try {
      controller.nudgeSelection({ x: 1, y: 0 }, NO_MODIFIERS)
      controller.pointerDown(at(200, 300))
      controller.pointerMove(at(250, 300))
      controller.pointerUp(at(250, 300))
      await controller.toggleSmooth(false)
      await controller.prepareTransition()
      expect(transport.calls).toEqual([])
    } finally {
      resume()
    }
  })
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

  it('finishes a tool against the old glyph before a text target changes', async () => {
    const { controller, transport } = makeController()
    controller.setTool('pen')
    controller.pointerDown(at(20, 280))
    controller.pointerUp(at(20, 280))
    controller.pointerDown(at(120, 280))
    controller.pointerUp(at(120, 280))

    await controller.prepareTransition()
    controller.setSource({ ...source(), gid: 43, glyph: 'b' })
    await settle()

    const applied = transport.calls.find(
      (entry) => entry.method === 'doc.apply',
    )?.params as ApplyParams
    expect(applied.command).toMatchObject({ AddContour: { glyph: 'a' } })
  })

  it('flushes a tab-owned metrics run before another command', async () => {
    const { controller } = makeController()
    const order: string[] = []
    controller.setExternalSettler(async () => {
      order.push('metrics')
    })
    controller.selectAll()

    await controller.toggleSmooth(false)

    expect(order).toEqual(['metrics'])
  })

  it('gates an immediate drag behind a tab-owned metrics close', async () => {
    const { controller, transport } = makeController()
    let release: (() => void) | undefined
    const metricsClosing = new Promise<void>((resolve) => {
      release = resolve
    })
    controller.setExternalSettler(() => {
      controller.setExternalSettler(null)
      return metricsClosing
    })

    controller.pointerDown(at(200, 300))
    controller.pointerMove(at(230, 300))
    expect(transport.calls).toHaveLength(0)

    release?.()
    controller.pointerUp(at(230, 300))
    await settle()

    expect(transport.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'doc.edit_apply',
      'doc.edit_end',
    ])
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

describe('EditorController against the kernel token rules', () => {
  const selectNode = (controller: EditorController, point: number) =>
    controller.setSelection(
      new Map([[`node:0:${point}`, { kind: 'node', contour: 0, point }]]),
    )

  function heldMetrics() {
    let release = () => undefined as void
    const until = new Promise<void>((resolve) => {
      release = () => resolve()
    })
    const made = makeController(
      createEditingTransport({ hold: { on: 'doc.edit_apply', until } }),
    )
    return { ...made, release }
  }

  it('keeps an immediate second metrics drag behind the first close', async () => {
    const { controller, transport, recorded, release } = heldMetrics()
    controller.pointerDown(at(0, 200))
    controller.pointerMove(at(10, 200))
    controller.pointerUp(at(10, 200))
    await settle()

    // The first drag's final optimistic frame remains the visible and logical
    // baseline until its reload arrives, so the second grabs x=10 and adds 20.
    controller.pointerDown(at(10, 200))
    controller.pointerMove(at(30, 200))
    controller.pointerUp(at(30, 200))
    await settle()
    expect(transport.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'doc.edit_apply',
    ])

    release()
    await controller.settle()
    await settle()
    expect(transport.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'doc.edit_apply',
      'doc.edit_end',
      'doc.edit_begin',
      'doc.edit_apply',
      'doc.edit_end',
    ])
    expect(
      transport.calls
        .filter((entry) => entry.method === 'doc.edit_apply')
        .map(
          (entry) =>
            (entry.params as { command: { SetMetrics: { lsb: number } } })
              .command.SetMetrics.lsb,
        ),
    ).toEqual([10, 30])
    expect(recorded.errors).toEqual([])
  })

  it('gates a new source without projecting the old metrics preview', async () => {
    const { controller, transport, recorded, release } = heldMetrics()
    controller.pointerDown(at(0, 200))
    controller.pointerMove(at(10, 200))
    controller.pointerUp(at(10, 200))
    await settle()
    expect(controller.frame().metrics?.lsb).toBe(10)

    const next = source()
    controller.setSource({
      ...next,
      gid: 43,
      glyph: 'b',
      outline: { ...next.outline, layer: 'layer-2' },
    })
    expect(controller.frame().metrics).toBeNull()
    controller.pointerDown(at(0, 200))
    controller.pointerMove(at(20, 200))
    controller.pointerUp(at(20, 200))
    await settle()
    expect(transport.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'doc.edit_apply',
    ])

    release()
    await controller.settle()
    await settle()
    expect(
      transport.calls
        .filter((entry) => entry.method === 'doc.edit_apply')
        .map(
          (entry) =>
            (entry.params as { command: { SetMetrics: { layer: string } } })
              .command.SetMetrics.layer,
        ),
    ).toEqual(['layer-1', 'layer-2'])
    expect(recorded.errors).toEqual([])
  })

  it('keeps a panel field commit behind a metrics drag close', async () => {
    const { controller, transport, recorded, release } = heldMetrics()
    const stop = registerEditor(controller)
    try {
      controller.pointerDown(at(0, 200))
      controller.pointerMove(at(10, 200))
      controller.pointerUp(at(10, 200))
      await settle()
      commitField(
        { doc: 1, glyph: 'a', layer: 'layer-1' },
        {
          SetMetrics: {
            glyph: 'a',
            layer: 'layer-1',
            width: 620,
            lsb: null,
            rsb: null,
          },
        },
        'Set metrics',
        () => undefined,
      )
      await settle()
      expect(transport.calls.map((entry) => entry.method)).toEqual([
        'doc.edit_begin',
        'doc.edit_apply',
      ])

      release()
      await controller.settle()
      await settle()
      expect(transport.calls.map((entry) => entry.method)).toEqual([
        'doc.edit_begin',
        'doc.edit_apply',
        'doc.edit_end',
        'doc.apply',
      ])
      expect(recorded.errors).toEqual([])
    } finally {
      stop()
    }
  })

  it('finishes Escape undo before a new command can apply', async () => {
    const { controller, transport, recorded, release } = heldMetrics()
    const stop = registerEditor(controller)
    try {
      controller.pointerDown(at(0, 200))
      controller.pointerMove(at(10, 200))
      await settleFrame()
      controller.escape()
      commitField(
        { doc: 1, glyph: 'a', layer: 'layer-1' },
        {
          SetGlyphMeta: {
            glyph: 'a',
            export: false,
            tags: null,
            note: null,
          },
        },
        'Set export flag',
        () => undefined,
      )
      await settle()
      expect(transport.calls.map((entry) => entry.method)).toEqual([
        'doc.edit_begin',
        'doc.edit_apply',
      ])

      release()
      await controller.settle()
      await settle()
      expect(transport.calls.map((entry) => entry.method)).toEqual([
        'doc.edit_begin',
        'doc.edit_apply',
        'doc.edit_end',
        'doc.undo',
        'doc.apply',
      ])
      expect(recorded.errors).toEqual([])
    } finally {
      stop()
    }
  })

  it('keeps a replacement controller command behind disposal cancellation', async () => {
    const { controller, transport, recorded, release } = heldMetrics()
    controller.pointerDown(at(0, 200))
    controller.pointerMove(at(10, 200))
    await settleFrame()
    controller.dispose()
    expect(recorded.editing.at(-1)).toBe(false)

    const replacement = makeController(transport).controller
    const next = source()
    replacement.setSource({
      ...next,
      gid: 43,
      glyph: 'b',
      outline: { ...next.outline, layer: 'layer-2' },
    })
    const stop = registerEditor(replacement)
    try {
      commitField(
        { doc: 1, glyph: 'b', layer: 'layer-2' },
        {
          SetMetrics: {
            glyph: 'b',
            layer: 'layer-2',
            width: 620,
            lsb: null,
            rsb: null,
          },
        },
        'Set metrics',
        () => undefined,
      )
      await settle()
      expect(transport.calls.map((entry) => entry.method)).toEqual([
        'doc.edit_begin',
        'doc.edit_apply',
      ])

      release()
      await settle()
      expect(transport.calls.map((entry) => entry.method)).toEqual([
        'doc.edit_begin',
        'doc.edit_apply',
        'doc.edit_end',
        'doc.undo',
        'doc.apply',
      ])
      expect(recorded.errors).toEqual([])
    } finally {
      stop()
    }
  })

  it('still reports a cancellation failure after disposal', async () => {
    const { controller, recorded } = makeController(
      createMockTransport({
        'doc.edit_begin': () => 5,
        'doc.edit_apply': () => change(),
        'doc.edit_end': () => null,
        'doc.undo': () => {
          throw new Error('undo failed')
        },
      }),
    )
    controller.pointerDown(at(0, 200))
    controller.pointerMove(at(10, 200))
    await settleFrame()

    controller.dispose()
    await settle()

    expect(recorded.errors.map(String)).toEqual(['Error: undo failed'])
  })

  it('cancels live metrics before editing a replacement source', async () => {
    const { controller, transport, recorded, release } = heldMetrics()
    controller.pointerDown(at(0, 200))
    controller.pointerMove(at(10, 200))
    await settleFrame()

    const next = source()
    controller.setSource({
      ...next,
      gid: 43,
      glyph: 'b',
      outline: { ...next.outline, layer: 'layer-2' },
    })
    expect(controller.frame().metrics).toBeNull()
    controller.pointerDown(at(0, 200))
    controller.pointerMove(at(20, 200))
    controller.pointerUp(at(20, 200))
    await settle()
    expect(transport.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'doc.edit_apply',
    ])

    release()
    await controller.settle()
    await settle()
    expect(transport.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'doc.edit_apply',
      'doc.edit_end',
      'doc.undo',
      'doc.edit_begin',
      'doc.edit_apply',
      'doc.edit_end',
    ])
    expect(recorded.errors).toEqual([])
  })

  it('does not cancel a live metrics edit on a same-source reload', async () => {
    const { controller, transport, release } = heldMetrics()
    controller.pointerDown(at(0, 200))
    controller.pointerMove(at(10, 200))
    await settleFrame()

    controller.setSource({ ...source(), outline: { ...source().outline } })
    expect(controller.frame().metrics?.lsb).toBe(10)
    expect(transport.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'doc.edit_apply',
    ])

    controller.escape()
    release()
    await controller.settle()
  })

  it('keeps a replacement command behind outline disposal cancellation', async () => {
    const { controller, transport, release } = heldMetrics()
    controller.pointerDown(at(200, 300))
    controller.pointerMove(at(220, 300))
    await settleFrame()
    controller.dispose()

    const replacement = makeController(transport).controller
    const stop = registerEditor(replacement)
    try {
      commitField(
        { doc: 1, glyph: 'a', layer: 'layer-1' },
        {
          SetMetrics: {
            glyph: 'a',
            layer: 'layer-1',
            width: 620,
            lsb: null,
            rsb: null,
          },
        },
        'Set metrics',
        () => undefined,
      )
      await settle()
      expect(transport.calls.map((entry) => entry.method)).toEqual([
        'doc.edit_begin',
        'doc.edit_apply',
      ])

      release()
      await settle()
      expect(transport.calls.map((entry) => entry.method)).toEqual([
        'doc.edit_begin',
        'doc.edit_apply',
        'doc.edit_end',
        'doc.undo',
        'doc.apply',
      ])
    } finally {
      stop()
    }
  })

  it('finishes a nudge before a replacement controller command', async () => {
    const { controller, transport, release } = heldMetrics()
    selectNode(controller, 1)
    controller.nudgeSelection({ x: 10, y: 0 }, NO_MODIFIERS)
    controller.dispose()
    await settle()

    const replacement = makeController(transport).controller
    const stop = registerEditor(replacement)
    try {
      commitField(
        { doc: 1, glyph: 'a', layer: 'layer-1' },
        {
          SetMetrics: {
            glyph: 'a',
            layer: 'layer-1',
            width: 620,
            lsb: null,
            rsb: null,
          },
        },
        'Set metrics',
        () => undefined,
      )
      await settle()
      expect(transport.calls.map((entry) => entry.method)).toEqual([
        'doc.edit_begin',
        'doc.edit_apply',
      ])

      release()
      await settle()
      expect(transport.calls.map((entry) => entry.method)).toEqual([
        'doc.edit_begin',
        'doc.edit_apply',
        'doc.edit_end',
        'doc.apply',
      ])
    } finally {
      stop()
    }
  })

  it('uses one Escape to close a nudge and a second to clear selection', async () => {
    const { controller, transport } = makeController(createEditingTransport())
    selectNode(controller, 1)
    controller.nudgeSelection({ x: 10, y: 0 }, NO_MODIFIERS)

    controller.escape()
    expect(controller.currentSelection.size).toBe(1)
    await settle()
    expect(transport.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'doc.edit_apply',
      'doc.edit_end',
    ])

    controller.escape()
    expect(controller.currentSelection.size).toBe(0)
  })

  it('starts a drag after Escape from the restored baseline', async () => {
    const { controller, transport, recorded, release } = heldMetrics()
    controller.pointerDown(at(0, 200))
    controller.pointerMove(at(10, 200))
    await settleFrame()
    controller.escape()
    expect(controller.frame().contours).toBeNull()

    controller.pointerDown(at(0, 200))
    controller.pointerMove(at(20, 200))
    controller.pointerUp(at(20, 200))
    await settle()
    expect(transport.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'doc.edit_apply',
    ])

    release()
    await controller.settle()
    await settle()
    expect(transport.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'doc.edit_apply',
      'doc.edit_end',
      'doc.undo',
      'doc.edit_begin',
      'doc.edit_apply',
      'doc.edit_end',
    ])
    expect(
      transport.calls
        .filter((entry) => entry.method === 'doc.edit_apply')
        .map(
          (entry) =>
            (entry.params as { command: { SetMetrics: { lsb: number } } })
              .command.SetMetrics.lsb,
        ),
    ).toEqual([10, 20])
    expect(recorded.errors).toEqual([])
  })

  it('keeps a second nudge run behind the first one it closes', async () => {
    const { controller, transport, recorded } = makeController(
      createEditingTransport(),
    )
    selectNode(controller, 1)
    controller.nudgeSelection({ x: 10, y: 0 }, NO_MODIFIERS)
    // Tab to another node inside the same animation frame: the first run's
    // apply has not left yet, and `edit_begin` finalises whatever is open.
    selectNode(controller, 2)
    controller.nudgeSelection({ x: 0, y: 10 }, NO_MODIFIERS)
    await controller.endNudge()
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

  it('keeps a drag behind the nudge run it interrupts', async () => {
    const { controller, transport, recorded } = makeController(
      createEditingTransport(),
    )
    selectNode(controller, 1)
    controller.nudgeSelection({ x: 10, y: 0 }, NO_MODIFIERS)
    controller.pointerDown(at(210, 300))
    controller.pointerMove(at(240, 300))
    controller.pointerUp(at(240, 300))
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

  it('keeps a single-shot command behind the run it interrupts', async () => {
    const { controller, transport, recorded } = makeController(
      createEditingTransport(),
    )
    selectNode(controller, 1)
    controller.nudgeSelection({ x: 10, y: 0 }, NO_MODIFIERS)
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

  it('does not let a late close clear the run that replaced it', async () => {
    let release = () => undefined as void
    const until = new Promise<void>((resolve) => {
      release = () => resolve()
    })
    const { controller, recorded } = makeController(
      createEditingTransport({ hold: { on: 'doc.edit_end', until } }),
    )
    selectNode(controller, 1)
    controller.nudgeSelection({ x: 10, y: 0 }, NO_MODIFIERS)
    // Close the run, then press again with the *same* selection while its
    // `edit_end` is still in flight.
    void controller.endNudge()
    await settle()
    controller.nudgeSelection({ x: 10, y: 0 }, NO_MODIFIERS)
    const previewing = controller.frame().contours?.[0]?.points[1]
    expect(previewing).toMatchObject({ x: 220, y: 0 })
    const reloadsBefore = recorded.reloads

    release()
    await settle()

    // The first run's late close must not take the second run's preview.
    expect(controller.frame().contours?.[0]?.points[1]).toMatchObject({
      x: 220,
      y: 0,
    })
    expect(recorded.reloads).toBe(reloadsBefore)
    await controller.endNudge()
    await settle()
  })

  it('drops the gate once the run it was waiting for has closed', async () => {
    const { controller, transport } = makeController(createEditingTransport())
    selectNode(controller, 1)
    controller.nudgeSelection({ x: 10, y: 0 }, NO_MODIFIERS)
    await controller.endNudge()
    await settle()
    const before = transport.calls.length

    // Nothing is closing, so the drag opens its edit immediately rather than
    // queueing behind a promise that has already resolved — a gate that is
    // never dropped would make one unanswered call block every later gesture.
    // (This controller has no view to reload it, so the node is still drawn at
    // the position the run started from.)
    controller.pointerDown(at(200, 300))
    expect(transport.calls.slice(before).map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
    ])
    controller.pointerUp(at(200, 300))
    await settle()
  })

  it('closes the run before an Info box commit reaches the kernel', async () => {
    const { controller, transport, recorded } = makeController(
      createEditingTransport(),
    )
    const stop = registerEditor(controller)
    try {
      selectNode(controller, 1)
      controller.nudgeSelection({ x: 10, y: 0 }, NO_MODIFIERS)
      // A field commit inside the same animation frame: `doc.apply` finalises
      // the open edit, so it has to wait for the run's apply just as a drag does.
      commitField(
        { doc: 1, glyph: 'a', layer: 'layer-1' },
        {
          SetPointPosition: {
            glyph: 'a',
            layer: 'layer-1',
            point: { contour: 0, point: 1 },
            x: 220,
            y: 0,
          },
        },
        'Set position',
        () => undefined,
      )
      await settle()

      expect(transport.calls.map((entry) => entry.method)).toEqual([
        'doc.edit_begin',
        'doc.edit_apply',
        'doc.edit_end',
        'doc.apply',
      ])
      expect(recorded.errors).toEqual([])
    } finally {
      stop()
    }
  })
  it('keeps an arrow press behind a run closed by endNudge', async () => {
    const { controller, transport, recorded } = makeController(
      createEditingTransport(),
    )
    selectNode(controller, 1)
    controller.nudgeSelection({ x: 10, y: 0 }, NO_MODIFIERS)
    // The close the runner did *not* start itself: one round trip later the
    // replacement run used to overtake the first run's queued apply.
    void controller.endNudge()
    controller.nudgeSelection({ x: 10, y: 0 }, NO_MODIFIERS)
    await controller.endNudge()
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

  it('keeps an arrow press behind the drag a pointer-up closed', async () => {
    const { controller, transport, recorded } = makeController(
      createEditingTransport(),
    )
    controller.pointerDown(at(200, 300))
    controller.pointerMove(at(230, 300))
    controller.pointerUp(at(230, 300))
    // Same tick as the pointer-up, so the drag's `edit_end` is still queued.
    controller.nudgeSelection({ x: 10, y: 0 }, NO_MODIFIERS)
    await controller.endNudge()
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

  it('closes a drag that never saw its pointer-up before the next one', async () => {
    const { controller, transport, recorded } = makeController(
      createEditingTransport(),
    )
    controller.pointerDown(at(200, 300))
    controller.pointerMove(at(230, 300))
    // A lost pointer capture: the second gesture starts with the first still
    // open, and its `edit_begin` would otherwise invalidate the queued apply.
    controller.pointerDown(at(230, 300))
    controller.pointerMove(at(260, 300))
    controller.pointerUp(at(260, 300))
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

  it('closes the drag a tool switch interrupts', async () => {
    const { controller, transport, recorded } = makeController(
      createEditingTransport(),
    )
    controller.pointerDown(at(200, 300))
    controller.pointerMove(at(230, 300))
    controller.setTool('hand')
    await settle()

    expect(transport.calls.map((entry) => entry.method)).toEqual([
      'doc.edit_begin',
      'doc.edit_apply',
      'doc.edit_end',
    ])
    expect(controller.dragging).toBe(false)
    expect(recorded.errors).toEqual([])
  })
})

describe('EditorController drawing tools', () => {
  /** Font (x, y) maps to canvas (x, 300 − y) in this fixture. */
  const clickAt = (controller: EditorController, x: number, y: number) => {
    controller.pointerDown(at(x, 300 - y))
    controller.pointerUp(at(x, 300 - y))
  }

  it('routes the pen and applies one AddContour for a closed triangle', async () => {
    const { controller, transport } = makeController()
    controller.setTool('pen')
    clickAt(controller, 300, 0)
    clickAt(controller, 400, 0)
    clickAt(controller, 350, 100)
    clickAt(controller, 300, 0)
    await settle()

    const applies = transport.calls.filter(
      (entry) => entry.method === 'doc.apply',
    )
    expect(applies).toHaveLength(1)
    const params = applies[0]?.params as ApplyParams
    const command = params.command as {
      AddContour: { contour: { closed: boolean; points: unknown[] } }
    }
    expect(command.AddContour.contour.closed).toBe(true)
    expect(command.AddContour.contour.points).toHaveLength(3)
  })

  it('shows the path being drawn in the frame it hands the canvas', () => {
    const { controller } = makeController()
    controller.setTool('pen')
    clickAt(controller, 300, 0)
    clickAt(controller, 400, 0)
    controller.pointerMove(at(400, 200))
    const frame = controller.frame()
    expect(frame.tool.contours).toHaveLength(1)
    expect(frame.tool.marks.length).toBeGreaterThan(0)
  })

  it('offers Enter and Escape to the tool before the editor sees them', () => {
    const { controller } = makeController()
    controller.setTool('pen')
    expect(controller.claimsKey('Enter')).toBe(false)
    clickAt(controller, 300, 0)
    clickAt(controller, 400, 0)
    expect(controller.claimsKey('Enter')).toBe(true)
    expect(
      controller.toolKey({
        key: 'Enter',
        modifiers: NO_MODIFIERS,
        repeat: false,
      }),
    ).toBe(true)
    expect(controller.claimsKey('Enter')).toBe(false)
  })

  it('asks the view for the numeric dialog when a shape tool is clicked', () => {
    const dialogs: unknown[] = []
    const { controller } = makeController()
    controller.setCallbacks({
      repaint: () => undefined,
      onSelectionChange: () => undefined,
      onEditingChange: () => undefined,
      onZoomChange: () => undefined,
      onReload: () => undefined,
      onCursorChange: () => undefined,
      onPreview: () => undefined,
      onShapeDialog: (request) => dialogs.push(request),
      onError: () => undefined,
    })
    controller.setTool('rectangle')
    clickAt(controller, 40, 60)
    expect(dialogs).toEqual([{ kind: 'rectangle', at: { x: 40, y: 60 } }])
  })

  it('commits an open path when the tool is switched away', async () => {
    const { controller, transport } = makeController()
    controller.setTool('pen')
    clickAt(controller, 300, 0)
    clickAt(controller, 400, 0)
    controller.setTool('select')
    await settle()

    const applies = transport.calls.filter(
      (entry) => entry.method === 'doc.apply',
    )
    expect(applies).toHaveLength(1)
    const params = applies[0]?.params as ApplyParams
    expect(params.label).toBe('Draw path')
  })
})

describe('EditorController nudging beside a drag', () => {
  it('keeps the nudge preview that starts while the drag is closing', async () => {
    const { controller } = makeController(createEditingTransport())
    controller.pointerDown(at(200, 300))
    controller.pointerMove(at(230, 300))
    controller.nudgeSelection({ x: 0, y: 10 }, NO_MODIFIERS)

    // Let the interrupted drag finish while the nudge run remains open. Its
    // cleanup must leave the newer preview in place.
    await settle()
    expect(controller.frame().contours?.[0]?.points[1]).toMatchObject({
      x: 230,
      y: 10,
    })
  })

  it('closes a drag that is still open before the arrow press begins', async () => {
    const { controller, transport, recorded } = makeController(
      createEditingTransport(),
    )
    // Pointer still down: the drag has an open edit and no gate of its own.
    controller.pointerDown(at(200, 300))
    controller.pointerMove(at(230, 300))
    controller.nudgeSelection({ x: 0, y: 10 }, NO_MODIFIERS)
    await controller.endNudge()
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
    expect(controller.dragging).toBe(false)
  })
})

describe('EditorController tool feedback', () => {
  it('repaints after clearing the snap halo at gesture end', () => {
    const { controller } = makeController()
    const paintedSnap: boolean[] = []
    controller.setCallbacks({
      repaint: () => paintedSnap.push(controller.frame().overlay.snap !== null),
      onSelectionChange: () => undefined,
      onEditingChange: () => undefined,
      onZoomChange: () => undefined,
      onReload: () => undefined,
      onCursorChange: () => undefined,
      onPreview: () => undefined,
      onShapeDialog: () => undefined,
      onError: () => undefined,
    })
    controller.setTool('pen')
    controller.pointerDown(at(202, 302))
    controller.pointerUp(at(202, 302))

    expect(paintedSnap.at(-1)).toBe(false)
  })

  it('repaints after switching away from a tool with idle feedback', () => {
    const { controller } = makeController()
    const paintedMeasure: boolean[] = []
    controller.setCallbacks({
      repaint: () =>
        paintedMeasure.push(controller.frame().tool.lines.length > 0),
      onSelectionChange: () => undefined,
      onEditingChange: () => undefined,
      onZoomChange: () => undefined,
      onReload: () => undefined,
      onCursorChange: () => undefined,
      onPreview: () => undefined,
      onShapeDialog: () => undefined,
      onError: () => undefined,
    })
    controller.setTool('measure')
    controller.pointerDown(at(20, 20))
    controller.pointerMove(at(120, 20))
    controller.pointerUp(at(120, 20))
    expect(paintedMeasure.at(-1)).toBe(true)

    controller.setTool('select')
    expect(paintedMeasure.at(-1)).toBe(false)
  })

  it('clears the snap halo when a tool gesture ends', () => {
    const { controller } = makeController()
    controller.setTool('pen')
    // The node lands on the square's corner, which the snap latches onto.
    controller.pointerDown(at(202, 302))
    expect(controller.frame().overlay.snap).not.toBeNull()
    controller.pointerUp(at(202, 302))
    expect(controller.frame().overlay.snap).toBeNull()
  })

  it('repaints for an event a tool consumed, not for one it ignored', () => {
    const { controller } = makeController()
    let repaints = 0
    controller.setCallbacks({
      repaint: () => {
        repaints += 1
      },
      onSelectionChange: () => undefined,
      onEditingChange: () => undefined,
      onZoomChange: () => undefined,
      onReload: () => undefined,
      onCursorChange: () => undefined,
      onPreview: () => undefined,
      onShapeDialog: () => undefined,
      onError: () => undefined,
    })
    controller.setTool('pen')

    // A key no tool claims is not a reason to repaint the whole editor: the
    // canvas offers every key to the active tool.
    repaints = 0
    expect(
      controller.toolKey({ key: 'q', modifiers: NO_MODIFIERS, repeat: false }),
    ).toBe(false)
    expect(repaints).toBe(0)

    // A click the pen consumed is.
    controller.pointerDown(at(300, 100))
    expect(repaints).toBeGreaterThan(0)
  })
})
