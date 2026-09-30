import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { setTransport, type ApplyParams } from '../../api'
import { selectionOf } from '../../editor/selection'
import { registerBuiltinCommands } from '../../shell/builtinCommands'
import {
  isCommandEnabled,
  resetCommands,
  runCommand,
  setCommandErrorReporter,
} from '../../shell/commands'
import { commandForEvent } from '../../shell/shortcuts'
import {
  initialDocumentState,
  useDocumentStore,
} from '../../stores/documentStore'
import { initialEditorState, useEditorStore } from '../../stores/editorStore'
import { initialSessionState, useSessionStore } from '../../stores/sessionStore'
import { initialUiState, useUiStore } from '../../stores/uiStore'
import {
  makeCell,
  makeContour,
  makeGlyph,
  makeGlyphDetail,
  makeLayer,
  makeLayerOutline,
  makeMaster,
  makePoint,
} from '../../test/fontFixtures'
import {
  createMockTransport,
  makeSummary,
  noChange,
  type MockTransport,
} from '../../test/mockTransport'

import { buildScene } from './glyphScene'

const closed = makeContour([
  makePoint(0, 0),
  makePoint(200, 0, 'Line'),
  makePoint(200, 200, 'Line'),
  makePoint(0, 200, 'Line'),
])
const openA = makeContour(
  [makePoint(0, 0), makePoint(50, 0, 'Line'), makePoint(90, 0, 'Line')],
  false,
)
const openB = makeContour([makePoint(200, 0), makePoint(300, 0, 'Line')], false)

let reloads = 0

function publish(contours = [closed], components: never[] = []) {
  const layer = makeLayer({ contours })
  const glyph = makeGlyph({ layers: [layer] })
  useEditorStore.setState({
    ...initialEditorState(),
    activeGlyph: {
      doc: 1,
      gid: 42,
      glyph,
      scene: buildScene({
        detail: makeGlyphDetail(glyph),
        cell: makeCell(),
        master: makeMaster(),
        masterId: 'm1',
        unitsPerEm: 1000,
      }),
      cell: makeCell(),
      outline: makeLayerOutline(layer, { components }),
      master: 'm1',
      masterGuides: [],
      reload: () => {
        reloads += 1
      },
    },
  })
}

function transport(): MockTransport {
  const mock = createMockTransport({
    'doc.apply': () => noChange([42]),
    'glyphs.search': () => [makeCell({ gid: 9, name: 'a' })],
  })
  setTransport(mock)
  return mock
}

function applied(mock: MockTransport): ApplyParams | undefined {
  return mock.calls.find((entry) => entry.method === 'doc.apply')?.params as
    ApplyParams | undefined
}

/** Lets the promise chain `commitField` starts finish. */
async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0))
  await new Promise((resolve) => setTimeout(resolve, 0))
}

beforeEach(() => {
  localStorage.clear()
  reloads = 0
  resetCommands()
  useUiStore.setState(initialUiState())
  useSessionStore.setState({ ...initialSessionState(), docs: [makeSummary()] })
  useDocumentStore.setState(initialDocumentState())
  useEditorStore.setState(initialEditorState())
  registerBuiltinCommands()
  useUiStore.getState().openTab({ kind: 'glyph', doc: 1, gid: 42 })
})

afterEach(() => {
  resetCommands()
  setCommandErrorReporter(() => undefined)
  setTransport(undefined)
})

describe('the Path menu', () => {
  it('is disabled until an Edit tab has a layer', () => {
    expect(isCommandEnabled('path.removeOverlap')).toBe(false)
    publish()
    expect(isCommandEnabled('path.removeOverlap')).toBe(true)
  })

  it.each([
    ['path.removeOverlap', 'RemoveOverlap', 'Remove overlap'],
    ['path.addExtrema', 'AddExtrema', 'Add extremes'],
    ['path.forceExtrema', 'ForceExtrema', 'Force extremes'],
    ['path.addInflections', 'AddInflections', 'Add inflections'],
  ])('sends %s as one PathOp', async (id, op, label) => {
    const mock = transport()
    publish()

    expect(runCommand(id)).toBe(true)
    await settle()

    expect(applied(mock)).toEqual({
      doc: 1,
      label,
      command: { PathOp: { glyph: 'a', layer: 'layer-1', op } },
    })
    expect(reloads).toBe(1)
  })

  it('rounds to the grid the preferences hold', async () => {
    const mock = transport()
    publish()

    runCommand('path.roundToGrid')
    await settle()

    expect(applied(mock)?.command).toEqual({
      PathOp: { glyph: 'a', layer: 'layer-1', op: { RoundToGrid: 1 } },
    })
  })

  it('corrects direction and normalizes starts/order in one undo entry', async () => {
    const mock = transport()
    publish()
    runCommand('path.correctDirection')
    await settle()
    expect(applied(mock)?.command).toEqual({
      Batch: [
        { PathOp: { glyph: 'a', layer: 'layer-1', op: 'CorrectDirection' } },
        { PathOp: { glyph: 'a', layer: 'layer-1', op: 'CanonicalOrder' } },
      ],
    })
  })

  it.each([
    ['path.correctDirectionAllMasters', ['CorrectDirection', 'CanonicalOrder']],
    ['path.tidyAllMasters', ['CanonicalStart', 'CanonicalOrder']],
  ])('batches %s over master layers only', async (id, ops) => {
    const mock = transport()
    publish()
    const active = useEditorStore.getState().activeGlyph!
    useEditorStore.getState().setActiveGlyph({
      ...active,
      glyph: {
        ...active.glyph,
        layers: [
          ...active.glyph.layers,
          makeLayer({ id: 'bold', kind: { Master: 'm2' } }),
          makeLayer({ id: 'background', kind: { Background: 'm1' } }),
          makeLayer({ id: 'sketch', kind: { Custom: 'sketch' } }),
        ],
      },
    })
    runCommand(id as string)
    await settle()
    expect(
      mock.calls.filter((call) => call.method === 'doc.apply'),
    ).toHaveLength(1)
    expect(applied(mock)?.command).toEqual({
      Batch: ['layer-1', 'bold'].flatMap((layer) =>
        (ops as string[]).map((op) => ({ PathOp: { glyph: 'a', layer, op } })),
      ),
    })
  })

  it('makes Tidy one undo entry over both canonical operations', async () => {
    const mock = transport()
    publish()

    runCommand('path.tidy')
    await settle()

    expect(applied(mock)?.command).toEqual({
      Batch: [
        { PathOp: { glyph: 'a', layer: 'layer-1', op: 'CanonicalStart' } },
        { PathOp: { glyph: 'a', layer: 'layer-1', op: 'CanonicalOrder' } },
      ],
    })
    expect(applied(mock)?.label).toBe('Tidy paths')
  })

  it('converts curves in both directions', async () => {
    const mock = transport()
    publish()

    runCommand('path.convertQuadratic')
    await settle()

    expect(applied(mock)?.command).toMatchObject({
      PathOp: { op: { ConvertCurves: { Quadratic: { max_err: 0.5 } } } },
    })
  })

  it('opens the two option dialogs rather than guessing', () => {
    publish()

    runCommand('path.simplify')
    expect(useUiStore.getState().dialog).toEqual({ kind: 'simplify' })

    runCommand('path.expandStroke')
    expect(useUiStore.getState().dialog).toEqual({ kind: 'expandStroke' })
  })

  it('reverses every contour when nothing is selected', async () => {
    const mock = transport()
    publish([closed, openA])

    runCommand('path.reverse')
    await settle()

    expect(applied(mock)?.command).toEqual({
      ReverseContours: { glyph: 'a', layer: 'layer-1', contours: [0, 1] },
    })
  })

  it('reverses only the contours the selection touches', async () => {
    const mock = transport()
    publish([closed, openA])
    useEditorStore
      .getState()
      .setSelection(selectionOf([{ kind: 'node', contour: 1, point: 0 }]))

    runCommand('path.reverse')
    await settle()

    expect(applied(mock)?.command).toMatchObject({
      ReverseContours: { contours: [1] },
    })
  })

  it('joins the two selected open ends, ordering them for the kernel', async () => {
    const mock = transport()
    publish([openA, openB])
    useEditorStore.getState().setSelection(
      selectionOf([
        { kind: 'node', contour: 1, point: 0 },
        { kind: 'node', contour: 0, point: 2 },
      ]),
    )

    expect(isCommandEnabled('path.join')).toBe(true)
    runCommand('path.join')
    await settle()

    expect(applied(mock)?.command).toEqual({
      JoinContours: {
        glyph: 'a',
        layer: 'layer-1',
        a: { contour: 0, point: 2 },
        b: { contour: 1, point: 0 },
      },
    })
  })

  it('sets the start point at the one selected node', async () => {
    const mock = transport()
    publish()
    useEditorStore
      .getState()
      .setSelection(selectionOf([{ kind: 'node', contour: 0, point: 2 }]))

    expect(isCommandEnabled('path.setStartPoint')).toBe(true)
    runCommand('path.setStartPoint')
    await settle()

    expect(applied(mock)?.command).toEqual({
      SetStartPoint: {
        glyph: 'a',
        layer: 'layer-1',
        point: { contour: 0, point: 2 },
      },
    })
  })

  it('breaks a closed contour at the selected node', async () => {
    const mock = transport()
    publish()
    useEditorStore
      .getState()
      .setSelection(selectionOf([{ kind: 'node', contour: 0, point: 1 }]))

    expect(isCommandEnabled('path.break')).toBe(true)
    runCommand('path.break')
    await settle()

    expect(applied(mock)?.command).toEqual({
      SplitContour: {
        glyph: 'a',
        layer: 'layer-1',
        point: { contour: 0, point: 1 },
      },
    })
  })

  it('gates the selection-scoped items on a usable selection', () => {
    publish([openA, openB])

    expect(isCommandEnabled('path.setStartPoint')).toBe(false)
    expect(isCommandEnabled('path.join')).toBe(false)
    expect(isCommandEnabled('path.break')).toBe(false)
  })

  it('binds the design spec shortcuts', () => {
    publish([openA, openB])
    useEditorStore.getState().setSelection(
      selectionOf([
        { kind: 'node', contour: 0, point: 2 },
        { kind: 'node', contour: 1, point: 0 },
      ]),
    )
    const press = (key: string, shift = true) =>
      commandForEvent(
        new KeyboardEvent('keydown', { key, ctrlKey: true, shiftKey: shift }),
        false,
      )

    expect(press('O')).toBe('path.removeOverlap')
    expect(press('R')).toBe('path.correctDirection')
    expect(press('E')).toBe('path.addExtrema')
    expect(press('M')).toBe('path.simplify')
    expect(press('T')).toBe('path.tidy')
    expect(press('j', false)).toBe('path.join')
  })
})

describe('the Layer and Glyph menus', () => {
  it('adds a background from the Layer menu', async () => {
    const mock = transport()
    publish()

    runCommand('layer.addBackground')
    await settle()

    expect(applied(mock)?.command).toMatchObject({
      AddLayer: { glyph: 'a', layer: { kind: { Background: 'm1' } } },
    })
  })

  it('offers Swap with Background only once one exists', () => {
    publish()
    expect(isCommandEnabled('layer.swapWithBackground')).toBe(false)
    expect(isCommandEnabled('layer.addBackground')).toBe(true)

    const layers = [
      makeLayer({ contours: [closed] }),
      makeLayer({ id: 'bg-1', kind: { Background: 'm1' }, name: 'Background' }),
    ]
    const glyph = makeGlyph({ layers })
    const active = useEditorStore.getState().activeGlyph
    if (!active) throw new Error('no glyph published')
    useEditorStore.getState().setActiveGlyph({ ...active, glyph })

    expect(isCommandEnabled('layer.swapWithBackground')).toBe(true)
    expect(isCommandEnabled('layer.addBackground')).toBe(false)
  })

  it('decomposes every component of the layer', async () => {
    const mock = transport()
    publish([closed], [])
    const active = useEditorStore.getState().activeGlyph
    if (!active?.outline) throw new Error('no layer published')
    useEditorStore.getState().setActiveGlyph({
      ...active,
      outline: {
        ...active.outline,
        components: [
          {
            component: {
              base: 'acute',
              transform: [1, 0, 0, 1, 0, 0],
              anchor: null,
              auto_align: false,
              use_my_metrics: false,
              round_to_grid: false,
              smart: null,
              point_match: null,
            },
            path: 'M0 0Z',
            bbox: [0, 0, 10, 10],
          },
        ],
      },
    })

    expect(isCommandEnabled('glyph.decomposeAll')).toBe(true)
    runCommand('glyph.decomposeAll')
    await settle()

    expect(applied(mock)?.command).toEqual({
      DecomposeComponents: { glyph: 'a', layer: 'layer-1', indices: [0] },
    })
  })

  it('duplicates the open glyph under a free name', async () => {
    const mock = transport()
    publish()

    runCommand('glyph.duplicate')
    await settle()
    await settle()

    expect(applied(mock)).toEqual({
      doc: 1,
      label: 'Duplicate glyph',
      command: { DuplicateGlyph: { source: 'a', name: 'a.copy' } },
    })
  })
})
